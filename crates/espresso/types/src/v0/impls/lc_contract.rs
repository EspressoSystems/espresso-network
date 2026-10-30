//! The LC contract's voting stake table and the candidate table that matches it.

use std::ops::RangeInclusive;

use hotshot_types::{
    PeerConfig,
    data::EpochNumber,
    stake_table::HSStakeTable,
    utils::{epoch_from_block_number, is_epoch_root},
};
use indexmap::IndexMap;
use thiserror::Error;
use vbs::version::Version;

use super::{
    AuthenticatedValidatorMap, StakeTableState, committee::committee_peer_configs,
    select_active_validator_set,
};
use crate::{
    PubKey, SeqTypes,
    v0_3::{EventKey, StakeTableError, StakeTableEvent},
};

/// The circuit commitment of a stake table. Aliased because `StakeTableState` in this crate is the
/// L1 validator state.
pub type StakeTableCommitment = hotshot_types::light_client::StakeTableState;

/// The LC contract's voting stake table, read at one L1 block.
#[derive(Clone, Debug)]
pub struct LcContractVotingStakeTable {
    /// L1 block the getters were called at.
    pub l1_block: u64,
    /// `finalizedState.blockHeight`.
    pub finalized_block_height: u64,
    /// `E'`: epoch whose table `commitment` is.
    pub voting_epoch: EpochNumber,
    /// `votingStakeTableState`: `comm(ST(voting_epoch))`.
    pub commitment: StakeTableCommitment,
}

impl LcContractVotingStakeTable {
    /// `E'`: `epoch(height)`, plus 1 when `height` is an epoch root; `first_epoch` before the first
    /// root. Pure.
    pub fn voting_epoch(
        finalized_block_height: u64,
        epoch_height: u64,
        first_epoch: EpochNumber,
    ) -> EpochNumber {
        let epoch = epoch_from_block_number(finalized_block_height, epoch_height)
            + u64::from(is_epoch_root(finalized_block_height, epoch_height));
        first_epoch.max(EpochNumber::new(epoch))
    }

    /// Whether `table` is the table this commitment is of. Errors when `table` is over `capacity`.
    pub fn commits_to(
        &self,
        table: &HSStakeTable<SeqTypes>,
        capacity: usize,
    ) -> anyhow::Result<bool> {
        Ok(table.commitment(capacity)? == self.commitment)
    }
}

/// Candidate: active set whose commitment equals the LC contract's for `epoch`. Used only to verify
/// the anchor leaf and to dial; never installed.
#[derive(Clone, Debug)]
pub struct LcContractCandidateStakeTable {
    /// Epoch whose table the commitment is.
    pub epoch: EpochNumber,
    /// Latest L1 block at or below the read block whose events give this set.
    pub l1_block: u64,
    /// Version whose selection rules produced the match.
    pub version: Version,
    /// Active validators with connect info as of `l1_block`.
    pub validators: AuthenticatedValidatorMap,
}

