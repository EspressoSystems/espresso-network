//! Kinds of data, heights and epochs, and the probes that check them.

use std::{fmt, ops::Range};

use serde::Deserialize;
use strum::{IntoStaticStr, VariantArray};

/// Heights per request for every kind: the server's block range limit (`availability.rs:76`).
pub const CHUNK_LEN: u64 = 100;

/// A block height.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Height(pub u64);

impl Height {
    /// `self - blocks`, at least 0.
    pub fn saturating_sub(self, blocks: u64) -> Height {
        Height(self.0.saturating_sub(blocks))
    }

    /// The height after `self`.
    pub fn next(self) -> Height {
        Height(self.0.saturating_add(1))
    }

    /// Blocks from `earlier` to `self`, 0 when `earlier` is above.
    pub fn blocks_since(self, earlier: Height) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

impl fmt::Display for Height {
    /// The number.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Heights in a range.
pub fn height_count(heights: &Range<Height>) -> u64 {
    heights.end.blocks_since(heights.start)
}

/// An epoch number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(pub u64);

impl Epoch {
    /// The epoch after `self`; saturates at the largest number.
    pub fn next(self) -> Epoch {
        Epoch(self.0.saturating_add(1))
    }
}

/// Kinds probed over a range of heights and refined to exact gaps after a 404.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, VariantArray)]
pub enum RangeKind {
    Leaf,
    Block,
    VidCommon,
}

/// Kinds probed at the last height of a chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, VariantArray)]
pub enum PointKind {
    LcLeafProof,
    Aggregate,
}

/// Kinds probed once per epoch, with the chunk holding the epoch's root block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, VariantArray)]
pub enum EpochKind {
    StakeTable,
    StateCert,
    LcStakeTable,
}

impl EpochKind {
    /// Epochs between an object's epoch and its root block's epoch: 2 for stake tables, 0 for the
    /// state cert.
    pub fn root_lag(self) -> u64 {
        match self {
            Self::StakeTable | Self::LcStakeTable => 2,
            Self::StateCert => 0,
        }
    }
}

/// Label of a kind in rows, metrics and logs; `missing_chunk.kind`. The declaration orders of
/// `RangeKind`, `PointKind` and `EpochKind` concatenate to this order, so `Ord` on `DataKind`
/// agrees with it.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, VariantArray, IntoStaticStr, sqlx::Type,
)]
#[strum(serialize_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum DataKind {
    Leaf,
    Block,
    VidCommon,
    LcLeafProof,
    Aggregate,
    StakeTable,
    StateCert,
    LcStakeTable,
}

/// The unit family of a kind label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindClass {
    Range(RangeKind),
    Point(PointKind),
    Epoch(EpochKind),
}

impl DataKind {
    /// The family and typed kind of this label.
    pub fn class(self) -> KindClass {
        match self {
            Self::Leaf => KindClass::Range(RangeKind::Leaf),
            Self::Block => KindClass::Range(RangeKind::Block),
            Self::VidCommon => KindClass::Range(RangeKind::VidCommon),
            Self::LcLeafProof => KindClass::Point(PointKind::LcLeafProof),
            Self::Aggregate => KindClass::Point(PointKind::Aggregate),
            Self::StakeTable => KindClass::Epoch(EpochKind::StakeTable),
            Self::StateCert => KindClass::Epoch(EpochKind::StateCert),
            Self::LcStakeTable => KindClass::Epoch(EpochKind::LcStakeTable),
        }
    }

    /// False for `Aggregate`: the node's aggregator lags the tip, so its rows never alert.
    pub fn can_alert(self) -> bool {
        self != Self::Aggregate
    }

    /// `"heights"` or `"epochs"`, for the page.
    pub fn unit_name(self) -> &'static str {
        match self.class() {
            KindClass::Epoch(_) => "epochs",
            KindClass::Range(_) | KindClass::Point(_) => "heights",
        }
    }
}

impl From<RangeKind> for DataKind {
    /// The label of a range kind.
    fn from(kind: RangeKind) -> Self {
        match kind {
            RangeKind::Leaf => Self::Leaf,
            RangeKind::Block => Self::Block,
            RangeKind::VidCommon => Self::VidCommon,
        }
    }
}

