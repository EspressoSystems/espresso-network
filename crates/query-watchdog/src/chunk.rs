use std::ops::Range;

use serde::Deserialize;
use strum::{EnumString, IntoStaticStr, VariantArray};

/// Heights per request for every kind: the server's block range limit (`availability.rs:76`);
/// larger values fail every block probe.
pub const CHUNK_LEN: u64 = 100;

/// Next chunk of `[next_height, until_height)`, at most `CHUNK_LEN` long; `None` when empty.
pub fn next_chunk(next_height: u64, until_height: u64) -> Option<Range<u64>> {
    (next_height < until_height)
        .then(|| next_height..until_height.min(next_height.saturating_add(CHUNK_LEN)))
}

/// Object an archive query node must serve; stored as its snake_case label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, VariantArray, IntoStaticStr, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum DataKind {
    // per height
    Leaf,
    Block,
    VidCommon,
    LcLeafProof,
    Aggregate,
    // per epoch
    StakeTable,
    StateCert,
    LcStakeTable,
}

impl DataKind {
    /// True for `Leaf`, `Block`, `VidCommon`: their ranges count heights and a failed range probe
    /// can be refined to the exact missing heights.
    pub fn is_range(self) -> bool {
        matches!(self, Self::Leaf | Self::Block | Self::VidCommon)
    }

    /// False for `Aggregate`: the node's aggregator legitimately lags the tip, so its missing
    /// ranges are counted but never alerted.
    pub fn can_alert(self) -> bool {
        self != Self::Aggregate
    }

    /// Epochs between an object's epoch and the epoch whose root block the node builds it from:
    /// 2 for the stake tables, 0 for the state cert.
    pub fn epoch_root_lag(self) -> u64 {
        match self {
            Self::StakeTable | Self::LcStakeTable => 2,
            _ => 0,
        }
    }

    /// True for `StakeTable`, `StateCert`, `LcStakeTable`; their ranges count epochs.
    pub fn is_per_epoch(self) -> bool {
        matches!(
            self,
            Self::StakeTable | Self::StateCert | Self::LcStakeTable
        )
    }
}

/// One request for a kind over a range of its units; also the key of a `missing_chunk` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probe {
    pub kind: DataKind,
    pub range: Range<u64>,
}

impl Probe {
    /// Route of the request:
    /// `v1/availability/{leaf,block,vid/common}/{start}/{end}`, `v1/light-client/leaf/{end - 1}`,
    /// `v1/node/transactions/count/{end - 1}`, `v1/node/stake-table/{start}`,
    /// `v1/catchup/{start}/state-cert`, `v1/light-client/stake-table/{start}`.
    pub fn route(&self) -> String {
        let Range { start, end } = self.range;
        match self.kind {
            DataKind::Leaf => format!("v1/availability/leaf/{start}/{end}"),
            DataKind::Block => format!("v1/availability/block/{start}/{end}"),
            DataKind::VidCommon => format!("v1/availability/vid/common/{start}/{end}"),
            DataKind::LcLeafProof => format!("v1/light-client/leaf/{}", end - 1),
            DataKind::Aggregate => format!("v1/node/transactions/count/{}", end - 1),
            DataKind::StakeTable => format!("v1/node/stake-table/{start}"),
            DataKind::StateCert => format!("v1/catchup/{start}/state-cert"),
            DataKind::LcStakeTable => format!("v1/light-client/stake-table/{start}"),
        }
    }
}

/// Epoch parameters of the network, from `v1/config/hotshot`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct EpochConfig {
    pub epoch_height: u64,
    pub epoch_start_block: u64,
}

impl EpochConfig {
    /// Epoch of a block: `ceil(block / epoch_height)`, with block 0 in epoch 1.
    fn epoch(&self, block: u64) -> u64 {
        block.div_ceil(self.epoch_height).max(1)
    }
}

/// The block `kind` of `epoch` depends on: the epoch `epoch_root_lag` back (epoch 0 for earlier
/// ones) times the epoch height. Missing data of the epoch is probed, and judged against the tip,
/// by the chunk holding this block. `None` only when the product overflows.
pub fn epoch_block(kind: DataKind, epoch: u64, epoch_height: u64) -> Option<u64> {
    epoch
        .saturating_sub(kind.epoch_root_lag())
        .checked_mul(epoch_height)
}