impl LcContractCandidateStakeTable {
    /// The latest event block at or below `lc_contract.l1_block` whose active set, under one of
    /// `candidate_versions`, commits to `lc_contract.commitment`; with the validator state after
    /// the last event, which gives `ST(now)` and the registry. Pure.
    ///
    /// One forward fold over `events` (sorted by key). At each block that carries events the
    /// active set of every version is selected, and its commitment computed only when it differs
    /// from the previous block's. A version earlier in `candidate_versions` wins a tie.
    pub fn find(
        lc_contract: &LcContractVotingStakeTable,
        events: &[(EventKey, StakeTableEvent)],
        candidate_versions: &[Version],
        stake_table_capacity: usize,
    ) -> Result<(Self, StakeTableState), LcContractStakeTableMatchError> {
        let mut state = StakeTableState::default();
        let mut found = None;
        // Per version: its active set at the previous block, and whether that set matches.
        let mut previous: Vec<(Option<AuthenticatedValidatorMap>, bool)> =
            vec![(None, false); candidate_versions.len()];
        let mut first_block = None;
        let mut events = events
            .iter()
            .take_while(|((l1_block, _), _)| *l1_block <= lc_contract.l1_block)
            .peekable();
        while let Some(((l1_block, _), event)) = events.next() {
            first_block.get_or_insert(*l1_block);
            if let Err(expected) = state.apply_event(event.clone())? {
                tracing::debug!(%expected, "stake table event skipped");
            }
            if events.peek().is_some_and(|((next, _), _)| next == l1_block) {
                continue;
            }
            for (version, (active, matches)) in candidate_versions.iter().zip(&mut previous) {
                let selected = match select_active_validator_set(state.validators(), *version) {
                    Ok(selected) => selected,
                    Err(StakeTableError::NoValidValidators) => {
                        (*active, *matches) = (None, false);
                        continue;
                    },
                    Err(err) => return Err(err.into()),
                };
                if active.as_ref() != Some(&selected) {
                    let commitment = peer_configs_of(&selected)
                        .commitment(stake_table_capacity)
                        .map_err(|source| LcContractStakeTableMatchError::Commitment { source })?;
                    *matches = commitment == lc_contract.commitment;
                    *active = Some(selected);
                }
            }
            let matching = candidate_versions.iter().zip(&previous).find_map(
                |(version, (active, matches))| {
                    active
                        .clone()
                        .filter(|_| *matches)
                        .map(|validators| (*version, validators))
                },
            );
            if let Some((version, validators)) = matching {
                found = Some(Self {
                    epoch: lc_contract.voting_epoch,
                    l1_block: *l1_block,
                    version,
                    validators,
                });
            }
        }
        let searched_l1_blocks = first_block.unwrap_or(lc_contract.l1_block)..=lc_contract.l1_block;
        found.map(|found| (found, state)).ok_or(
            LcContractStakeTableMatchError::NoLcContractCommitmentMatch {
                epoch: lc_contract.voting_epoch,
                searched_l1_blocks,
            },
        )
    }

    /// Entries in committee order.
    pub fn peer_configs(&self) -> IndexMap<PubKey, PeerConfig<SeqTypes>> {
        committee_peer_configs(&self.validators)
    }

    /// Stake table of `epoch` that verifies its certificates: `installed`, else this candidate when
    /// `epoch == self.epoch`. Pure.
    ///
    /// The caller reads `installed` through `membership().snapshot(epoch)`, public and never
    /// starting catchup; never `stake_table_for_epoch` or `membership_for_epoch`, which start HTTP
    /// catchup. It derives entries with `StakeTableEntries::from` and the threshold with
    /// `supermajority_threshold(total_stakes())`.
    pub fn quorum_for(
        &self,
        epoch: EpochNumber,
        installed: Option<HSStakeTable<SeqTypes>>,
    ) -> Option<HSStakeTable<SeqTypes>> {
        installed.or_else(|| (epoch == self.epoch).then(|| peer_configs_of(&self.validators)))
    }
}

/// The committee-ordered entries of `validators` as a stake table.
fn peer_configs_of(validators: &AuthenticatedValidatorMap) -> HSStakeTable<SeqTypes> {
    HSStakeTable::from_iter(committee_peer_configs(validators).values())
}

/// The validator state after all `events` up to `to_l1_block`.
pub fn stake_table_state_at(
    events: &[(EventKey, StakeTableEvent)],
    to_l1_block: u64,
) -> Result<StakeTableState, StakeTableError> {
    let mut state = StakeTableState::default();
    for (_, event) in events
        .iter()
        .take_while(|((l1_block, _), _)| *l1_block <= to_l1_block)
    {
        if let Err(expected) = state.apply_event(event.clone())? {
            tracing::debug!(%expected, "stake table event skipped");
        }
    }
    Ok(state)
}

