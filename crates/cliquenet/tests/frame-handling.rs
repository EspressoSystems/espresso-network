use std::{collections::HashMap, net::Ipv4Addr};

use bytes::Bytes;
use cliquenet::{Config, NetAddr, Network, Slot, x25519::Keypair};
use rand::Rng;

/// Send and receive 5 MiB messages.
#[tokio::test]
async fn multiple_frames() {
    const PARTIES: u16 = 30;

    let parties = (0..PARTIES)
        .map(|i| {
            (
                Keypair::generate().unwrap(),
                NetAddr::from((Ipv4Addr::LOCALHOST, 50000 + i)),
            )
        })
        .collect::<Vec<_>>();

    let mut networks = HashMap::new();
    for (i, (k, a)) in parties.clone().into_iter().enumerate() {
        networks.insert(
            i,
            Network::create(
                Config::builder()
                    .name("frames")
                    .keypair(k)
                    .bind(a)
                    .parties(parties.iter().map(|(k, a)| (k.public_key(), a.clone())))
                    .build(),
            )
            .await
            .unwrap(),
        );
    }

    let mut counters: HashMap<usize, HashMap<Bytes, usize>> = HashMap::new();

    for b in 0..10 {
        for net in networks.values_mut() {
            net.broadcast(Slot::new(b), gen_message()).unwrap();
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
            net.gc(Slot::new(b)).unwrap()
        }
    }
}

fn gen_message() -> Vec<u8> {
    let mut g = rand::rng();
    let mut v = vec![0; 5 * 1024 * 1024];
    g.fill_bytes(&mut v);
    v
}
