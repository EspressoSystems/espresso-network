use std::collections::{BTreeMap, BTreeSet};

use hotshot_types::{
    data::EpochNumber,
    drb::{DrbInput, DrbResult, compute_drb_result},
    traits::storage::{LoadDrbProgressFn, StoreDrbProgressFn},
};
use tokio::task::{AbortHandle, JoinSet};
use tracing::warn;

pub struct DrbRequester {
    calculations: BTreeMap<EpochNumber, AbortHandle>,
    completed_epochs: BTreeSet<EpochNumber>,
    store_drb_progress: StoreDrbProgressFn,
    load_drb_progress: LoadDrbProgressFn,
    tasks: JoinSet<(EpochNumber, DrbResult)>,
}

impl DrbRequester {
    pub fn new(
        store_drb_progress: StoreDrbProgressFn,
        load_drb_progress: LoadDrbProgressFn,
    ) -> Self {
        Self {
            calculations: BTreeMap::new(),
            completed_epochs: BTreeSet::new(),
            store_drb_progress,
            load_drb_progress,
            tasks: JoinSet::new(),
        }
    }

    pub async fn next(&mut self) -> Option<(EpochNumber, DrbResult)> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok((epoch, result))) => {
                    self.calculations.remove(&epoch);
                    self.completed_epochs.insert(epoch);
                    return Some((epoch, result));
                },
                Some(Err(e)) => {
                    warn!("Error in drb request task: {e}");
                    continue;
                },
                None => return None,
            }
        }
    }

    pub fn request_drb(&mut self, drb_input: DrbInput) {
        let store_drb_progress = self.store_drb_progress.clone();
        let load_drb_progress = self.load_drb_progress.clone();
        let epoch = EpochNumber::new(drb_input.epoch);
        let handle = self.tasks.spawn(async move {
            let result = compute_drb_result(drb_input, store_drb_progress, load_drb_progress).await;
            (epoch, result)
        });
        self.calculations.insert(epoch, handle);
    }

    pub fn gc(&mut self, epoch: EpochNumber) {
        let keep = self.calculations.split_off(&epoch);
        self.completed_epochs = self.completed_epochs.split_off(&epoch);
        for (epoch, handle) in self.calculations.iter_mut() {
            handle.abort();
        }
        self.calculations = keep;
    }
}