impl From<PointKind> for DataKind {
    /// The label of a point kind.
    fn from(kind: PointKind) -> Self {
        match kind {
            PointKind::LcLeafProof => Self::LcLeafProof,
            PointKind::Aggregate => Self::Aggregate,
        }
    }
}

impl From<EpochKind> for DataKind {
    /// The label of an epoch kind.
    fn from(kind: EpochKind) -> Self {
        match kind {
            EpochKind::StakeTable => Self::StakeTable,
            EpochKind::StateCert => Self::StateCert,
            EpochKind::LcStakeTable => Self::LcStakeTable,
        }
    }
}

/// One presence request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    Range {
        kind: RangeKind,
        heights: Range<Height>,
    },
    Point {
        kind: PointKind,
        height: Height,
    },
    Epoch {
        kind: EpochKind,
        epoch: Epoch,
    },
}

impl Probe {
    /// The kind label.
    pub fn kind(&self) -> DataKind {
        match self {
            Self::Range { kind, .. } => (*kind).into(),
            Self::Point { kind, .. } => (*kind).into(),
            Self::Epoch { kind, .. } => (*kind).into(),
        }
    }

    /// Whether the node may wait for blocks before answering: a range over several heights.
    pub fn waits_server_side(&self) -> bool {
        matches!(self, Self::Range { heights, .. } if height_count(heights) > 1)
    }

