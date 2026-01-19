use std::{collections::HashMap, net::Ipv4Addr};

use bytes::{Bytes, BytesMut};
use cliquenet::{Address, Keypair, NetConf, Network, Retry, retry::Data};
use rand::RngCore;

/// Send and receive messages of various sizes between 1 byte and 5 MiB.
#[tokio::test]
async fn multiple_frames() {
    let _ = tracing_subscriber::fmt::try_init();

    const PARTIES: u16 = 30;

    let parties = (0..PARTIES)
        .map(|i| {
            (
                i,
                Keypair::generate().unwrap(),
                Address::from((Ipv4Addr::LOCALHOST, 50000 + i)),
            )
        })
        .collect::<Vec<_>>();

    let mut networks = HashMap::new();
    for (k, x, a) in parties.clone() {
        networks.insert(
            k,
            Retry::new(
                Network::create(
                    NetConf::builder()
                        .name("frames")
                        .keypair(x)
                        .label(k)
                        .bind(a)
                        .parties(
                            parties
                                .iter()
                                .map(|(i, x, a)| (*i, x.public_key(), a.clone())),
                        )
                        .build(),
                )
                .await
                .unwrap(),
            ),
        );
    }

    let mut counters: HashMap<u16, HashMap<Bytes, usize>> = HashMap::new();

    for b in 0..10 {
        for net in networks.values_mut() {
            net.broadcast(b, gen_message()).await.unwrap();
        }
        loop {
            for (k, net) in &mut networks {
                if counters.get(k).map(|m| m.len()).unwrap_or(0) == usize::from(PARTIES) {
                    continue;
                }
                let (_, data) = net.receive().await.unwrap();
                *counters.entry(*k).or_default().entry(data).or_default() += 1
            }
            if counters.values().all(|m| m.len() == usize::from(PARTIES)) {
                break;
            }
        }
        for net in networks.values_mut() {
            net.gc(b)
        }
    }
}

fn gen_message() -> Data {
    let mut g = rand::rng();
    let mut v = vec![0; 5 * 1024 * 1024];
    g.fill_bytes(&mut v);
    Data::try_from(BytesMut::from(&v[..])).unwrap()
}
