// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{fs::OpenOptions, io::Write, sync::Arc};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::traits::node_implementation::NodeType;
use hotshot_utils::anytrace::Result;

use crate::events::HotShotEvent;

/// The task state for the `Rewind` task is used to capture all events received
/// by a particular node, in the order they've been received.
pub struct RewindTaskState<TYPES: NodeType> {
    /// All events received by this node since the beginning of time.
    pub events: Vec<Arc<HotShotEvent<TYPES>>>,

    /// The id of this node
    pub id: u64,
}

impl<TYPES: NodeType> RewindTaskState<TYPES> {
    /// Handles all events, storing them to the private state
    pub fn handle(&mut self, event: &Arc<HotShotEvent<TYPES>>) {
        self.events.push(Arc::clone(event));
    }
}

#[async_trait]
impl<TYPES: NodeType> TaskState for RewindTaskState<TYPES> {
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        _sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(&event);
        Ok(())
    }

    fn cancel_subtasks(&mut self) {
        tracing::info!("Node ID {} Recording {} events", self.id, self.events.len());
        let filename = format!("rewind_{}.log", self.id);
        let mut file = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&filename)
        {
            Ok(file) => file,
            Err(e) => {
                tracing::error!("Failed to write file {filename}; error = {e}");
                return;
            },
        };

        for (event_number, event) in self.events.iter().enumerate() {
            // We do not want to die here, so we log and move on capturing as many events as we can.
            if let Err(e) = writeln!(file, "{event_number}: {event}") {
                tracing::error!(
                    "Failed to write event number {event_number} and event {event}; error = {e}"
                );
            }
        }
    }
}
