// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::sync::Arc;

use async_lock::RwLock;
use async_trait::async_trait;
use hotshot_task_impls::events::{HotShotEvent, HotShotEvent::*};
use hotshot_types::{
    data::null_block,
    traits::{
        block_contents::BlockHeader,
        node_implementation::{NodeType, Versions},
    },
};

use crate::predicates::{Predicate, PredicateResult};

type EventCallback<TYPES> = Arc<dyn Fn(Arc<HotShotEvent<TYPES>>) -> bool + Send + Sync>;

pub struct EventPredicate<TYPES>
where
    TYPES: NodeType + Send + Sync,
{
    check: EventCallback<TYPES>,
    info: String,
}

impl<TYPES: NodeType> std::fmt::Debug for EventPredicate<TYPES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.info)
    }
}

#[allow(clippy::type_complexity)]
pub struct TestPredicate<INPUT> {
    pub function: Arc<RwLock<dyn FnMut(&INPUT) -> PredicateResult + Send + Sync>>,
    pub info: String,
}

impl<INPUT> std::fmt::Debug for TestPredicate<INPUT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.info)
    }
}

#[async_trait]
impl<INPUT> Predicate<INPUT> for TestPredicate<INPUT>
where
    INPUT: Send + Sync,
{
    async fn evaluate(&self, input: &INPUT) -> PredicateResult {
        let mut function = self.function.write().await;
        function(input)
    }

    async fn info(&self) -> String {
        self.info.clone()
    }
}

pub fn all<TYPES>(events: Vec<HotShotEvent<TYPES>>) -> Box<TestPredicate<Arc<HotShotEvent<TYPES>>>>
where
    TYPES: NodeType,
{
    all_predicates(events.into_iter().map(exact).collect())
}

pub fn all_predicates<TYPES: NodeType>(
    predicates: Vec<Box<EventPredicate<TYPES>>>,
) -> Box<TestPredicate<Arc<HotShotEvent<TYPES>>>> {
    let info = format!("{:?}", predicates);

    let mut unsatisfied: Vec<_> = predicates.into_iter().map(Arc::new).collect();

    let function = move |e: &Arc<HotShotEvent<TYPES>>| {
        if !unsatisfied
            .clone()
            .into_iter()
            .any(|pred| (pred.check)(e.clone()))
        {
            return PredicateResult::Fail;
        }

        unsatisfied.retain(|pred| !(pred.check)(e.clone()));

        if unsatisfied.is_empty() {
            PredicateResult::Pass
        } else {
            PredicateResult::Incomplete
        }
    };

    Box::new(TestPredicate {
        function: Arc::new(RwLock::new(function)),
        info,
    })
}

#[macro_export]
macro_rules! all_predicates {
    ($($x:expr),* $(,)?) => {
        {
            vec![all_predicates(vec![$($x),*])]
        }
    };
}

#[async_trait]
impl<TYPES> Predicate<Arc<HotShotEvent<TYPES>>> for EventPredicate<TYPES>
where
    TYPES: NodeType + Send + Sync + 'static,
{
    async fn evaluate(&self, input: &Arc<HotShotEvent<TYPES>>) -> PredicateResult {
        PredicateResult::from((self.check)(input.clone()))
    }

    async fn info(&self) -> String {
        self.info.clone()
    }
}

pub fn exact<TYPES>(event: HotShotEvent<TYPES>) -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = format!("{:?}", event);
    let event = Arc::new(event);

    let check: EventCallback<TYPES> = Arc::new(move |e: Arc<HotShotEvent<TYPES>>| {
        let event_clone = event.clone();
        *e == *event_clone
    });

    Box::new(EventPredicate { check, info })
}

pub fn quorum_vote_send<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "QuorumVoteSend".to_string();
    let check: EventCallback<TYPES> =
        Arc::new(move |e: Arc<HotShotEvent<TYPES>>| matches!(e.as_ref(), QuorumVoteSend(_)));

    Box::new(EventPredicate { check, info })
}

pub fn view_change<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "ViewChange".to_string();
    let check: EventCallback<TYPES> =
        Arc::new(move |e: Arc<HotShotEvent<TYPES>>| matches!(e.as_ref(), ViewChange(_, _)));
    Box::new(EventPredicate { check, info })
}

