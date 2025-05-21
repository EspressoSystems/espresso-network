// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::HashSet, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use hotshot_task_impls::events::HotShotEvent;
use hotshot_types::traits::node_implementation::{NodeType, TestableNodeImplementation};
use hotshot_utils::anytrace::*;
use thiserror::Error;

use crate::test_task::{TestResult, TestTaskState};

/// `ViewSync` Task error
#[derive(Error, Debug, Clone)]
pub enum ViewSyncTaskError {
    #[error("{} nodes hit view sync", hit_view_sync.len())]
    HitViewSync { hit_view_sync: HashSet<usize> },
}

/// `ViewSync` task state
pub struct ViewSyncTask<TYPES: NodeType, I: TestableNodeImplementation<TYPES>> {
    /// nodes that hit view sync
    pub(crate) hit_view_sync: HashSet<usize>,
    /// properties of task
    pub(crate) description: ViewSyncTaskDescription,
    /// Phantom data for TYPES and I
    pub(crate) _pd: PhantomData<(TYPES, I)>,
}

#[async_trait]
impl<TYPES: NodeType, I: TestableNodeImplementation<TYPES>> TestTaskState
    for ViewSyncTask<TYPES, I>
{
    type Event = Arc<HotShotEvent<TYPES>>;
    type Error = Error;

    /// Handles an event from one of multiple receivers.
    async fn handle_event(&mut self, (event, id): (Self::Event, usize)) -> Result<()> {
        match event.as_ref() {
            // all the view sync events
            HotShotEvent::ViewSyncTimeout(..)
            | HotShotEvent::ViewSyncPreCommitVoteRecv(_)
            | HotShotEvent::ViewSyncCommitVoteRecv(_)
            | HotShotEvent::ViewSyncFinalizeVoteRecv(_)
            | HotShotEvent::ViewSyncPreCommitVoteSend(_)
            | HotShotEvent::ViewSyncCommitVoteSend(_)
            | HotShotEvent::ViewSyncFinalizeVoteSend(_)
            | HotShotEvent::ViewSyncPreCommitCertificateRecv(_)
            | HotShotEvent::ViewSyncCommitCertificateRecv(_)
            | HotShotEvent::ViewSyncFinalizeCertificateRecv(_)
            | HotShotEvent::ViewSyncPreCommitCertificateSend(..)
            | HotShotEvent::ViewSyncCommitCertificateSend(..)
            | HotShotEvent::ViewSyncFinalizeCertificateSend(..)
            | HotShotEvent::ViewSyncTrigger(_) => {
                self.hit_view_sync.insert(id);
            },
            _ => (),
        }

        Ok(())
    }

    async fn check(&self) -> TestResult {
        match self.description.clone() {
            ViewSyncTaskDescription::Threshold(min, max) => {
                let num_hits = self.hit_view_sync.len();
                if min <= num_hits && num_hits <= max {
                    TestResult::Pass
                } else {
                    TestResult::Fail(Box::new(ViewSyncTaskError::HitViewSync {
                        hit_view_sync: self.hit_view_sync.clone(),
                    }))
                }
            },
        }
    }
}

/// enum desecribing whether a node should hit view sync
#[derive(Clone, Debug, Copy)]
pub enum ShouldHitViewSync {
    /// the node should hit view sync
    Yes,
    /// the node should not hit view sync
    No,
    /// don't care if the node should hit view sync
    Ignore,
}

/// Description for a view sync task.
#[derive(Clone, Debug)]
pub enum ViewSyncTaskDescription {
    /// (min, max) number nodes that may hit view sync, inclusive
    Threshold(usize, usize),
}
