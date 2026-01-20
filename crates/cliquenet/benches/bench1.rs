use std::{collections::HashMap, io, sync::LazyLock, time::Duration};

use cliquenet::{
    Address, Keypair, MAX_MESSAGE_SIZE, NetConf, Network, PublicKey, Retry,
    retry::{Data, NetworkDown},
};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::RngCore;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    runtime::Runtime, time::sleep,
};

const A: u8 = 0;
const B: u8 = 1;
const SIZES: &[usize] = &[128 * 1024, 512 * 1024, 1024 * 1024, MAX_MESSAGE_SIZE];

static DATA: LazyLock<HashMap<usize, Data>> = LazyLock::new(|| {
    let mut g = rand::rng();
    HashMap::from_iter(SIZES.iter().map(|n| {
        let mut v = vec![0; *n];
        g.fill_bytes(&mut v);
        let d = Data::try_from(v).unwrap();
        (*n, d)
    }))
});

async fn setup_tcp() -> (TcpStream, TcpStream) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = l.local_addr().unwrap();
    let (a, b) = tokio::join!(l.accept(), TcpStream::connect(a));
    let a = a.unwrap().0;
    let b = b.unwrap();
    a.set_nodelay(true).unwrap();
    b.set_nodelay(true).unwrap();
    (a, b)
}

async fn setup_cliquenet() -> (Retry<u8>, Retry<u8>) {
    let a = Keypair::generate().unwrap();
    let b = Keypair::generate().unwrap();

    let all: [(u8, PublicKey, Address); 2] = [
        (
            A,
            a.public_key(),
            Address::try_from("127.0.0.1:50000").unwrap(),
        ),
        (
            B,
            b.public_key(),
            Address::try_from("127.0.0.1:51000").unwrap(),
        ),
    ];

    let net_a = Retry::new(
        Network::create(
            NetConf::builder()
                .name("bench")
                .label(A)
                .keypair(a)
                .bind(all[0].2.clone())
                .parties(all.clone())
                .build(),
        )
        .await
        .unwrap(),
    );
    let net_b = Retry::new(
        Network::create(
            NetConf::builder()
                .name("bench")
                .label(B)
                .keypair(b)
                .bind(all[1].2.clone())
                .parties(all.clone())
                .build(),
        )
        .await
        .unwrap(),
    );

    (net_a, net_b)
}

async fn tcp(size: usize, srv: &mut TcpStream, clt: &mut TcpStream) {
    async fn echo_server(stream: &mut TcpStream) -> io::Result<()> {
        let len = stream.read_u32().await?;
        let mut v = vec![0; len as usize];
        stream.read_exact(&mut v).await?;
        stream.write_u32(len).await?;
        stream.write_all(&v).await
    }

    async fn echo_client(stream: &mut TcpStream, d: Data) -> io::Result<()> {
        stream.write_u32(d.len() as u32).await?;
        stream.write_all(&d).await?;
        let len = stream.read_u32().await?;
        assert_eq!(len as usize, d.len());
        let mut v = vec![0; len as usize];
        stream.read_exact(&mut v).await?;
        assert_eq!(&*v, &*d);
        Ok(())
    }

    let dat = DATA[&size].clone();
    let (ra, rb) = tokio::join!(echo_server(srv), echo_client(clt, dat));
    ra.unwrap();
    rb.unwrap();
}

async fn cliquenet(to: u8, size: usize, srv: &mut Retry<u8>, clt: &mut Retry<u8>) {
    async fn echo_server(net: &mut Retry<u8>) -> Result<(), NetworkDown> {
        let (src, data) = net.receive().await?;
        let _ = net.unicast(src, 0, data.try_into().unwrap()).await?;
        Ok(())
    }

    async fn echo_client(to: u8, net: &mut Retry<u8>, d: Data) -> Result<(), NetworkDown> {
        let _ = net.unicast(to, 0, d.clone()).await?;
        let (src, data) = net.receive().await?;
        assert_eq!(src, to);
        assert_eq!(&*data, &*d);
        Ok(())
    }

    let dat = DATA[&size].clone();
    let fa = echo_server(srv);
    let fb = echo_client(to, clt, dat);
    let (ra, rb) = tokio::join!(fa, fb);
    ra.unwrap();
    rb.unwrap();
}

fn bench_tcp(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (mut srv, mut clt) = rt.block_on(setup_tcp());
    let mut group = c.benchmark_group("tcp");
    for n in SIZES {
        group
            .throughput(Throughput::Bytes(*n as u64))
            .bench_with_input(
                BenchmarkId::from_parameter(format!("{}k", n / 1024)),
                n,
                |b, n| b.iter(|| rt.block_on(tcp(*n, &mut srv, &mut clt))),
            );
    }
    group.finish();
}

fn bench_cliquenet(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (mut srv, mut clt) = rt.block_on(async {
        let (a, b) = setup_cliquenet().await;
        sleep(Duration::from_secs(3)).await;
        (a, b)
    });
    let mut group = c.benchmark_group("cliquenet");
    for n in SIZES {
        group
            .throughput(Throughput::Bytes(*n as u64))
            .bench_with_input(
                BenchmarkId::from_parameter(format!("{}k", n / 1024)),
                n,
                |b, n| b.iter(|| rt.block_on(cliquenet(A, *n, &mut srv, &mut clt))),
            );
    }
    group.finish();
}

criterion_group!(benches, bench_tcp, bench_cliquenet);
criterion_main!(benches);