    /// Route of the request, as at HEAD; a point probe of height `h` uses `h` where HEAD used
    /// `end - 1`.
    pub fn route(&self) -> String {
        match self {
            Self::Range { kind, heights } => {
                let Range { start, end } = heights;
                match kind {
                    RangeKind::Leaf => format!("v1/availability/leaf/{start}/{end}"),
                    RangeKind::Block => format!("v1/availability/block/{start}/{end}"),
                    RangeKind::VidCommon => format!("v1/availability/vid/common/{start}/{end}"),
                }
            },
            Self::Point { kind, height } => match kind {
                PointKind::LcLeafProof => format!("v1/light-client/leaf/{height}"),
                PointKind::Aggregate => format!("v1/node/transactions/count/{height}"),
            },
            Self::Epoch { kind, epoch } => {
                let epoch = epoch.0;
                match kind {
                    EpochKind::StakeTable => format!("v1/node/stake-table/{epoch}"),
                    EpochKind::StateCert => format!("v1/catchup/{epoch}/state-cert"),
                    EpochKind::LcStakeTable => format!("v1/light-client/stake-table/{epoch}"),
                }
            },
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
    /// Epoch of a block: `ceil(block / epoch_height)`, block 0 in epoch 1.
    fn epoch_of(&self, block: Height) -> Epoch {
        Epoch(block.0.div_ceil(self.epoch_height).max(1))
    }

    /// The block `kind` of `epoch` is probed with: `(epoch - root_lag)` saturating, times
    /// `epoch_height`; `None` on overflow. The only epoch to block mapping. For stake tables this
    /// is the boundary of epoch `epoch - 2`, five blocks after the root block the node builds the
    /// table from (`root_block_in_epoch` subtracts 5).
    pub fn root_block(&self, kind: EpochKind, epoch: Epoch) -> Option<Height> {
        epoch
            .0
            .saturating_sub(kind.root_lag())
            .checked_mul(self.epoch_height)
            .map(Height)
    }
}

/// Next chunk of `[next, until)`, at most `CHUNK_LEN` long; `None` when empty.
pub fn next_chunk(next: Height, until: Height) -> Option<Range<Height>> {
    (next < until).then(|| next..until.min(Height(next.0.saturating_add(CHUNK_LEN))))
}

/// Every probe of `chunk`: each range kind over it, each point kind at its last height, each
/// epoch kind for every epoch whose root block lies in it (from the first epoch, `+ 2` for
/// `LcStakeTable`). `None` config: no epoch probes.
pub fn probes(chunk: &Range<Height>, epoch_config: Option<EpochConfig>) -> Vec<Probe> {
    let ranges = RangeKind::VARIANTS.iter().map(|&kind| Probe::Range {
        kind,
        heights: chunk.clone(),
    });
    let last = Height(chunk.end.0.saturating_sub(1));
    let points = PointKind::VARIANTS
        .iter()
        .map(|&kind| Probe::Point { kind, height: last });
    let epochs = epoch_config.into_iter().flat_map(|config| {
        let first = config.epoch_of(Height(config.epoch_start_block)).0;
        let candidates = chunk.start.0 / config.epoch_height..=last.0 / config.epoch_height + 2;
        EpochKind::VARIANTS.iter().flat_map(move |&kind| {
            let first = if kind == EpochKind::LcStakeTable {
                first + 2
            } else {
                first
            };
            let chunk = chunk.clone();
            candidates
                .clone()
                .filter(move |&e| {
                    e >= first
                        && config
                            .root_block(kind, Epoch(e))
                            .is_some_and(|block| chunk.contains(&block))
                })
                .map(move |e| Probe::Epoch {
                    kind,
                    epoch: Epoch(e),
                })
        })
    });
    ranges.chain(points).chain(epochs).collect()
}

#[cfg(test)]
mod tests {
    use EpochKind::{LcStakeTable, StakeTable, StateCert};
    use pretty_assertions::assert_eq;

    use super::*;

    const EPOCHS: EpochConfig = EpochConfig {
        epoch_height: 30,
        epoch_start_block: 61,
    };

    fn h(range: Range<u64>) -> Range<Height> {
        Height(range.start)..Height(range.end)
    }

    /// The range kinds cover the chunk; the point kinds only its last height.
    fn per_height(chunk: Range<u64>) -> Vec<Probe> {
        let last = Height(chunk.end - 1);
        let mut probes: Vec<_> = RangeKind::VARIANTS
            .iter()
            .map(|&kind| Probe::Range {
                kind,
                heights: h(chunk.clone()),
            })
            .collect();
        probes.extend(
            PointKind::VARIANTS
                .iter()
                .map(|&kind| Probe::Point { kind, height: last }),
        );
        probes
    }

    fn epoch_probes(kinds: &[(EpochKind, u64)]) -> Vec<Probe> {
        kinds
            .iter()
            .map(|&(kind, e)| Probe::Epoch {
                kind,
                epoch: Epoch(e),
            })
            .collect()
    }

    #[test]
    fn probes_epochs() {
        // First epoch = ceil(61 / 30) = 3, `lc_stake_table` starts at 5. A stake table of epoch e
        // is probed with the chunk holding block (e - 2) * 30, a state cert with e * 30.
        #[allow(clippy::type_complexity)]
        let cases: Vec<(&str, Range<u64>, Option<EpochConfig>, Vec<Probe>)> = vec![
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
            assert_eq!(probes(&h(chunk), epochs), expected, "{name}");
        }
    }

    #[test]
    fn root_block_table() {
        let cases = [
            (StateCert, 3, Some(90)),
            (StakeTable, 3, Some(30)),
            (LcStakeTable, 5, Some(90)),
            (StakeTable, 2, Some(0)),
            (StakeTable, 1, Some(0)),
        ];
        for (kind, epoch, expected) in cases {
            assert_eq!(
                EPOCHS.root_block(kind, Epoch(epoch)),
                expected.map(Height),
                "{kind:?} {epoch}"
            );
        }
    }

    #[test]
    fn root_block_overflow_is_none() {
        assert_eq!(
            EPOCHS.root_block(EpochKind::StateCert, Epoch(u64::MAX)),
            None
        );
    }

    #[test]
    fn data_kind_order_concatenates_the_families() {
        let families: Vec<DataKind> = RangeKind::VARIANTS
            .iter()
            .map(|&k| k.into())
            .chain(PointKind::VARIANTS.iter().map(|&k| k.into()))
            .chain(EpochKind::VARIANTS.iter().map(|&k| k.into()))
            .collect();
        assert_eq!(families, DataKind::VARIANTS);
        assert!(DataKind::VARIANTS.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn next_chunk_is_at_most_chunk_len() {
        assert_eq!(next_chunk(Height(5), Height(5)), None);
        assert_eq!(next_chunk(Height(0), Height(250)), Some(h(0..100)));
        assert_eq!(next_chunk(Height(200), Height(250)), Some(h(200..250)));
    }

    #[test]
    fn point_route_uses_the_height() {
        let probe = Probe::Point {
            kind: PointKind::Aggregate,
            height: Height(1199),
        };
        assert_eq!(probe.route(), "v1/node/transactions/count/1199");
    }
}
