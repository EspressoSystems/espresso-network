use std::{
    collections::HashMap, io, net::Ipv4Addr, num::NonZeroUsize, sync::LazyLock, time::Duration,
};

use cliquenet::{
    Config, Network, RetryPolicy, SendAction, SendCommand, Slot,
    noise::Protocol,
    x25519::{Keypair, PublicKey},
};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::Rng;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    runtime::Runtime,
    task::JoinSet,
    time::sleep,
};

const KIBI: usize = 1024;
const MEBI: usize = KIBI * KIBI;
const GIBI: usize = MEBI * KIBI;

const SIZES: &[usize] = &[
    1,
    128 * KIBI,
    //512 * KIBI,
    //MEBI,
    //5 * MEBI,
    //10 * MEBI,
    //50 * MEBI,
    //100 * MEBI,
];

static DATA: LazyLock<HashMap<usize, Vec<u8>>> = LazyLock::new(|| {
    let mut g = rand::rng();
    HashMap::from_iter(SIZES.iter().map(|n| {
        let mut v = vec![0; *n];
        g.fill_bytes(&mut v);
        (*n, v)
    }))
});

fn reserve_port() -> u16 {
    let s = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let a = s.local_addr().unwrap();
    let _ = std::net::TcpStream::connect(a).unwrap();
    let _ = s.accept().unwrap();
    a.port()
}

// -- TCP baseline ------------------------------------------------------------

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

async fn tcp_echo(srv: &mut TcpStream, clt: &mut TcpStream, data: &[u8]) {
    async fn echo_server(stream: &mut TcpStream) -> io::Result<()> {
        let len = stream.read_u32().await?;
        let mut v = vec![0; len as usize];
        stream.read_exact(&mut v).await?;
        stream.write_u32(len).await?;
        stream.write_all(&v).await
    }

    async fn echo_client(stream: &mut TcpStream, d: &[u8]) -> io::Result<()> {
        stream.write_u32(d.len() as u32).await?;
        stream.write_all(d).await?;
        let len = stream.read_u32().await?;
        let mut v = vec![0; len as usize];
        stream.read_exact(&mut v).await?;
        assert_eq!(v, d);
        Ok(())
    }

    let (ra, rb) = tokio::join!(echo_server(srv), echo_client(clt, data));
    ra.unwrap();
    rb.unwrap();
}