/// Every probe for `chunk`: each range kind over `chunk`, each single-height kind over its last
/// height, and each per-epoch kind for every epoch `e` with `epoch_block(kind, e, epoch_height)` in
/// `chunk`, `e >= epoch(epoch_start_block)` (`+ 2` for `LcStakeTable`), as `[e, e + 1)`. `None`
/// epochs: per-height kinds only.
pub fn probes(chunk: &Range<u64>, epochs: Option<EpochConfig>) -> Vec<Probe> {
    let per_height = DataKind::VARIANTS
        .iter()
        .filter(|kind| !kind.is_per_epoch())
        .map(|&kind| Probe {
            kind,
            range: if kind.is_range() {
                chunk.clone()
            } else {
                chunk.end.saturating_sub(1)..chunk.end
            },
        });
    let per_epoch = epochs.into_iter().flat_map(|config| {
        let first = config.epoch(config.epoch_start_block);
        let candidates = chunk.start / config.epoch_height
            ..=chunk.end.saturating_sub(1) / config.epoch_height + 2;
        let chunk = chunk.clone();
        DataKind::VARIANTS
            .iter()
            .filter(|kind| kind.is_per_epoch())
            .flat_map(move |&kind| {
                let first = if kind == DataKind::LcStakeTable {
                    first + 2
                } else {
                    first
                };
                let chunk = chunk.clone();
                candidates
                    .clone()
                    .filter(move |&e| {
                        e >= first
                            && epoch_block(kind, e, config.epoch_height)
                                .is_some_and(|block| chunk.contains(&block))
                    })
                    .map(move |e| Probe {
                        kind,
                        range: e..e + 1,
                    })
            })
    });
    per_height.chain(per_epoch).collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    const EPOCHS: EpochConfig = EpochConfig {
        epoch_height: 30,
        epoch_start_block: 61,
    };

    type KindRange = (DataKind, Range<u64>);
    type Case = (
        &'static str,
        Range<u64>,
        Option<EpochConfig>,
        Vec<KindRange>,
    );

    fn kinds_and_ranges(probes: Vec<Probe>) -> Vec<KindRange> {
        probes.into_iter().map(|p| (p.kind, p.range)).collect()
    }

    /// The range kinds cover the chunk; the single-height kinds only its last height.
    fn per_height(chunk: Range<u64>) -> Vec<KindRange> {
        let last = chunk.end - 1..chunk.end;
        vec![
            (DataKind::Leaf, chunk.clone()),
            (DataKind::Block, chunk.clone()),
            (DataKind::VidCommon, chunk),
            (DataKind::LcLeafProof, last.clone()),
            (DataKind::Aggregate, last),
        ]
    }

    #[test]
    fn probes_epochs() {
        use DataKind::*;
        // First epoch = ceil(61 / 30) = 3, `lc_stake_table` starts at 5. A stake table of epoch e
        // is probed with the chunk holding block (e - 2) * 30, a state cert with e * 30.
        let epoch_probes = |kinds: &[(DataKind, u64)]| {
            kinds
                .iter()
                .map(|&(k, e)| (k, e..e + 1))
                .collect::<Vec<_>>()
        };
        let cases: Vec<Case> = vec![
            ("no epochs", 0..100, None, per_height(0..100)),
            (
                "stake table of the first epoch comes early",
                0..60,
                Some(EPOCHS),
                [per_height(0..60), epoch_probes(&[(StakeTable, 3)])].concat(),
            ),
            (
                "state cert and lc stake table later",
                60..100,
                Some(EPOCHS),
                [
                    per_height(60..100),
                    epoch_probes(&[(StakeTable, 4), (StakeTable, 5), (StateCert, 3)]),
                    epoch_probes(&[(LcStakeTable, 5)]),
                ]
                .concat(),
            ),
            (
                "several epochs in one chunk",
                100..220,
                Some(EPOCHS),
                [
                    per_height(100..220),
                    epoch_probes(&[
                        (StakeTable, 6),
                        (StakeTable, 7),
                        (StakeTable, 8),
                        (StakeTable, 9),
                    ]),
                    epoch_probes(&[
                        (StateCert, 4),
                        (StateCert, 5),
                        (StateCert, 6),
                        (StateCert, 7),
                    ]),
                    epoch_probes(&[
                        (LcStakeTable, 6),
                        (LcStakeTable, 7),
                        (LcStakeTable, 8),
                        (LcStakeTable, 9),
                    ]),
                ]
                .concat(),
            ),
            (
                "block at chunk start",
                90..100,
                Some(EPOCHS),
                [
                    per_height(90..100),
                    epoch_probes(&[(StakeTable, 5), (StateCert, 3), (LcStakeTable, 5)]),
                ]
                .concat(),
            ),
            (
                "block at chunk end excluded",
                0..90,
                Some(EPOCHS),
                [
                    per_height(0..90),
                    epoch_probes(&[(StakeTable, 3), (StakeTable, 4)]),
                ]
                .concat(),
            ),
        ];
        for (name, chunk, epochs, expected) in cases {
            assert_eq!(kinds_and_ranges(probes(&chunk, epochs)), expected, "{name}");
        }
    }

    #[test]
    fn epoch_block_table() {
        use DataKind::*;
        let cases = [
            (StateCert, 3, Some(90)),
            (StakeTable, 3, Some(30)),
            (LcStakeTable, 5, Some(90)),
            (StakeTable, 2, Some(0)),
            (StakeTable, 1, Some(0)),
            (Leaf, 3, Some(90)),
        ];
        for (kind, epoch, expected) in cases {
            assert_eq!(epoch_block(kind, epoch, 30), expected, "{kind:?} {epoch}");
        }
    }
}
