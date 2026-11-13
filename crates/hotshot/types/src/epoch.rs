// Copyright (c) 2026 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.
// SPDX-License-Identifier: MIT

//! Checked mathematical epoch geometry, independent of historical activation/version rules.

use std::num::NonZeroU64;

use thiserror::Error;

use crate::data::{BlockNumber, EpochNumber};

/// A mathematical epoch length. Production root geometry may require a larger minimum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpochLength(NonZeroU64);

impl EpochLength {
    pub fn new(blocks: u64) -> Result<Self, EpochArithmeticError> {
        NonZeroU64::new(blocks)
            .map(Self)
            .ok_or(EpochArithmeticError::ZeroLength)
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }

    /// Genesis belongs mathematically to epoch one; this does not imply epochs are enabled.
    pub fn epoch_of(self, height: BlockNumber) -> EpochNumber {
        EpochNumber::new(if *height == 0 {
            1
        } else {
            (*height - 1) / self.get() + 1
        })
    }

    /// Epoch one starts at genesis; later epochs start one block after the prior boundary.
    pub fn first_block(self, epoch: EpochNumber) -> Result<BlockNumber, EpochArithmeticError> {
        require_epoch(epoch)?;
        if *epoch == 1 {
            return Ok(BlockNumber::new(0));
        }
        (*epoch - 1)
            .checked_mul(self.get())
            .and_then(|height| height.checked_add(1))
            .map(BlockNumber::new)
            .ok_or(EpochArithmeticError::Overflow)
    }

    pub fn last_block(self, epoch: EpochNumber) -> Result<BlockNumber, EpochArithmeticError> {
        require_epoch(epoch)?;
        epoch
            .checked_mul(self.get())
            .map(BlockNumber::new)
            .ok_or(EpochArithmeticError::Overflow)
    }

    /// The root inside `epoch`, which derives the stake table two epochs later.
    pub fn root_inside(self, epoch: EpochNumber) -> Result<BlockNumber, EpochArithmeticError> {
        self.block_before_boundary(epoch, 5)
    }

    /// The transition leaving `epoch`, which carries the next epoch's DRB.
    pub fn transition_leaving(
        self,
        epoch: EpochNumber,
    ) -> Result<BlockNumber, EpochArithmeticError> {
        self.block_before_boundary(epoch, 3)
    }

    pub fn boundary_at_or_before(self, height: BlockNumber) -> BlockNumber {
        BlockNumber::new(*height - *height % self.get())
    }

    pub fn position(self, height: BlockNumber) -> EpochPosition {
        EpochPosition {
            length: self,
            epoch: self.epoch_of(height),
            offset: if *height == 0 {
                0
            } else {
                (*height - 1) % self.get() + 1
            },
        }
    }

    fn block_before_boundary(
        self,
        epoch: EpochNumber,
        distance: u64,
    ) -> Result<BlockNumber, EpochArithmeticError> {
        require_epoch(epoch)?;
        if self.get() <= distance {
            return Err(EpochArithmeticError::NoPosition { distance });
        }
        // The boundary must itself exist; subtracting an offset must not hide its overflow.
        Ok(BlockNumber::new(*self.last_block(epoch)? - distance))
    }
}

/// A position within a mathematical epoch. Genesis has offset zero in epoch one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpochPosition {
    length: EpochLength,
    epoch: EpochNumber,
    offset: u64,
}

impl EpochPosition {
    pub fn epoch(self) -> EpochNumber {
        self.epoch
    }

    pub fn is_first(self) -> bool {
        if *self.epoch == 1 {
            self.offset == 0
        } else {
            self.offset == 1
        }
    }

    pub fn is_last(self) -> bool {
        self.offset == self.length.get()
    }

    pub fn is_root(self) -> bool {
        self.is_before_boundary(5)
    }

    pub fn is_transition_block(self) -> bool {
        self.is_before_boundary(3)
    }

    /// The first block after the transition block, matching the legacy predicate's naming.
    pub fn is_first_transition_block(self) -> bool {
        self.is_before_boundary(2)
    }

    pub fn is_middle_transition_block(self) -> bool {
        self.is_before_boundary(1) || self.is_before_boundary(2)
    }

    /// Includes the transition block and the epoch's last block.
    pub fn is_epoch_transition(self) -> bool {
        self.is_within_boundary(3)
    }

    pub fn is_at_or_after_root(self) -> bool {
        self.is_within_boundary(5)
    }

