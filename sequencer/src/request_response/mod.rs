use std::future::Future;

use data_source::DataSource;
use derive_more::derive::Deref;
use espresso_types::{traits::SequencerPersistence, PubKey, SeqTypes};
use hotshot::{traits::NodeImplementation, types::BLSPrivKey};
use hotshot_types::traits::{network::ConnectedNetwork, node_implementation::Versions};
use network::Sender;
use recipient_source::RecipientSource;
use request::{Request, Response};
use request_response::{
    network::Bytes, RequestError, RequestResponse, RequestResponseConfig, RequestType,
};
use tokio::sync::mpsc::Receiver;

pub mod catchup;
pub mod data_source;
pub mod network;
pub mod recipient_source;
pub mod request;

/// A concrete type wrapper around `RequestResponse`. We need this so that we can implement
/// local traits like `StateCatchup`. It also helps with readability.
#[derive(Clone, Deref)]
pub struct RequestResponseProtocol<
    I: NodeImplementation<SeqTypes>,
    V: Versions,
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
> {
    #[deref]
    #[allow(clippy::type_complexity)]
    /// The actual inner request response protocol
    inner: RequestResponse<
        Sender,
        Receiver<Bytes>,
        Request,
        RecipientSource<I, V>,
        DataSource<I, V, N, P>,
        PubKey,
    >,

    /// The configuration we used for the above inner protocol. This is nice to have for
    /// estimating when we should make another request
    config: RequestResponseConfig,

    /// The public key of this node
    public_key: PubKey,
    /// The private key of this node
    private_key: BLSPrivKey,
}

impl<
        I: NodeImplementation<SeqTypes>,
        V: Versions,
        N: ConnectedNetwork<PubKey>,
        P: SequencerPersistence,
    > RequestResponseProtocol<I, V, N, P>
{
    /// Create a new RequestResponseProtocol from the inner
    pub fn new(
        // The configuration for the protocol
        config: RequestResponseConfig,
        // The network sender that [`RequestResponseProtocol`] will use to send messages
        sender: Sender,
        // The network receiver that [`RequestResponseProtocol`] will use to receive messages
        receiver: Receiver<Bytes>,
        // The recipient source that [`RequestResponseProtocol`] will use to get the recipients
        // that a specific message should expect responses from
        recipient_source: RecipientSource<I, V>,
        // The [response] data source that [`RequestResponseProtocol`] will use to derive the
        // response data for a specific request
        data_source: DataSource<I, V, N, P>,
        // The public key of this node
        public_key: PubKey,
        // The private key of this node
        private_key: BLSPrivKey,
    ) -> Self {
        Self {
            inner: RequestResponse::new(
                config.clone(),
                sender,
                receiver,
                recipient_source,
                data_source,
            ),
            config,
            public_key,
            private_key,
        }
    }
}

impl<
        I: NodeImplementation<SeqTypes>,
        V: Versions,
        N: ConnectedNetwork<PubKey>,
        P: SequencerPersistence,
    > RequestResponseProtocol<I, V, N, P>
{
    pub async fn request_indefinitely<F, Fut, O>(
        &self,
        // The request to make
        request: Request,
        // The type of request
        request_type: RequestType,
        // The response validation function
        response_validation_fn: F,
    ) -> std::result::Result<O, RequestError>
    where
        F: Fn(&Request, Response) -> Fut + Send + Sync + 'static + Clone,
        Fut: Future<Output = anyhow::Result<O>> + Send + Sync + 'static,
        O: Send + Sync + 'static + Clone,
    {
        // Request from the inner protocol
        self.inner
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                request_type,
                self.config.incoming_request_ttl,
                request,
                response_validation_fn,
            )
            .await
    }
}
