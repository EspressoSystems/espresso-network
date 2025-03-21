// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use futures::{channel::mpsc, StreamExt};
use tokio::{spawn, sync::mpsc::UnboundedSender, time::timeout};

use crate::network::ClientRequest;

/// Internal bootstrap events
pub enum InputEvent {
    /// Start bootstrap
    StartBootstrap,
    /// Bootstrap has finished
    BootstrapFinished,
    /// Shutdown bootstrap
    ShutdownBootstrap,
}
/// Bootstrap task's state
pub struct DHTBootstrapTask {
    /// Task's receiver
    rx: mpsc::Receiver<InputEvent>,
    /// Task's sender
    network_tx: UnboundedSender<ClientRequest>,
    /// Field indicating progress state
    in_progress: bool,
}

impl DHTBootstrapTask {
    /// Run bootstrap task
    pub fn run(rx: mpsc::Receiver<InputEvent>, tx: UnboundedSender<ClientRequest>) {
        spawn(async move {
            let state = Self {
                rx,
                network_tx: tx,
                in_progress: false,
            };
            state.run_loop().await;
        });
    }
    /// Task's loop
    async fn run_loop(mut self) {
        loop {
            if self.in_progress {
                match self.rx.next().await {
                    Some(InputEvent::BootstrapFinished) => {
                        tracing::debug!("Bootstrap finished");
                        self.in_progress = false;
                    },
                    Some(InputEvent::ShutdownBootstrap) => {
                        tracing::info!("ShutdownBootstrap received, shutting down");
                        break;
                    },
                    Some(InputEvent::StartBootstrap) => {
                        tracing::warn!("Trying to start bootstrap that's already in progress");
                        continue;
                    },
                    None => {
                        tracing::debug!("Bootstrap channel closed, exiting loop");
                        break;
                    },
                }
            } else if let Ok(maybe_event) = timeout(Duration::from_secs(120), self.rx.next()).await
            {
                match maybe_event {
                    Some(InputEvent::StartBootstrap) => {
                        tracing::debug!("Start bootstrap in bootstrap task");
                        self.bootstrap();
                    },
                    Some(InputEvent::ShutdownBootstrap) => {
                        tracing::debug!("ShutdownBootstrap received, shutting down");
                        break;
                    },
                    Some(InputEvent::BootstrapFinished) => {
                        tracing::debug!("not in progress got bootstrap finished");
                    },
                    None => {
                        tracing::debug!("Bootstrap channel closed, exiting loop");
                        break;
                    },
                }
            } else {
                tracing::debug!("Start bootstrap in bootstrap task after timeout");
                self.bootstrap();
            }
        }
    }

    /// Start bootstrap
    fn bootstrap(&mut self) {
        self.in_progress = true;
        let _ = self.network_tx.send(ClientRequest::BeginBootstrap);
    }
}
