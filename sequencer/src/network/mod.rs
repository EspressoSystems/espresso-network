use either::Either;
use espresso_types::PubKey;

use super::*;

pub mod cdn;
pub mod libp2p;

pub type Production = Either<
    CombinedNetworks<SeqTypes>,
    CompatNetwork<CombinedNetworks<SeqTypes>, <SeqTypes as NodeType>::SignatureKey>,
>;

pub type Memory = MemoryNetwork<PubKey>;
