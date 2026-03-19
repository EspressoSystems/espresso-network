use anyhow::Result; // TODO: proper error type
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::traits::{
    consensus_api::ConsensusApi, network::ConnectedNetwork, node_implementation::NodeType,
};

use crate::{
    consensus::Consensus,
    events::{Action, ConsensusInput, ConsensusOutput},
    helpers::Outbox,
    io::Network,
    message::{ConsensusMessage, MessageType},
};

pub(crate) mod mock;

pub struct Coordinator<T: NodeType, N> {
    consensus: Consensus<T>,
    outbox: Outbox<ConsensusOutput<T>>,
    network: Network<T, N>,
    external: async_broadcast::Sender<hotshot_types::event::Event<T>>,
}

impl<T, N> Coordinator<T, N>
where
    T: NodeType,
    N: ConnectedNetwork<T::SignatureKey>,
{
    pub fn new<I: NodeImplementation<T, Network = N>>(
        ctx: SystemContextHandle<T, I>,
        ext: async_broadcast::Sender<hotshot_types::event::Event<T>>,
    ) -> Self {
        Self {
            consensus: Consensus::new(
                ctx.membership_coordinator.clone(),
                ctx.public_key(),
                ctx.private_key().clone(),
            ),
            outbox: Outbox::new(),
            network: Network::new(ctx.network.clone(), ctx.hotshot.upgrade_lock.clone()),
            external: ext,
        }
    }

    pub fn outputs(&self) -> &Outbox<ConsensusOutput<T>> {
        &self.outbox
    }

    pub async fn next_step(&mut self) -> Result<()> {
        self.execute().await?;
        let msg = self.network.receive().await?;
        match msg.message_type {
            MessageType::Consensus(ConsensusMessage::Proposal(p)) => {
                self.consensus
                    .apply(ConsensusInput::Proposal(p), &mut self.outbox)
                    .await;
            },
            _ => todo!(),
        }
        Ok(())
    }

    async fn execute(&mut self) -> Result<()> {
        while let Some(output) = self.outbox.pop_front() {
            match output {
                ConsensusOutput::Event(e) => {
                    todo!()
                },
                ConsensusOutput::Action(a) => match a {
                    Action::SendProposal(p, vid) => {},
                    _ => todo!(),
                },
            }
        }
        Ok(())
    }
}