    pub fn is_after_root(self) -> bool {
        self.is_at_or_after_root() && !self.is_root()
    }

    fn is_before_boundary(self, distance: u64) -> bool {
        self.length.get() > distance && self.blocks_to_boundary() == Some(distance)
    }

    fn is_within_boundary(self, distance: u64) -> bool {
        self.length.get() > distance
            && self
                .blocks_to_boundary()
                .is_some_and(|left| left <= distance)
    }

    /// Blocks left until this epoch's last block. `None` at genesis, which precedes every
    /// boundary position.
    fn blocks_to_boundary(self) -> Option<u64> {
        (self.offset > 0).then(|| self.length.get() - self.offset)
    }
}

/// The activation epoch is explicit: mathematical epoch assignment alone never enables epochs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpochSchedule {
    length: EpochLength,
    first_epoch: EpochNumber,
}

impl EpochSchedule {
    pub fn new(
        length: EpochLength,
        first_epoch: EpochNumber,
    ) -> Result<Self, EpochArithmeticError> {
        require_epoch(first_epoch)?;
        Ok(Self {
            length,
            first_epoch,
        })
    }

    pub fn length(self) -> EpochLength {
        self.length
    }

    pub fn first_epoch(self) -> EpochNumber {
        self.first_epoch
    }

    pub fn seeded_epochs(self) -> Result<EpochWindow, EpochArithmeticError> {
        EpochWindow::new(self.first_epoch, self.first_epoch.checked_advance(1)?)
    }

    pub fn first_dynamic_epoch(self) -> Result<EpochNumber, EpochArithmeticError> {
        self.first_epoch.checked_advance(2)
    }

    pub fn table_source(
        self,
        epoch: EpochNumber,
    ) -> Result<StakeTableSource, EpochArithmeticError> {
        if self.is_seeded(epoch)? {
            return Ok(StakeTableSource::GenesisSeed);
        }
        let root_epoch = epoch.checked_rewind(2)?;
        Ok(StakeTableSource::EpochRoot {
            root_epoch,
            height: self.length.root_inside(root_epoch)?,
        })
    }

    pub fn drb_source(self, epoch: EpochNumber) -> Result<DrbSource, EpochArithmeticError> {
        if self.is_seeded(epoch)? {
            return Ok(DrbSource::GenesisSeed);
        }
        let leaving_epoch = epoch.checked_rewind(1)?;
        Ok(DrbSource::EpochTransition {
            leaving_epoch,
            height: self.length.transition_leaving(leaving_epoch)?,
        })
    }

    /// Adjacent peer epochs, clipped at activation. No membership lookup or acquisition occurs.
    pub fn peer_window(self, current: EpochNumber) -> Result<EpochWindow, EpochArithmeticError> {
        self.require_active(current)?;
        let first = if current == self.first_epoch {
            current
        } else {
            current.checked_rewind(1)?
        };
        EpochWindow::new(first, current.checked_advance(1)?)
    }

    fn is_seeded(self, epoch: EpochNumber) -> Result<bool, EpochArithmeticError> {
        self.require_active(epoch)?;
        Ok(*epoch - *self.first_epoch < 2)
    }

    fn require_active(self, epoch: EpochNumber) -> Result<(), EpochArithmeticError> {
        require_epoch(epoch)?;
        if epoch < self.first_epoch {
            return Err(EpochArithmeticError::BeforeActivation {
                epoch,
                first_epoch: self.first_epoch,
            });
        }
        Ok(())
    }
}

/// Source of the target epoch's stake table, distinct from a root inside the target epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StakeTableSource {
    GenesisSeed,
    EpochRoot {
        root_epoch: EpochNumber,
        height: BlockNumber,
    },
}

/// Source carrying an accepted DRB; computing a DRB from a root is a separate protocol policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrbSource {
    GenesisSeed,
    EpochTransition {
        leaving_epoch: EpochNumber,
        height: BlockNumber,
    },
}

/// A nonempty inclusive window of positive epochs, independent of a particular retention policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpochWindow {
    first: EpochNumber,
    last: EpochNumber,
}

impl EpochWindow {
    pub fn new(first: EpochNumber, last: EpochNumber) -> Result<Self, EpochArithmeticError> {
        require_epoch(first)?;
        require_epoch(last)?;
        if first > last {
            return Err(EpochArithmeticError::ReversedWindow);
        }
        Ok(Self { first, last })
    }

    pub fn first(self) -> EpochNumber {
        self.first
    }

    pub fn last(self) -> EpochNumber {
        self.last
    }

