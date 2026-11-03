use std::ops::Range;

use serde::Deserialize;
use strum::{EnumString, IntoStaticStr, VariantArray};

/// Heights per request for every kind; the block range limit (`availability.rs:76`).
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

/// Every probe for `chunk`: each per-height kind over `chunk`, and each per-epoch kind for every
/// epoch `e` with `e * epoch_height` in `chunk`, `e >= epoch(epoch_start_block)`
/// (`+ 2` for `LcStakeTable`), as `[e, e + 1)`. `None` epochs: per-height kinds only.
pub fn probes(chunk: &Range<u64>, epochs: Option<EpochConfig>) -> Vec<Probe> {
    let per_height = DataKind::VARIANTS
        .iter()
        .filter(|kind| !kind.is_per_epoch())
        .map(|&kind| Probe {
            kind,
            range: chunk.clone(),
        });
    let per_epoch = epochs.into_iter().flat_map(|config| {
        let first = config.epoch(config.epoch_start_block);
        let in_chunk = chunk.start.div_ceil(config.epoch_height)
            ..=chunk.end.saturating_sub(1) / config.epoch_height;
        DataKind::VARIANTS
            .iter()
            .filter(|kind| kind.is_per_epoch())
            .flat_map(move |&kind| {
                let first = if kind == DataKind::LcStakeTable {
                    first + 2
                } else {
                    first
                };
                in_chunk
                    .clone()
                    .filter(move |&e| e >= first)
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

    fn per_height(chunk: Range<u64>) -> Vec<KindRange> {
        [
            DataKind::Leaf,
            DataKind::Block,
            DataKind::VidCommon,
            DataKind::LcLeafProof,
            DataKind::Aggregate,
        ]
        .map(|kind| (kind, chunk.clone()))
        .to_vec()
    }

    #[test]
    fn probes_epochs() {
        use DataKind::*;
        // first epoch = ceil(61 / 30) = 3; lc_stake_table starts at 5
        let epoch_probes = |kinds: &[(DataKind, u64)]| {
            kinds
                .iter()
                .map(|&(k, e)| (k, e..e + 1))
                .collect::<Vec<_>>()
        };
        let cases: Vec<Case> = vec![
            ("no epochs", 0..100, None, per_height(0..100)),
            ("epoch before first", 0..60, Some(EPOCHS), per_height(0..60)),
            (
                "first epoch, no lc_stake_table yet",
                60..100,
                Some(EPOCHS),
                [
                    per_height(60..100),
                    epoch_probes(&[(StakeTable, 3), (StateCert, 3)]),
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
                        (StakeTable, 4),
                        (StakeTable, 5),
                        (StakeTable, 6),
                        (StakeTable, 7),
                    ]),
                    epoch_probes(&[
                        (StateCert, 4),
                        (StateCert, 5),
                        (StateCert, 6),
                        (StateCert, 7),
                    ]),
                    epoch_probes(&[(LcStakeTable, 5), (LcStakeTable, 6), (LcStakeTable, 7)]),
                ]
                .concat(),
            ),
            (
                "epoch end at chunk start",
                90..100,
                Some(EPOCHS),
                [
                    per_height(90..100),
                    epoch_probes(&[(StakeTable, 3), (StateCert, 3)]),
                ]
                .concat(),
            ),
            (
                "epoch end excluded at chunk end",
                0..90,
                Some(EPOCHS),
                per_height(0..90),
            ),
        ];
        for (name, chunk, epochs, expected) in cases {
            assert_eq!(kinds_and_ranges(probes(&chunk, epochs)), expected, "{name}");
        }
    }
}
