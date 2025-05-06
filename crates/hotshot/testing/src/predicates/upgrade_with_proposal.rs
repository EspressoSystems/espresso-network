// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::sync::Arc;

use async_trait::async_trait;
use hotshot_example_types::node_types::{MemoryImpl, TestTypes, TestVersions};
use hotshot_task_impls::quorum_proposal::QuorumProposalTaskState;
use hotshot_types::simple_certificate::UpgradeCertificate;

use crate::predicates::{Predicate, PredicateResult};

type QuorumProposalTaskTestState = QuorumProposalTaskState<TestTypes, MemoryImpl, TestVersions>;

type UpgradeCertCallback =
    Arc<dyn Fn(Arc<Option<UpgradeCertificate<TestTypes>>>) -> bool + Send + Sync>;

pub struct UpgradeCertPredicate {
    check: UpgradeCertCallback,
    info: String,
}

impl std::fmt::Debug for UpgradeCertPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.info)
    }
}

#[async_trait]
impl Predicate<QuorumProposalTaskTestState> for UpgradeCertPredicate {
    async fn evaluate(&self, input: &QuorumProposalTaskTestState) -> PredicateResult {
        let upgrade_cert = input
            .upgrade_lock
            .decided_upgrade_certificate
            .read()
            .await
            .clone();
        PredicateResult::from((self.check)(upgrade_cert.into()))
    }

    async fn info(&self) -> String {
        self.info.clone()
    }
}

pub fn no_decided_upgrade_certificate() -> Box<UpgradeCertPredicate> {
    let info = "expected decided_upgrade_certificate to be None".to_string();
    let check: UpgradeCertCallback = Arc::new(move |s| s.is_none());
    Box::new(UpgradeCertPredicate { info, check })
}

pub fn decided_upgrade_certificate() -> Box<UpgradeCertPredicate> {
    let info = "expected decided_upgrade_certificate to be Some(_)".to_string();
    let check: UpgradeCertCallback = Arc::new(move |s| s.is_some());
    Box::new(UpgradeCertPredicate { info, check })
}