    pub fn contains(self, epoch: EpochNumber) -> bool {
        self.first <= epoch && epoch <= self.last
    }
}

impl EpochNumber {
    /// Checked forward adjacency within the positive mathematical epoch domain.
    pub fn checked_advance(self, count: u64) -> Result<Self, EpochArithmeticError> {
        require_epoch(self)?;
        self.checked_add(count)
            .map(Self::new)
            .ok_or(EpochArithmeticError::Overflow)
    }

    /// Checked backward adjacency; the wire-compatible epoch-zero sentinel is not a predecessor.
    pub fn checked_rewind(self, count: u64) -> Result<Self, EpochArithmeticError> {
        require_epoch(self)?;
        let epoch = self
            .checked_sub(count)
            .map(Self::new)
            .ok_or(EpochArithmeticError::ZeroEpoch)?;
        require_epoch(epoch)?;
        Ok(epoch)
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum EpochArithmeticError {
    #[error("epoch length must be nonzero")]
    ZeroLength,
    #[error("epoch zero has no mathematical epoch geometry")]
    ZeroEpoch,
    #[error("epoch arithmetic exceeds u64")]
    Overflow,
    #[error("epoch length must exceed boundary offset {distance}")]
    NoPosition { distance: u64 },
    #[error("epoch {epoch} precedes first activation epoch {first_epoch}")]
    BeforeActivation {
        epoch: EpochNumber,
        first_epoch: EpochNumber,
    },
    #[error("epoch window ends before it starts")]
    ReversedWindow,
}

fn require_epoch(epoch: EpochNumber) -> Result<(), EpochArithmeticError> {
    if *epoch == 0 {
        return Err(EpochArithmeticError::ZeroEpoch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch(value: u64) -> EpochNumber {
        EpochNumber::new(value)
    }

    fn height(value: u64) -> BlockNumber {
        BlockNumber::new(value)
    }

    #[test]
    fn boundary_positions_and_genesis() {
        let length = EpochLength::new(10).unwrap();
        let cases = [
            // height, epoch, first, root, transition, last
            (0, 1, true, false, false, false),
            (1, 1, false, false, false, false),
            (4, 1, false, false, false, false),
            (5, 1, false, true, false, false),
            (6, 1, false, false, false, false),
            (7, 1, false, false, true, false),
            (8, 1, false, false, false, false),
            (9, 1, false, false, false, false),
            (10, 1, false, false, false, true),
            (11, 2, true, false, false, false),
            (15, 2, false, true, false, false),
            (20, 2, false, false, false, true),
            (21, 3, true, false, false, false),
        ];
        for (h, e, first, root, transition, last) in cases {
            let position = length.position(height(h));
            assert_eq!(position.epoch(), epoch(e), "height {h}");
            assert_eq!(position.is_first(), first, "height {h}");
            assert_eq!(position.is_root(), root, "height {h}");
            assert_eq!(position.is_transition_block(), transition, "height {h}");
            assert_eq!(position.is_last(), last, "height {h}");
            assert_eq!(
                position.is_epoch_transition(),
                (7..=10).contains(&h) || h == 20
            );
            assert_eq!(position.is_middle_transition_block(), h == 8 || h == 9);
            assert_eq!(position.is_first_transition_block(), h == 8);
        }
        assert_eq!(length.first_block(epoch(1)), Ok(height(0)));
        assert_eq!(length.first_block(epoch(2)), Ok(height(11)));
        assert_eq!(length.boundary_at_or_before(height(0)), height(0));
        assert_eq!(length.boundary_at_or_before(height(9)), height(0));
        assert_eq!(length.boundary_at_or_before(height(10)), height(10));
        assert_eq!(length.boundary_at_or_before(height(11)), height(10));
    }

    #[test]
    fn seeds_and_dynamic_sources_are_distinct_from_inside_positions() {
        let length = EpochLength::new(10).unwrap();
        let schedule = EpochSchedule::new(length, epoch(7)).unwrap();
        assert_eq!(schedule.length(), length);
        assert_eq!(schedule.first_epoch(), epoch(7));
        assert_eq!(
            schedule.seeded_epochs().unwrap(),
            EpochWindow::new(epoch(7), epoch(8)).unwrap()
        );
        assert_eq!(schedule.first_dynamic_epoch(), Ok(epoch(9)));
        for e in [7, 8] {
            assert_eq!(
                schedule.table_source(epoch(e)),
                Ok(StakeTableSource::GenesisSeed)
            );
            assert_eq!(schedule.drb_source(epoch(e)), Ok(DrbSource::GenesisSeed));
        }
        assert_eq!(
            schedule.table_source(epoch(9)),
            Ok(StakeTableSource::EpochRoot {
                root_epoch: epoch(7),
                height: height(65),
            })
        );
        assert_eq!(
            schedule.drb_source(epoch(9)),
            Ok(DrbSource::EpochTransition {
                leaving_epoch: epoch(8),
                height: height(77),
            })
        );
        assert_eq!(length.root_inside(epoch(9)), Ok(height(85)));
        assert_eq!(length.transition_leaving(epoch(9)), Ok(height(87)));
        assert!(matches!(
            schedule.table_source(epoch(6)),
            Err(EpochArithmeticError::BeforeActivation { .. })
        ));
        assert!(matches!(
            schedule.drb_source(epoch(6)),
            Err(EpochArithmeticError::BeforeActivation { .. })
        ));
    }

    #[test]
    fn small_lengths_do_not_invent_positions_in_prior_epochs() {
        assert_eq!(EpochLength::new(0), Err(EpochArithmeticError::ZeroLength));
        for h in 1..=5 {
            let length = EpochLength::new(h).unwrap();
            assert_eq!(length.epoch_of(height(h)), epoch(1));
            assert_eq!(length.epoch_of(height(h + 1)), epoch(2));
            assert_eq!(
                length.root_inside(epoch(2)),
                Err(EpochArithmeticError::NoPosition { distance: 5 })
            );
            assert!(!length.position(height(h)).is_root());
            if h <= 3 {
                assert_eq!(
                    length.transition_leaving(epoch(2)),
                    Err(EpochArithmeticError::NoPosition { distance: 3 })
                );
                assert!(!length.position(height(h)).is_transition_block());
            } else {
                assert_eq!(length.transition_leaving(epoch(2)), Ok(height(2 * h - 3)));
            }
            let schedule = EpochSchedule::new(length, epoch(1)).unwrap();
            assert_eq!(
                schedule.table_source(epoch(2)),
                Ok(StakeTableSource::GenesisSeed)
            );
            assert!(schedule.table_source(epoch(3)).is_err());
        }
    }

    #[test]
    fn zero_epoch_and_checked_overflow_are_not_genesis() {
        let length = EpochLength::new(10).unwrap();
        assert_eq!(
            length.first_block(epoch(0)),
            Err(EpochArithmeticError::ZeroEpoch)
        );
        assert_eq!(
            length.last_block(epoch(0)),
            Err(EpochArithmeticError::ZeroEpoch)
        );
        assert_eq!(
            length.root_inside(epoch(0)),
            Err(EpochArithmeticError::ZeroEpoch)
        );
        assert_eq!(
            length.transition_leaving(epoch(0)),
            Err(EpochArithmeticError::ZeroEpoch)
        );
        assert_eq!(
            EpochSchedule::new(length, epoch(0)),
            Err(EpochArithmeticError::ZeroEpoch)
        );
        assert_eq!(
            epoch(1).checked_rewind(1),
            Err(EpochArithmeticError::ZeroEpoch)
        );
        assert_eq!(
            epoch(1).checked_rewind(2),
            Err(EpochArithmeticError::ZeroEpoch)
        );
        assert_eq!(
            epoch(0).checked_advance(1),
            Err(EpochArithmeticError::ZeroEpoch)
        );
        assert_eq!(
            epoch(u64::MAX).checked_advance(1),
            Err(EpochArithmeticError::Overflow)
        );
        assert_eq!(epoch(2).checked_rewind(1), Ok(epoch(1)));
        for e in [u64::MAX / 10 + 1, u64::MAX] {
            assert_eq!(
                length.last_block(epoch(e)),
                Err(EpochArithmeticError::Overflow)
            );
            assert_eq!(
                length.root_inside(epoch(e)),
                Err(EpochArithmeticError::Overflow)
            );
            assert_eq!(
                length.transition_leaving(epoch(e)),
                Err(EpochArithmeticError::Overflow)
            );
        }
        assert_eq!(
            length.first_block(epoch(u64::MAX)),
            Err(EpochArithmeticError::Overflow)
        );
        let schedule = EpochSchedule::new(length, epoch(u64::MAX)).unwrap();
        assert_eq!(
            schedule.seeded_epochs(),
            Err(EpochArithmeticError::Overflow)
        );
        assert_eq!(
            schedule.first_dynamic_epoch(),
            Err(EpochArithmeticError::Overflow)
        );
        assert_eq!(
            schedule.table_source(epoch(u64::MAX)),
            Ok(StakeTableSource::GenesisSeed)
        );
        assert_eq!(
            schedule.peer_window(epoch(u64::MAX)),
            Err(EpochArithmeticError::Overflow)
        );
    }

    #[test]
    fn maximum_height_position_is_total() {
        for blocks in [1, 2, 3, 5, 6, 10, u64::MAX - 1, u64::MAX] {
            let length = EpochLength::new(blocks).unwrap();
            let position = length.position(height(u64::MAX));
            assert_eq!(
                *position.epoch() as u128,
                (u64::MAX as u128).div_ceil(blocks as u128)
            );
            let remaining = (blocks - u64::MAX % blocks) % blocks;
            assert_eq!(position.is_root(), blocks > 5 && remaining == 5);
            assert_eq!(position.is_transition_block(), blocks > 3 && remaining == 3);
            assert_eq!(position.is_last(), remaining == 0);
            assert_eq!(position.is_at_or_after_root(), blocks > 5 && remaining <= 5);
            assert_eq!(position.is_after_root(), blocks > 5 && remaining < 5);
        }
    }

    #[test]
    fn peer_window_clips_only_at_activation() {
        let schedule = EpochSchedule::new(EpochLength::new(10).unwrap(), epoch(7)).unwrap();
        let first = schedule.peer_window(epoch(7)).unwrap();
        assert_eq!((first.first(), first.last()), (epoch(7), epoch(8)));
        assert!(!first.contains(epoch(6)));
        assert!(first.contains(epoch(7)));
        assert!(first.contains(epoch(8)));
        assert!(!first.contains(epoch(9)));
        let next = schedule.peer_window(epoch(8)).unwrap();
        assert_eq!((next.first(), next.last()), (epoch(7), epoch(9)));
        assert!(schedule.peer_window(epoch(6)).is_err());
        assert_eq!(
            EpochWindow::new(epoch(0), epoch(1)),
            Err(EpochArithmeticError::ZeroEpoch)
        );
        assert_eq!(
            EpochWindow::new(epoch(2), epoch(1)),
            Err(EpochArithmeticError::ReversedWindow)
        );
    }

    #[test]
    fn solidity_v2_v3_boundary_vectors() {
        // LightClientV2.sol's epochFromBlockNumber/isEpochRoot/isGtEpochRoot;
        // V3 inherits this geometry. Values avoid Solidity's checked h+5 overflow domain.
        let length = EpochLength::new(10).unwrap();
        for (h, e, root, after_root) in [
            (0, 1, false, false),
            (1, 1, false, false),
            (4, 1, false, false),
            (5, 1, true, false),
            (6, 1, false, true),
            (10, 1, false, true),
            (11, 2, false, false),
            (15, 2, true, false),
            (16, 2, false, true),
            (20, 2, false, true),
        ] {
            let position = length.position(height(h));
            assert_eq!(position.epoch(), epoch(e));
            assert_eq!(position.is_root(), root);
            assert_eq!(position.is_after_root(), after_root);
            assert_eq!(position.is_at_or_after_root(), root || after_root);
        }
        let length = EpochLength::new(6).unwrap();
        assert!(length.position(height(1)).is_root());
        assert!(length.position(height(3)).is_transition_block());
    }

    quickcheck::quickcheck! {
        fn mapping_and_boundaries_agree_with_wide_arithmetic(h: u64, nonzero_length: u64) -> bool {
            let blocks = nonzero_length.max(1);
            let length = EpochLength::new(blocks).unwrap();
            let expected = if h == 0 { 1 } else { (h as u128).div_ceil(blocks as u128) };
            let epoch = length.epoch_of(height(h));
            if *epoch as u128 != expected || length.position(height(h)).epoch() != epoch {
                return false;
            }
            let boundary = length.boundary_at_or_before(height(h));
            if *boundary > h || !boundary.is_multiple_of(blocks) || h - *boundary >= blocks {
                return false;
            }
            let first = length.first_block(epoch).unwrap();
            if first > height(h) || length.epoch_of(first) != epoch {
                return false;
            }
            match length.last_block(epoch) {
                Ok(last) => last >= height(h) && length.epoch_of(last) == epoch,
                Err(EpochArithmeticError::Overflow) => expected * blocks as u128 > u64::MAX as u128,
                _ => false,
            }
        }
    }
}