pub fn upgrade_certificate_formed<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "UpgradeCertificateFormed".to_string();
    let check: EventCallback<TYPES> = Arc::new(move |e: Arc<HotShotEvent<TYPES>>| {
        matches!(e.as_ref(), UpgradeCertificateFormed(_))
    });
    Box::new(EventPredicate { check, info })
}

pub fn quorum_proposal_send_with_upgrade_certificate<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "QuorumProposalSend with UpgradeCertificate attached".to_string();
    let check: EventCallback<TYPES> =
        Arc::new(move |e: Arc<HotShotEvent<TYPES>>| match e.as_ref() {
            QuorumProposalSend(proposal, _) => proposal.data.upgrade_certificate().is_some(),
            _ => false,
        });
    Box::new(EventPredicate { info, check })
}

pub fn quorum_proposal_validated<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "QuorumProposalValidated".to_string();
    let check: EventCallback<TYPES> = Arc::new(move |e: Arc<HotShotEvent<TYPES>>| {
        matches!(*e.clone(), QuorumProposalValidated(..))
    });
    Box::new(EventPredicate { check, info })
}

pub fn quorum_proposal_send<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "QuorumProposalSend".to_string();
    let check: EventCallback<TYPES> =
        Arc::new(move |e: Arc<HotShotEvent<TYPES>>| matches!(e.as_ref(), QuorumProposalSend(..)));
    Box::new(EventPredicate { check, info })
}

pub fn quorum_proposal_send_with_null_block<TYPES, V>(
    num_storage_nodes: usize,
) -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
    V: Versions,
{
    let info = "QuorumProposalSend with null block payload".to_string();
    let check: EventCallback<TYPES> =
        Arc::new(move |e: Arc<HotShotEvent<TYPES>>| match e.as_ref() {
            QuorumProposalSend(proposal, _) => {
                Some(proposal.data.block_header().payload_commitment())
                    == null_block::commitment::<V>(num_storage_nodes)
            },
            _ => false,
        });
    Box::new(EventPredicate { check, info })
}

pub fn timeout_vote_send<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "TimeoutVoteSend".to_string();
    let check: EventCallback<TYPES> =
        Arc::new(move |e: Arc<HotShotEvent<TYPES>>| matches!(e.as_ref(), TimeoutVoteSend(..)));
    Box::new(EventPredicate { check, info })
}

pub fn view_sync_timeout<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "ViewSyncTimeout".to_string();
    let check: EventCallback<TYPES> =
        Arc::new(move |e: Arc<HotShotEvent<TYPES>>| matches!(e.as_ref(), ViewSyncTimeout(..)));
    Box::new(EventPredicate { check, info })
}

pub fn view_sync_precommit_vote_send<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "ViewSyncPreCommitVoteSend".to_string();
    let check: EventCallback<TYPES> = Arc::new(move |e: Arc<HotShotEvent<TYPES>>| {
        matches!(e.as_ref(), ViewSyncPreCommitVoteSend(..))
    });
    Box::new(EventPredicate { check, info })
}

pub fn vid_share_validated<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "VidShareValidated".to_string();
    let check: EventCallback<TYPES> =
        Arc::new(move |e: Arc<HotShotEvent<TYPES>>| matches!(e.as_ref(), VidShareValidated(..)));
    Box::new(EventPredicate { check, info })
}

pub fn da_certificate_validated<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "DaCertificateValidated".to_string();
    let check: EventCallback<TYPES> = Arc::new(move |e: Arc<HotShotEvent<TYPES>>| {
        matches!(e.as_ref(), DaCertificateValidated(..))
    });
    Box::new(EventPredicate { check, info })
}

pub fn quorum_proposal_preliminarily_validated<TYPES>() -> Box<EventPredicate<TYPES>>
where
    TYPES: NodeType,
{
    let info = "QuorumProposalPreliminarilyValidated".to_string();
    let check: EventCallback<TYPES> = Arc::new(move |e: Arc<HotShotEvent<TYPES>>| {
        matches!(e.as_ref(), QuorumProposalPreliminarilyValidated(..))
    });
    Box::new(EventPredicate { check, info })
}