fn bench_tcp(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (mut srv, mut clt) = rt.block_on(setup_tcp());
    let mut group = c.benchmark_group("tcp echo");
    for &n in SIZES {
        group.throughput(Throughput::Bytes(2 * n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(show(n)), &n, |b, &n| {
            let data = &DATA[&n];
            b.iter(|| rt.block_on(tcp_echo(&mut srv, &mut clt, data)))
        });
    }
    group.finish();
}

// -- Network echo ------------------------------------------------------------

struct Echo {
    net_a: Network,
    pkb: PublicKey,
    _echo_handle: tokio::task::AbortHandle,
}

async fn setup_echo() -> Echo {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pka = ka.public_key();
    let pkb = kb.public_key();

    let port_a = reserve_port();
    let port_b = reserve_port();

    let addr_a = (Ipv4Addr::LOCALHOST, port_a);
    let addr_b = (Ipv4Addr::LOCALHOST, port_b);

    let conf_a = Config::builder()
        .name("bench")
        .keypair(ka)
        .bind(addr_a.into())
        .parties([(pkb, addr_b.into())])
        .max_message_size(NonZeroUsize::new(100 * MEBI).unwrap())
        .receive_timeout(Duration::from_secs(60))
        .retry_delays(vec![1, 3])
        .max_retry_delay(Duration::from_secs(5))
        .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
        .build();

    let conf_b = Config::builder()
        .name("bench")
        .keypair(kb)
        .bind(addr_b.into())
        .parties([(pka, addr_a.into())])
        .max_message_size(NonZeroUsize::new(100 * MEBI).unwrap())
        .receive_timeout(Duration::from_secs(60))
        .retry_delays(vec![1, 3])
        .max_retry_delay(Duration::from_secs(5))
        .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
        .build();

    let net_a = Network::create(conf_a).await.unwrap();
    let mut net_b = Network::create(conf_b).await.unwrap();

    sleep(Duration::from_secs(2)).await;

    let echo = tokio::spawn(async move {
        while let Some((src, data)) = net_b.receive().await {
            let _ = net_b.unicast(Slot::MIN, src, data.to_vec());
        }
    });

    Echo {
        net_a,
        pkb,
        _echo_handle: echo.abort_handle(),
    }
}

fn bench_cliquenet(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut echo = rt.block_on(setup_echo());

    let mut group = c.benchmark_group("cliquenet echo");
    for &n in SIZES {
        group.throughput(Throughput::Bytes(2 * n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(show(n)), &n, |b, &n| {
            let data = &DATA[&n];
            b.iter(|| {
                echo.net_a
                    .unicast(Slot::MIN, echo.pkb, data.clone())
                    .unwrap();
                let (src, recv) = rt.block_on(async { echo.net_a.receive().await.unwrap() });
                assert_eq!(src, echo.pkb);
                assert_eq!(recv.len(), n);
            });
        });
    }
    group.finish();

    let mut group = c.benchmark_group("cliquenet echo (no-retry)");
    for &n in SIZES {
        group.throughput(Throughput::Bytes(2 * n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(show(n)), &n, |b, &n| {
            let data = &DATA[&n];
            b.iter(|| {
                let cmd = SendCommand::builder()
                    .slot(Slot::MIN)
                    .retry(RetryPolicy::NoRetry)
                    .action(SendAction::Unicast(echo.pkb, data.clone()))
                    .build();
                echo.net_a.send(cmd).unwrap();
                let (src, recv) = rt.block_on(async { echo.net_a.receive().await.unwrap() });
                assert_eq!(src, echo.pkb);
                assert_eq!(recv.len(), n);
            });
        });
    }
    group.finish();
}

// -- Bidirectional throughput -------------------------------------------------

struct BiDir {
    ctrl_a: cliquenet::NetworkController,
    recv_a: cliquenet::NetworkReceiver,
    pka: PublicKey,
    ctrl_b: cliquenet::NetworkController,
    recv_b: cliquenet::NetworkReceiver,
    pkb: PublicKey,
}

async fn setup_bidir() -> BiDir {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pka = ka.public_key();
    let pkb = kb.public_key();

    let port_a = reserve_port();
    let port_b = reserve_port();

    let addr_a = (Ipv4Addr::LOCALHOST, port_a);
    let addr_b = (Ipv4Addr::LOCALHOST, port_b);

    let conf_a = Config::builder()
        .name("bench")
        .keypair(ka)
        .bind(addr_a.into())
        .parties([(pkb, addr_b.into())])
        .max_message_size(NonZeroUsize::new(100 * MEBI).unwrap())
        .receive_timeout(Duration::from_secs(60))
        .retry_delays(vec![1, 3])
        .max_retry_delay(Duration::from_secs(5))
        .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
        .build();

    let conf_b = Config::builder()
        .name("bench")
        .keypair(kb)
        .bind(addr_b.into())
        .parties([(pka, addr_a.into())])
        .max_message_size(NonZeroUsize::new(100 * MEBI).unwrap())
        .receive_timeout(Duration::from_secs(60))
        .retry_delays(vec![1, 3])
        .max_retry_delay(Duration::from_secs(5))
        .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
        .build();

    let net_a = Network::create(conf_a).await.unwrap();
    let net_b = Network::create(conf_b).await.unwrap();

    sleep(Duration::from_secs(2)).await;

    let (ctrl_a, recv_a) = net_a.split_into();
    let (ctrl_b, recv_b) = net_b.split_into();

    BiDir {
        ctrl_a,
        recv_a,
        pka,
        ctrl_b,
        recv_b,
        pkb,
    }
}

fn bench_bidirectional(c: &mut Criterion) {
    const ROUNDS: usize = 10;

    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("bidirectional echos (10 rounds)");
    for &n in SIZES {
        if n > 10 * MEBI {
            continue;
        }
        let mut bd = rt.block_on(setup_bidir());
        group.throughput(Throughput::Bytes((2 * n * ROUNDS) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(show(n)), &n, |b, &n| {
            let data = &DATA[&n];
            b.iter(|| {
                rt.block_on(async {
                    let BiDir {
                        ctrl_a,
                        recv_a,
                        pka,
                        ctrl_b,
                        recv_b,
                        pkb,
                    } = &mut bd;
                    let pka = *pka;
                    let pkb = *pkb;
                    let s_a = async {
                        for _ in 0..ROUNDS {
                            ctrl_a.unicast(Slot::MIN, pkb, data.clone()).unwrap();
                        }
                    };
                    let r_a = async {
                        for _ in 0..ROUNDS {
                            let (src, msg) = recv_a.receive().await.unwrap();
                            assert_eq!(src, pkb);
                            assert_eq!(msg.len(), n);
                        }
                    };
                    let s_b = async {
                        for _ in 0..ROUNDS {
                            ctrl_b.unicast(Slot::MIN, pka, data.clone()).unwrap();
                        }
                    };
                    let r_b = async {
                        for _ in 0..ROUNDS {
                            let (src, msg) = recv_b.receive().await.unwrap();
                            assert_eq!(src, pka);
                            assert_eq!(msg.len(), n);
                        }
                    };
                    tokio::join!(s_a, r_a, s_b, r_b);
                });
            });
        });
    }
    group.finish();

    let mut group = c.benchmark_group("bidirectional echos (10 rounds, no-retry)");
    for &n in SIZES {
        if n > 10 * MEBI {
            continue;
        }
        let mut bd = rt.block_on(setup_bidir());
        group.throughput(Throughput::Bytes((2 * n * ROUNDS) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(show(n)), &n, |b, &n| {
            let data = &DATA[&n];
            b.iter(|| {
                rt.block_on(async {
                    let BiDir {
                        ctrl_a,
                        recv_a,
                        pka,
                        ctrl_b,
                        recv_b,
                        pkb,
                    } = &mut bd;
                    let pka = *pka;
                    let pkb = *pkb;
                    let s_a = async {
                        for _ in 0..ROUNDS {
                            let cmd = SendCommand::builder()
                                .slot(Slot::MIN)
                                .retry(RetryPolicy::NoRetry)
                                .action(SendAction::Unicast(pkb, data.clone()))
                                .build();
                            ctrl_a.send(cmd).unwrap();
                        }
                    };
                    let r_a = async {
                        for _ in 0..ROUNDS {
                            let (src, msg) = recv_a.receive().await.unwrap();
                            assert_eq!(src, pkb);
                            assert_eq!(msg.len(), n);
                        }
                    };
                    let s_b = async {
                        for _ in 0..ROUNDS {
                            let cmd = SendCommand::builder()
                                .slot(Slot::MIN)
                                .retry(RetryPolicy::NoRetry)
                                .action(SendAction::Unicast(pka, data.clone()))
                                .build();
                            ctrl_b.send(cmd).unwrap();
                        }
                    };
                    let r_b = async {
                        for _ in 0..ROUNDS {
                            let (src, msg) = recv_b.receive().await.unwrap();
                            assert_eq!(src, pka);
                            assert_eq!(msg.len(), n);
                        }
                    };
                    tokio::join!(s_a, r_a, s_b, r_b);
                });
            });
        });
    }
    group.finish();
}

// -- N-node mesh throughput ---------------------------------------------------

struct MeshNode {
    ctrl: cliquenet::NetworkController,
    recv: cliquenet::NetworkReceiver,
    peers: Vec<PublicKey>,
}

struct Mesh {
    nodes: Vec<MeshNode>,
}

async fn setup_mesh(n_nodes: usize) -> Mesh {
    let keypairs: Vec<Keypair> = (0..n_nodes).map(|_| Keypair::generate().unwrap()).collect();
    let pks: Vec<PublicKey> = keypairs.iter().map(|k| k.public_key()).collect();
    let ports: Vec<u16> = (0..n_nodes).map(|_| reserve_port()).collect();

    let mut nets = Vec::with_capacity(n_nodes);
    for (i, kp) in keypairs.into_iter().enumerate() {
        let parties: Vec<_> = (0..n_nodes)
            .filter(|&j| j != i)
            .map(|j| (pks[j], (Ipv4Addr::LOCALHOST, ports[j]).into()))
            .collect();
        let conf = Config::builder()
            .name("bench")
            .keypair(kp)
            .bind((Ipv4Addr::LOCALHOST, ports[i]).into())
            .parties(parties)
            .max_message_size(NonZeroUsize::new(100 * MEBI).unwrap())
            .receive_timeout(Duration::from_secs(60))
            .retry_delays(vec![1, 3])
            .max_retry_delay(Duration::from_secs(5))
            .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
            .build();
        nets.push(Network::create(conf).await.unwrap());
    }

    sleep(Duration::from_secs(3)).await;

    let nodes = nets
        .into_iter()
        .enumerate()
        .map(|(i, net)| {
            let (ctrl, recv) = net.split_into();
            let peers: Vec<PublicKey> = (0..n_nodes).filter(|&j| j != i).map(|j| pks[j]).collect();
            MeshNode { ctrl, recv, peers }
        })
        .collect();

    Mesh { nodes }
}

fn bench_mesh(c: &mut Criterion) {
    const ROUNDS: usize = 10;
    const NODES: usize = 10;

    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("mesh echos (10 nodes, 10 rounds)");
    for &n in SIZES {
        // Per iter we move NODES * (NODES - 1) * ROUNDS * n bytes total.
        // Cap message size to keep iter time bounded.
        if n > MEBI {
            continue;
        }
        let mut mesh = rt.block_on(setup_mesh(NODES));
        let total = (NODES * (NODES - 1) * ROUNDS * n) as u64;
        group.throughput(Throughput::Bytes(total));
        group.bench_with_input(BenchmarkId::from_parameter(show(n)), &n, |b, &n| {
            let data = &DATA[&n];
            b.iter(|| {
                rt.block_on(async {
                    let mut set: JoinSet<MeshNode> = JoinSet::new();
                    for node in mesh.nodes.drain(..) {
                        let task_data = data.clone();
                        set.spawn(async move {
                            let MeshNode {
                                mut ctrl,
                                mut recv,
                                peers,
                            } = node;
                            let n_recv = ROUNDS * peers.len();
                            {
                                let send = async {
                                    for _ in 0..ROUNDS {
                                        for &peer in &peers {
                                            ctrl.unicast(Slot::MIN, peer, task_data.clone())
                                                .unwrap();
                                        }
                                    }
                                };
                                let receive = async {
                                    for _ in 0..n_recv {
                                        let (src, msg) = recv.receive().await.unwrap();
                                        debug_assert!(peers.contains(&src));
                                        debug_assert_eq!(msg.len(), n);
                                    }
                                };
                                tokio::join!(send, receive);
                            }
                            MeshNode { ctrl, recv, peers }
                        });
                    }
                    while let Some(res) = set.join_next().await {
                        mesh.nodes.push(res.unwrap());
                    }
                });
            });
        });
    }
    group.finish();

    let mut group = c.benchmark_group("mesh echos (10 nodes, 10 rounds, no-retry)");
    for &n in SIZES {
        if n > MEBI {
            continue;
        }
        let mut mesh = rt.block_on(setup_mesh(NODES));
        let total = (NODES * (NODES - 1) * ROUNDS * n) as u64;
        group.throughput(Throughput::Bytes(total));
        group.bench_with_input(BenchmarkId::from_parameter(show(n)), &n, |b, &n| {
            let data = &DATA[&n];
            b.iter(|| {
                rt.block_on(async {
                    let mut set: JoinSet<MeshNode> = JoinSet::new();
                    for node in mesh.nodes.drain(..) {
                        let task_data = data.clone();
                        set.spawn(async move {
                            let MeshNode {
                                mut ctrl,
                                mut recv,
                                peers,
                            } = node;
                            let n_recv = ROUNDS * peers.len();
                            {
                                let send = async {
                                    for _ in 0..ROUNDS {
                                        for &peer in &peers {
                                            let cmd = SendCommand::builder()
                                                .slot(Slot::MIN)
                                                .retry(RetryPolicy::NoRetry)
                                                .action(SendAction::Unicast(
                                                    peer,
                                                    task_data.clone(),
                                                ))
                                                .build();
                                            ctrl.send(cmd).unwrap();
                                        }
                                    }
                                };
                                let receive = async {
                                    for _ in 0..n_recv {
                                        let (src, msg) = recv.receive().await.unwrap();
                                        debug_assert!(peers.contains(&src));
                                        debug_assert_eq!(msg.len(), n);
                                    }
                                };
                                tokio::join!(send, receive);
                            }
                            MeshNode { ctrl, recv, peers }
                        });
                    }
                    while let Some(res) = set.join_next().await {
                        mesh.nodes.push(res.unwrap());
                    }
                });
            });
        });
    }
    group.finish();
}

fn show(size: usize) -> String {
    match size {
        1 => "1 byte".to_string(),
        n if n < KIBI => format!("{n} bytes"),
        n if n < MEBI => format!("{} KiB", n / KIBI),
        n if n < GIBI => format!("{} MiB", n / MEBI),
        n => format!("{n} bytes"),
    }
}

criterion_group!(
    benches,
    bench_tcp,
    bench_cliquenet,
    bench_bidirectional,
    bench_mesh
);
criterion_main!(benches);