/// Why `find` produced no candidate.
#[derive(Debug, Error)]
pub enum LcContractStakeTableMatchError {
    /// No L1 block and version reproduce the LC contract commitment.
    #[error(
        "no stake table for epoch {epoch} matches the LC contract commitment in L1 blocks \
         {searched_l1_blocks:?}"
    )]
    NoLcContractCommitmentMatch {
        epoch: EpochNumber,
        searched_l1_blocks: RangeInclusive<u64>,
    },
    /// Applying an L1 event or selecting the active set failed fatally.
    #[error(transparent)]
    StakeTable(#[from] StakeTableError),
    /// The commitment of a candidate could not be computed (over capacity).
    #[error("failed to compute the stake table commitment")]
    Commitment { source: anyhow::Error },
}

#[cfg(test)]
mod tests {
    use alloy::primitives::{Address, U256};
    use hotshot_contract_adapter::sol_types::StakeTableV3::{
        Delegated, ValidatorRegistered, ValidatorRegisteredV3,
    };
    use rstest::rstest;
    use versions::{EPOCH_VERSION, NEW_PROTOCOL_VERSION};

    use super::*;
    use crate::{ValidatorSet, v0::impls::testing::TestValidator};

    const EPOCH_HEIGHT: u64 = 100;
    const CAPACITY: usize = 10;
    const FIRST: u64 = 3;

    #[rstest]
    // Before the first root: the genesis table.
    #[case(0, 3)]
    #[case(50, 3)]
    #[case(299, 3)]
    // The root of the first epoch (block 295): `first + 1`, the seeded genesis pair.
    #[case(295, 4)]
    // Another root: `epoch + 1`.
    #[case(395, 5)]
    #[case(495, 6)]
    // Not a root: `epoch`.
    #[case(396, 4)]
    #[case(400, 4)]
    #[case(401, 5)]
    fn voting_epoch(#[case] height: u64, #[case] expected: u64) {
        assert_eq!(
            LcContractVotingStakeTable::voting_epoch(height, EPOCH_HEIGHT, EpochNumber::new(FIRST)),
            EpochNumber::new(expected)
        );
    }

    #[test]
    fn voting_epoch_at_root_ok() {
        let first = EpochNumber::new(1);
        let root = hotshot_types::utils::root_block_in_epoch(7, EPOCH_HEIGHT);
        assert_eq!(
            LcContractVotingStakeTable::voting_epoch(root, EPOCH_HEIGHT, first),
            EpochNumber::new(8)
        );
        assert_eq!(
            LcContractVotingStakeTable::voting_epoch(root + 1, EPOCH_HEIGHT, first),
            EpochNumber::new(7)
        );
    }

    /// A registered validator with `stake` delegated, one registration event and one delegation.
    fn validator_events(
        validator: &TestValidator,
        register: StakeTableEvent,
        stake: u64,
    ) -> [StakeTableEvent; 2] {
        [
            register,
            Delegated {
                delegator: Address::random(),
                validator: validator.account,
                amount: U256::from(stake),
            }
            .into(),
        ]
    }

    /// Eligible at every version: registered with an x25519 key and a p2p address.
    fn v3_events(stake: u64) -> [StakeTableEvent; 2] {
        let validator = TestValidator::random();
        let register = ValidatorRegisteredV3::from(&validator).into();
        validator_events(&validator, register, stake)
    }

    /// Ineligible from `NEW_PROTOCOL_VERSION` on: no x25519 key.
    fn v1_events(stake: u64) -> [StakeTableEvent; 2] {
        let validator = TestValidator::random();
        let register = ValidatorRegistered::from(&validator).into();
        validator_events(&validator, register, stake)
    }

    /// Events keyed at `l1_block`, with consecutive log indices.
    fn at_block(
        l1_block: u64,
        events: impl IntoIterator<Item = StakeTableEvent>,
    ) -> Vec<(EventKey, StakeTableEvent)> {
        events
            .into_iter()
            .enumerate()
            .map(|(log_index, event)| ((l1_block, log_index as u64), event))
            .collect()
    }

