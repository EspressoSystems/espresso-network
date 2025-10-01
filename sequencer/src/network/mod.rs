use espresso_types::PubKey;

use super::*;

pub mod cdn;
pub mod libp2p;

pub type Production = Libp2pNetwork<SeqTypes>;

pub type Memory = MemoryNetwork<PubKey>;
