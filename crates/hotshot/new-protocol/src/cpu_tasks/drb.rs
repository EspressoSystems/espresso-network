use std::collections::BTreeMap;

use hotshot_types::{
    data::EpochNumber,
    drb::{DrbInput, DrbResult, compute_drb_result},
    traits::{
        node_implementation::NodeType,
        storage::{LoadDrbProgressFn, StoreDrbProgressFn},
    },
};
use tokio::{
    spawn,
    sync::mpsc::{self},
    task::JoinHandle,
};

use crate::coordinator::handle::CoordinatorHandle;

pub(super) struct DrbRequestTask<TYPES: NodeType> {
    calculations: BTreeMap<EpochNumber, JoinHandle<()>>,
    rx: mpsc::Receiver<DrbInput>,
    coordinator_handle: CoordinatorHandle<TYPES>,
    store_drb_progress: StoreDrbProgressFn,
    load_drb_progress: LoadDrbProgressFn,
    internal_tx: mpsc::Sender<(EpochNumber, DrbResult)>,
    internal_rx: mpsc::Receiver<(EpochNumber, DrbResult)>,
}

impl<TYPES: NodeType> DrbRequestTask<TYPES> {
    pub fn new(
        rx: mpsc::Receiver<DrbInput>,
        coordinator_handle: CoordinatorHandle<TYPES>,
        store_drb_progress: StoreDrbProgressFn,
        load_drb_progress: LoadDrbProgressFn,
    ) -> Self {
        let (internal_tx, internal_rx) = mpsc::channel(100);
        Self {
            calculations: BTreeMap::new(),
            rx,
            coordinator_handle,
            store_drb_progress,
            load_drb_progress,
            internal_tx,
            internal_rx,
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(drb_input) = self.rx.recv() => {
                    let epoch = EpochNumber::new(drb_input.epoch);
                    if self.calculations.contains_key(&epoch) {
                        continue;
                    }
                    let handle = self.handle_drb_request(drb_input);
                    self.calculations.insert(epoch, handle);
                },
                Some((epoch, result)) = self.internal_rx.recv() => {
                    self.calculations.remove(&epoch);
                    let _ = self.coordinator_handle.respond_drb(result).await;
                },
                else => break,
            }
        }
    }

    fn handle_drb_request(&self, drb_input: DrbInput) -> JoinHandle<()> {
        let tx = self.internal_tx.clone();
        let store_drb_progress = self.store_drb_progress.clone();
        let load_drb_progress = self.load_drb_progress.clone();
        let epoch = EpochNumber::new(drb_input.epoch);
        spawn(async move {
            let result = compute_drb_result(drb_input, store_drb_progress, load_drb_progress).await;
            let _ = tx.send((epoch, result)).await;
        })
    }
}