    fn commitment_of(
        events: &[(EventKey, StakeTableEvent)],
        version: Version,
    ) -> StakeTableCommitment {
        let set =
            ValidatorSet::from_l1_events(events.iter().map(|(_, event)| event.clone()), version)
                .unwrap();
        peer_configs_of(set.active_validators())
            .commitment(CAPACITY)
            .unwrap()
    }

    fn lc_contract(l1_block: u64, commitment: StakeTableCommitment) -> LcContractVotingStakeTable {
        LcContractVotingStakeTable {
            l1_block,
            finalized_block_height: 0,
            voting_epoch: EpochNumber::new(5),
            commitment,
        }
    }

    #[test]
    fn match_across_upgrade_ok() {
        let mut events = at_block(10, v1_events(5).into_iter().chain(v3_events(7)));
        events.extend(at_block(12, v3_events(9)));
        // The set at block 12 under the first version differs; the match is the older block under
        // the second version only if the newer block is not the match.
        let target = events.iter().take(4).cloned().collect::<Vec<_>>();
        let lc = lc_contract(20, commitment_of(&target, EPOCH_VERSION));

        let (found, _) = LcContractCandidateStakeTable::find(
            &lc,
            &events[..4],
            &[NEW_PROTOCOL_VERSION, EPOCH_VERSION],
            CAPACITY,
        )
        .unwrap();

        assert_eq!(found.version, EPOCH_VERSION);
        assert_eq!(found.l1_block, 10);
        assert_eq!(found.validators.len(), 2);
    }

    #[test]
    fn latest_block_wins() {
        let mut events = at_block(10, v3_events(5));
        // A validator without stake does not change the active set.
        let idle = TestValidator::random();
        events.extend(at_block(12, [ValidatorRegisteredV3::from(&idle).into()]));
        let lc = lc_contract(20, commitment_of(&events, EPOCH_VERSION));

        let found = LcContractCandidateStakeTable::find(&lc, &events, &[EPOCH_VERSION], CAPACITY)
            .unwrap()
            .0;

        assert_eq!(found.l1_block, 12);
        assert_eq!(found.epoch, EpochNumber::new(5));
    }

    #[test]
    fn events_above_the_read_block_are_ignored() {
        let mut events = at_block(10, v3_events(5));
        let lc = lc_contract(11, commitment_of(&events, EPOCH_VERSION));
        events.extend(at_block(15, v3_events(8)));

        let found = LcContractCandidateStakeTable::find(&lc, &events, &[EPOCH_VERSION], CAPACITY)
            .unwrap()
            .0;

        assert_eq!(found.l1_block, 10);
    }

    #[test]
    fn no_match_fails() {
        let events = at_block(10, v3_events(5));
        let lc = lc_contract(
            20,
            commitment_of(&at_block(10, v3_events(5)), EPOCH_VERSION),
        );

        let err = LcContractCandidateStakeTable::find(&lc, &events, &[EPOCH_VERSION], CAPACITY)
            .unwrap_err();

        let LcContractStakeTableMatchError::NoLcContractCommitmentMatch {
            epoch,
            searched_l1_blocks,
        } = err
        else {
            panic!("unexpected error: {err:#}");
        };
        assert_eq!(epoch, EpochNumber::new(5));
        assert_eq!(searched_l1_blocks, 10..=20);
    }

    #[test]
    fn quorum_for_falls_back_to_candidate() {
        let events = at_block(10, v3_events(5));
        let lc = lc_contract(20, commitment_of(&events, EPOCH_VERSION));
        let (candidate, _) =
            LcContractCandidateStakeTable::find(&lc, &events, &[EPOCH_VERSION], CAPACITY).unwrap();

        let table = candidate.quorum_for(candidate.epoch, None).unwrap();

        assert_eq!(table.len(), candidate.validators.len());
        assert!(candidate.quorum_for(EpochNumber::new(6), None).is_none());
        // An installed table takes precedence.
        let installed = HSStakeTable::<SeqTypes>::default();
        assert_eq!(
            candidate.quorum_for(candidate.epoch, Some(installed.clone())),
            Some(installed)
        );
    }
}
