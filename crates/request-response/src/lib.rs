//! This crate contains a general request-response protocol. It is used to send requests to
//! a set of recipients and wait for responses.

use std::{
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use data_source::DataSource;
use derive_more::derive::Deref;
use hotshot_types::traits::signature_key::SignatureKey;
use message::{Message, RequestMessage, ResponseMessage};
use network::{Bytes, Receiver, Sender};
use parking_lot::RwLock;
use rand::seq::SliceRandom;
use recipient_source::RecipientSource;
use request::Request;
use tokio::{
    spawn,
    sync::mpsc,
    time::{sleep, timeout},
};
use tokio_util::task::AbortOnDropHandle;
use tracing::{debug, error, info, trace, warn};
use util::{BoundedVecDeque, NamedSemaphore, NamedSemaphoreError};

/// The data source trait. Is what we use to derive the response data for a request
pub mod data_source;
/// The message type. Is the base type for all messages in the request-response protocol
pub mod message;
/// The network traits. Is what we use to send and receive messages over the network as
/// the protocol
pub mod network;
/// The recipient source trait. Is what we use to get the recipients that a specific message should
/// expect responses from
pub mod recipient_source;
/// The request trait. Is what we use to define a request and a corresponding response type
pub mod request;
/// Utility types and functions
mod util;

/// A type alias for the hash of a request
pub type RequestHash = blake3::Hash;

/// The map of active outgoing requests: request hash → the waiters registered by concurrent
/// [`RequestResponseInner::request`] calls for the same data. Guarded by a synchronous lock so
/// waiters can deregister themselves in `Drop`; it is never held across an `await`
type ActiveRequestsMap<Req> = Arc<RwLock<HashMap<RequestHash, Vec<Waiter<Req>>>>>;

/// A type alias for the list of tasks that are responding to requests
pub type IncomingRequests<K> = NamedSemaphore<K>;

/// The number of responses that can be buffered for each waiter before drops occur. Must cover
/// the maximum number of simultaneous responders: a broadcast request can be answered by every
/// node at once while its single waiter validates responses serially
const RESPONSE_BUFFER_SIZE: usize = 128;

/// The type of request to make
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum RequestType {
    /// A request that can be satisfied by a single participant,
    /// and as such will be batched to a few participants at a time
    /// until one succeeds
    Batched,
    /// A request that needs most or all participants to respond,
    /// and as such will be broadcasted to all participants
    Broadcast,
}

/// The errors that can occur when making a request for data
#[derive(thiserror::Error, Debug)]
pub enum RequestError {
    /// The request timed out
    #[error("request timed out")]
    Timeout,
    /// The request was invalid
    #[error("request was invalid")]
    InvalidRequest(anyhow::Error),
    /// Other errors
    #[error("other error")]
    Other(anyhow::Error),
}

/// A trait for serializing and deserializing a type to and from a byte array. [`Request`] types and
/// [`Response`] types will need to implement this trait
pub trait Serializable: Sized {
    /// Serialize the type to a byte array. If this is for a [`Request`] and your [`Request`] type
    /// is represented as an enum, please make sure that you serialize it with a unique type ID. Otherwise,
    /// you may end up with collisions as the request hash is used as a unique identifier
    ///
    /// # Errors
    /// - If the type cannot be serialized to a byte array
    fn to_bytes(&self) -> Result<Vec<u8>>;

    /// Deserialize the type from a byte array
    ///
    /// # Errors
    /// - If the byte array is not a valid representation of the type
    fn from_bytes(bytes: &[u8]) -> Result<Self>;
}

/// The underlying configuration for the request-response protocol
#[derive(Clone)]
pub struct RequestResponseConfig {
    /// The timeout for incoming requests. Do not respond to a request after this threshold
    /// has passed.
    pub incoming_request_ttl: Duration,
    /// The maximum amount of time we will spend trying to both derive a response for a request and
    /// send the response over the wire.
    pub incoming_request_timeout: Duration,
    /// The batch size for outgoing requests. This is the number of request messages that we will
    /// send out at a time for a single request before waiting for the [`request_batch_interval`].
    pub request_batch_size: usize,
    /// The time to wait (per request) between sending out batches of request messages
    pub request_batch_interval: Duration,
    /// The maximum (global) number of incoming requests that can be processed at any given time.
    pub max_incoming_requests: usize,
    /// The maximum number of incoming requests that can be processed for a single key at any given time.
    pub max_incoming_requests_per_key: usize,
}

/// A protocol that allows for request-response communication. Is cheaply cloneable, so there is no
/// need to wrap it in an `Arc`
#[derive(Deref)]
pub struct RequestResponse<
    S: Sender<K>,
    R: Receiver,
    Req: Request,
    RS: RecipientSource<Req, K>,
    DS: DataSource<Req>,
    K: SignatureKey + 'static,
> {
    #[deref]
    /// The inner implementation of the request-response protocol
    pub inner: Arc<RequestResponseInner<S, R, Req, RS, DS, K>>,
    /// A handle to the receiving task. This will automatically get cancelled when the protocol is dropped
    _receiving_task_handle: Arc<AbortOnDropHandle<()>>,
}

/// We need to manually implement the `Clone` trait for this type because deriving
/// `Deref` will cause an issue where it tries to clone the inner field instead
impl<
    S: Sender<K>,
    R: Receiver,
    Req: Request,
    RS: RecipientSource<Req, K>,
    DS: DataSource<Req>,
    K: SignatureKey + 'static,
> Clone for RequestResponse<S, R, Req, RS, DS, K>
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            _receiving_task_handle: Arc::clone(&self._receiving_task_handle),
        }
    }
}

impl<
    S: Sender<K>,
    R: Receiver,
    Req: Request,
    RS: RecipientSource<Req, K>,
    DS: DataSource<Req>,
    K: SignatureKey + 'static,
> RequestResponse<S, R, Req, RS, DS, K>
{
    /// Create a new [`RequestResponseProtocol`]
    pub fn new(
        // The configuration for the protocol
        config: RequestResponseConfig,
        // The network sender that [`RequestResponseProtocol`] will use to send messages
        sender: S,
        // The network receiver that [`RequestResponseProtocol`] will use to receive messages
        receiver: R,
        // The recipient source that [`RequestResponseProtocol`] will use to get the recipients
        // that a specific message should expect responses from
        recipient_source: RS,
        // The [response] data source that [`RequestResponseProtocol`] will use to derive the
        // response data for a specific request
        data_source: DS,
    ) -> Self {
        // Create the inner implementation
        let inner = Arc::new(RequestResponseInner {
            config,
            sender,
            recipient_source,
            data_source,
            active_requests: ActiveRequestsMap::default(),
            next_waiter_id: AtomicU64::new(0),
            phantom_data: PhantomData,
        });

        // Start the task that receives messages and handles them. This will automatically get cancelled
        // when the protocol is dropped
        let inner_clone = Arc::clone(&inner);
        let receive_task_handle =
            AbortOnDropHandle::new(tokio::spawn(inner_clone.receiving_task(receiver)));

        // Return the protocol
        Self {
            inner,
            _receiving_task_handle: Arc::new(receive_task_handle),
        }
    }
}

/// The inner implementation for the request-response protocol
pub struct RequestResponseInner<
    S: Sender<K>,
    R: Receiver,
    Req: Request,
    RS: RecipientSource<Req, K>,
    DS: DataSource<Req>,
    K: SignatureKey + 'static,
> {
    /// The configuration of the protocol
    config: RequestResponseConfig,
    /// The sender to use for the protocol
    pub sender: S,
    /// The recipient source to use for the protocol
    pub recipient_source: RS,
    /// The data source to use for the protocol
    data_source: DS,
    /// The map of currently active, outgoing requests
    active_requests: ActiveRequestsMap<Req>,
    /// The id to assign to the next registered waiter
    next_waiter_id: AtomicU64,
    /// Phantom data to help with type inference
    phantom_data: PhantomData<(K, R, Req, DS)>,
}
impl<
    S: Sender<K>,
    R: Receiver,
    Req: Request,
    RS: RecipientSource<Req, K>,
    DS: DataSource<Req>,
    K: SignatureKey + 'static,
> RequestResponseInner<S, R, Req, RS, DS, K>
{
    /// Request something from the protocol indefinitely until we get a response
    /// or there was a critical error (e.g. the request could not be signed)
    ///
    /// # Errors
    /// - If the request was invalid
    /// - If there was a critical error (e.g. the channel was closed)
    pub async fn request_indefinitely<F, Fut, O>(
        self: &Arc<Self>,
        public_key: &K,
        private_key: &K::PrivateKey,
        // The type of request to make
        request_type: RequestType,
        // The estimated TTL of other participants. This is used to decide when to
        // stop making requests and sign a new one
        estimated_request_ttl: Duration,
        // The request to make
        request: Req,
        // The response validation function
        response_validation_fn: F,
    ) -> std::result::Result<O, RequestError>
    where
        F: Fn(&Req, Req::Response) -> Fut + Send + Sync + Clone,
        Fut: Future<Output = anyhow::Result<O>> + Send,
        O: Send,
    {
        loop {
            // Sign a request message
            let request_message = RequestMessage::new_signed(public_key, private_key, &request)
                .map_err(|e| {
                    RequestError::InvalidRequest(anyhow::anyhow!(
                        "failed to sign request message: {e}"
                    ))
                })?;

            // Request the data, handling the errors appropriately
            match self
                .request(
                    request_message,
                    request_type,
                    estimated_request_ttl,
                    response_validation_fn.clone(),
                )
                .await
            {
                Ok(response) => return Ok(response),
                Err(RequestError::Timeout) => continue,
                Err(e) => return Err(e),
            }
        }
    }

    /// Request something from the protocol and wait for the first response that passes
    /// validation. Concurrent requests for the same data (determined by `Blake3` hash of the
    /// request) share incoming responses, but each caller validates them independently and all
    /// of them make requests until their own timeout is reached.
    ///
    /// # Errors
    /// - If the request times out
    /// - If the request we sign is invalid
    pub async fn request<F, Fut, O>(
        self: &Arc<Self>,
        request_message: RequestMessage<Req, K>,
        request_type: RequestType,
        timeout_duration: Duration,
        response_validation_fn: F,
    ) -> std::result::Result<O, RequestError>
    where
        F: Fn(&Req, Req::Response) -> Fut + Send + Sync,
        Fut: Future<Output = anyhow::Result<O>> + Send,
        O: Send,
    {
        // The hash identifies the request on the wire and joins concurrent callers
        let request_hash = blake3::hash(&request_message.request.to_bytes().map_err(|e| {
            RequestError::InvalidRequest(anyhow::anyhow!(
                "failed to serialize request message: {e}"
            ))
        })?);

        // Register before sending so no response can be missed. Deregisters itself when
        // dropped, on success, timeout, and cancellation alike
        let mut response_receiver = self.register_waiter(request_hash);

        let message = Bytes::from(
            Message::Request(request_message.clone())
                .to_bytes()
                .map_err(|e| {
                    RequestError::InvalidRequest(anyhow::anyhow!(
                        "failed to serialize request message: {e}"
                    ))
                })?,
        );

        // Broadcast requests go out once; batched requests are re-sent by a background task
        // that is aborted when the handle drops with this function
        let _batched_sending_task = match request_type {
            RequestType::Broadcast => {
                trace!("Sending request {request_message:?} to all participants");

                self.sender
                    .send_broadcast_message(&message)
                    .await
                    .map_err(|e| {
                        RequestError::Other(anyhow::anyhow!(
                            "failed to send broadcast message: {e}"
                        ))
                    })?;

                None
            },
            RequestType::Batched => Some(
                self.spawn_batched_sender(request_message.clone(), message, timeout_duration)
                    .await?,
            ),
        };

        timeout(timeout_duration, async {
            loop {
                let response = response_receiver.recv().await.ok_or_else(|| {
                    // Unreachable: the active-requests map holds a sender for as long as
                    // `response_receiver` is registered, and it deregisters only on drop
                    RequestError::Other(anyhow!("response channel closed"))
                })?;

                // Clones only when this request is shared with another concurrent caller
                let response = Arc::unwrap_or_clone(response);

                match response_validation_fn(&request_message.request, response).await {
                    Ok(validated) => return Ok(validated),
                    Err(e) => debug!("Received invalid response: {e:#}"),
                }
            }
        })
        .await
        .map_err(|_| RequestError::Timeout)?
    }

    /// Register a waiter for responses to the request with the given hash
    fn register_waiter(self: &Arc<Self>, request_hash: RequestHash) -> ResponseReceiver<Req> {
        let (sender, receiver) = mpsc::channel(RESPONSE_BUFFER_SIZE);
        let id = self.next_waiter_id.fetch_add(1, Ordering::Relaxed);

        self.active_requests
            .write()
            .entry(request_hash)
            .or_default()
            .push(Waiter { id, sender });

        ResponseReceiver {
            request_hash,
            id,
            receiver,
            active_requests: Arc::clone(&self.active_requests),
        }
    }

    /// Spawn the task that repeatedly sends a batched request to
    /// [`config.request_batch_size`] recipients at a time until `timeout_duration` elapses
    /// or the returned handle is dropped
    async fn spawn_batched_sender(
        self: &Arc<Self>,
        request_message: RequestMessage<Req, K>,
        message: Bytes,
        timeout_duration: Duration,
    ) -> std::result::Result<AbortOnDropHandle<()>, RequestError> {
        // Shuffle so we don't always send to the same recipients in the same order
        let mut recipients = self
            .recipient_source
            .get_expected_responders(&request_message.request)
            .await
            .map_err(|e| {
                RequestError::InvalidRequest(anyhow::anyhow!(
                    "failed to get expected responders for request: {e}"
                ))
            })?;
        recipients.shuffle(&mut rand::thread_rng());

        let start_time = Instant::now();

        let self_clone = Arc::clone(self);
        Ok(AbortOnDropHandle::new(spawn(async move {
            // At most `request_batch_size` sends in flight at a time: pushing beyond the queue's
            // capacity evicts (and thereby aborts) the oldest send task
            let mut outgoing_requests = BoundedVecDeque::new(self_clone.config.request_batch_size);

            while start_time.elapsed() < timeout_duration {
                for recipient_batch in recipients.chunks(self_clone.config.request_batch_size) {
                    for recipient in recipient_batch {
                        let self_clone = Arc::clone(&self_clone);
                        let request_message_clone = request_message.clone();
                        let recipient_clone = recipient.clone();
                        let message_clone = Arc::clone(&message);

                        let individual_sending_task = spawn(async move {
                            trace!(
                                "Sending request {request_message_clone:?} to {recipient_clone:?}"
                            );

                            let _ = self_clone
                                .sender
                                .send_direct_message(&message_clone, recipient_clone)
                                .await;
                        });

                        outgoing_requests.push(AbortOnDropHandle::new(individual_sending_task));
                    }

                    sleep(self_clone.config.request_batch_interval).await;
                }
            }
        })))
    }

    /// The task responsible for receiving messages from the receiver and handling them
    async fn receiving_task(self: Arc<Self>, mut receiver: R) {
        // Upper bound the number of concurrently processed incoming requests
        let mut incoming_requests = NamedSemaphore::new(
            self.config.max_incoming_requests_per_key,
            Some(self.config.max_incoming_requests),
        );

        loop {
            match receiver.receive_message().await {
                Ok(message) => {
                    let message = match Message::from_bytes(&message) {
                        Ok(message) => message,
                        Err(e) => {
                            warn!("Received invalid message: {e:#}");
                            continue;
                        },
                    };

                    match message {
                        Message::Request(request_message) => {
                            self.handle_request(request_message, &mut incoming_requests);
                        },
                        Message::Response(response_message) => {
                            self.handle_response(response_message);
                        },
                    }
                },
                // An error here means the receiver will _NEVER_ receive any more messages
                Err(e) => {
                    error!("Request/response receive task exited: {e:#}");
                    return;
                },
            }
        }
    }

    /// Handle a request sent to us
    fn handle_request(
        self: &Arc<Self>,
        request_message: RequestMessage<Req, K>,
        incoming_requests: &mut IncomingRequests<K>,
    ) {
        trace!("Handling request {:?}", request_message);

        // Spawn a task to:
        // - Validate the request
        // - Derive the response data (check if we have it)
        // - Send the response to the requester
        let self_clone = Arc::clone(self);

        // Attempt to acquire a permit for the request. Warn if there are too many requests currently being processed
        // either globally or per-key
        let permit = incoming_requests.try_acquire(request_message.public_key.clone());
        match permit {
            Ok(ref permit) => permit,
            Err(NamedSemaphoreError::PerKeyLimitReached) => {
                info!(
                    "Failed to process request from {}: too many requests from the same key are \
                     already being processed",
                    request_message.public_key
                );
                return;
            },
            Err(NamedSemaphoreError::GlobalLimitReached) => {
                info!(
                    "Failed to process request from {}: too many requests are already being \
                     processed",
                    request_message.public_key
                );
                return;
            },
        };

        tokio::spawn(async move {
            let result = timeout(self_clone.config.incoming_request_timeout, async move {
                // Validate the request message. This includes:
                // - Checking the signature and making sure it's valid
                // - Checking the timestamp and making sure it's not too old
                // - Calling the request's application-specific validation function
                request_message
                    .validate(self_clone.config.incoming_request_ttl)
                    .with_context(|| "failed to validate request")?;

                // Try to fetch the response data from the data source
                let response = self_clone
                    .data_source
                    .derive_response_for(&request_message.request)
                    .await
                    .with_context(|| "failed to derive response for request")?;

                // Create the response message and serialize it
                let response = Bytes::from(
                    Message::Response::<Req, K>(ResponseMessage {
                        request_hash: blake3::hash(&request_message.request.to_bytes()?),
                        response,
                    })
                    .to_bytes()
                    .with_context(|| "failed to serialize response message")?,
                );

                // Send the response to the requester
                self_clone
                    .sender
                    .send_direct_message(&response, request_message.public_key)
                    .await
                    .with_context(|| "failed to send response to requester")?;

                // Drop the permit
                _ = permit;
                drop(permit);

                Ok::<(), anyhow::Error>(())
            })
            .await
            .map_err(|_| anyhow::anyhow!("timed out while sending response"))
            .and_then(|result| result);

            if let Err(e) = result {
                debug!("Failed to send response to requester: {e:#}");
            }
        });
    }

    /// Handle a response sent to us: fan it out to every waiter registered for the request hash
    fn handle_response(&self, response: ResponseMessage<Req>) {
        trace!("Handling response {response:?}");

        // Snapshot the waiting senders so the lock is not held while sending
        let waiters: Vec<mpsc::Sender<Arc<Req::Response>>> = {
            let active_requests = self.active_requests.read();
            let Some(waiters) = active_requests.get(&response.request_hash) else {
                // Not an error: a response for a request that was already satisfied or timed out
                trace!(
                    "Received response for inactive request {}",
                    response.request_hash
                );
                return;
            };
            waiters.iter().map(|waiter| waiter.sender.clone()).collect()
        };

        // `try_send` drops the response when a waiter's buffer is full: that waiter is
        // backlogged with earlier candidates, and batched senders keep re-requesting until
        // satisfied. A waiter dropped concurrently just yields a `Closed` error, also ignored
        let response = Arc::new(response.response);
        for waiter in waiters {
            let _ = waiter.try_send(Arc::clone(&response));
        }
    }
}

/// A waiter registered by one [`RequestResponseInner::request`] call, to which incoming
/// responses for its request hash are delivered
struct Waiter<Req: Request> {
    /// Identifies this waiter for deregistration when its receiver is dropped
    id: u64,
    /// Delivers candidate responses to the corresponding [`ResponseReceiver`]
    sender: mpsc::Sender<Arc<Req::Response>>,
}

/// Receives candidate responses for one [`RequestResponseInner::request`] call. Deregisters
/// the waiter when dropped, so map entries are cleaned up on success, timeout, and
/// cancellation alike
struct ResponseReceiver<Req: Request> {
    request_hash: RequestHash,
    id: u64,
    receiver: mpsc::Receiver<Arc<Req::Response>>,
    active_requests: ActiveRequestsMap<Req>,
}

impl<Req: Request> ResponseReceiver<Req> {
    async fn recv(&mut self) -> Option<Arc<Req::Response>> {
        self.receiver.recv().await
    }
}

impl<Req: Request> Drop for ResponseReceiver<Req> {
    fn drop(&mut self) {
        let mut active_requests = self.active_requests.write();
        if let Some(waiters) = active_requests.get_mut(&self.request_hash) {
            waiters.retain(|waiter| waiter.id != self.id);
            if waiters.is_empty() {
                active_requests.remove(&self.request_hash);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{
            Mutex,
            atomic::{AtomicBool, AtomicUsize},
        },
    };

    use async_trait::async_trait;
    use hotshot_types::signature_key::{BLSPrivKey, BLSPubKey};
    use rand::Rng;
    use tokio::{sync::mpsc, task::JoinSet};

    use super::*;

    /// A test sender that has a list of all the participants in the network
    #[derive(Clone)]
    pub struct TestSender {
        network: Arc<HashMap<BLSPubKey, mpsc::Sender<Bytes>>>,
    }

    /// An implementation of the [`Sender`] trait for the [`TestSender`] type
    #[async_trait]
    impl Sender<BLSPubKey> for TestSender {
        async fn send_direct_message(&self, message: &Bytes, recipient: BLSPubKey) -> Result<()> {
            self.network
                .get(&recipient)
                .ok_or(anyhow::anyhow!("recipient not found"))?
                .send(Arc::clone(message))
                .await
                .map_err(|_| anyhow::anyhow!("failed to send message"))?;

            Ok(())
        }

        async fn send_broadcast_message(&self, message: &Bytes) -> Result<()> {
            for sender in self.network.values() {
                sender
                    .send(Arc::clone(message))
                    .await
                    .map_err(|_| anyhow::anyhow!("failed to send message"))?;
            }
            Ok(())
        }
    }

    // Implement the [`RecipientSource`] trait for the [`TestSender`] type
    #[async_trait]
    impl RecipientSource<TestRequest, BLSPubKey> for TestSender {
        async fn get_expected_responders(&self, _request: &TestRequest) -> Result<Vec<BLSPubKey>> {
            // Get all the participants in the network
            Ok(self.network.keys().copied().collect())
        }
    }

    // Create a test request that is just some bytes
    #[derive(Clone, Debug)]
    struct TestRequest(Vec<u8>);

    // Implement the [`Serializable`] trait for the [`TestRequest`] type
    impl Serializable for TestRequest {
        fn to_bytes(&self) -> Result<Vec<u8>> {
            Ok(self.0.clone())
        }

        fn from_bytes(bytes: &[u8]) -> Result<Self> {
            Ok(TestRequest(bytes.to_vec()))
        }
    }

    // Implement the [`Request`] trait for the [`TestRequest`] type
    impl Request for TestRequest {
        type Response = Vec<u8>;
        fn validate(&self) -> Result<()> {
            Ok(())
        }
    }

    // Create a test data source that pretends to have the data or not
    #[derive(Clone)]
    struct TestDataSource {
        /// Whether we have the data or not
        has_data: bool,
        /// The time at which the data will be available if we have it
        data_available_time: Instant,

        /// Whether or not the data will be taken once served
        take_data: bool,
        /// Whether or not the data has been taken
        taken: Arc<AtomicBool>,
    }

    #[async_trait]
    impl DataSource<TestRequest> for TestDataSource {
        async fn derive_response_for(&self, request: &TestRequest) -> Result<Vec<u8>> {
            // Return a response if we hit the hit rate
            if self.has_data && Instant::now() >= self.data_available_time {
                if self.take_data && !self.taken.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    return Err(anyhow::anyhow!("data already taken"));
                }
                Ok(blake3::hash(&request.0).as_bytes().to_vec())
            } else {
                Err(anyhow::anyhow!("did not have the data"))
            }
        }
    }

    /// Create and return a default protocol configuration
    fn default_protocol_config() -> RequestResponseConfig {
        RequestResponseConfig {
            incoming_request_ttl: Duration::from_secs(40),
            incoming_request_timeout: Duration::from_secs(40),
            request_batch_size: 10,
            request_batch_interval: Duration::from_millis(100),
            max_incoming_requests: 10,
            max_incoming_requests_per_key: 1,
        }
    }

    /// Create fully connected test networks with `num_participants` participants
    fn create_participants(
        num: usize,
    ) -> Vec<(TestSender, mpsc::Receiver<Bytes>, (BLSPubKey, BLSPrivKey))> {
        // The entire network
        let mut network = HashMap::new();

        // All receivers in the network
        let mut receivers = Vec::new();

        // All keypairs in the network
        let mut keypairs = Vec::new();

        // For each participant,
        for i in 0..num {
            // Create a unique `BLSPubKey`
            let (public_key, private_key) =
                BLSPubKey::generated_from_seed_indexed([2; 32], i.try_into().unwrap());

            // Add the keypair to the list
            keypairs.push((public_key, private_key));

            // Create a channel for sending and receiving messages
            let (sender, receiver) = mpsc::channel::<Bytes>(100);

            // Add the participant to the network
            network.insert(public_key, sender);

            // Add the receiver to the list of receivers
            receivers.push(receiver);
        }

        // Create a test sender from the network
        let sender = TestSender {
            network: Arc::new(network),
        };

        // Return all senders and receivers
        receivers
            .into_iter()
            .zip(keypairs)
            .map(|(r, k)| (sender.clone(), r, k))
            .collect()
    }

    /// The configuration for an integration test
    #[derive(Clone)]
    struct IntegrationTestConfig {
        /// The request response protocol configuration
        request_response_config: RequestResponseConfig,
        /// The number of participants in the network
        num_participants: usize,
        /// The number of participants that have the data
        num_participants_with_data: usize,
        /// The timeout for the requests
        request_timeout: Duration,
        /// The delay before the nodes have the data available
        data_available_delay: Duration,
    }

    /// The result of an integration test
    struct IntegrationTestResult {
        /// The number of nodes that received a response
        num_succeeded: usize,
    }

    /// Run an integration test with the given parameters
    async fn run_integration_test(config: IntegrationTestConfig) -> IntegrationTestResult {
        // Create a fully connected network with `num_participants` participants
        let participants = create_participants(config.num_participants);

        // Create a join set to wait for all the tasks to finish
        let mut join_set = JoinSet::new();

        // We need to keep these here so they don't get dropped
        let handles = Arc::new(Mutex::new(Vec::new()));

        // For each one, create a new [`RequestResponse`] protocol
        for (i, (sender, receiver, (public_key, private_key))) in
            participants.into_iter().enumerate()
        {
            let config_clone = config.request_response_config.clone();
            let handles_clone = Arc::clone(&handles);
            join_set.spawn(async move {
                let protocol = RequestResponse::new(
                    config_clone,
                    sender.clone(),
                    receiver,
                    sender,
                    TestDataSource {
                        has_data: i < config.num_participants_with_data,
                        data_available_time: Instant::now() + config.data_available_delay,
                        take_data: false,
                        taken: Arc::new(AtomicBool::new(false)),
                    },
                );

                // Add the handle to the handles list so it doesn't get dropped and
                // cancelled
                #[allow(clippy::used_underscore_binding)]
                handles_clone
                    .lock()
                    .unwrap()
                    .push(Arc::clone(&protocol._receiving_task_handle));

                // Create a random request
                let request = TestRequest(vec![rand::thread_rng().r#gen(); 100]);

                // Get the hash of the request
                let request_hash = blake3::hash(&request.0).as_bytes().to_vec();

                // Create a new request message
                let request = RequestMessage::new_signed(&public_key, &private_key, &request)
                    .expect("failed to create request message");

                // Request the data from the protocol
                let response = protocol
                    .request(
                        request,
                        RequestType::Batched,
                        config.request_timeout,
                        |_request, response| async move { Ok(response) },
                    )
                    .await?;

                // Make sure the response is the hash of the request
                assert_eq!(response, request_hash);

                Ok::<(), anyhow::Error>(())
            });
        }

        // Wait for all the tasks to finish
        let mut num_succeeded = config.num_participants;
        while let Some(result) = join_set.join_next().await {
            if result.is_err() || result.unwrap().is_err() {
                num_succeeded -= 1;
            }
        }

        IntegrationTestResult { num_succeeded }
    }

    /// Test the integration of the protocol with 50% of the participants having the data
    #[tokio::test(flavor = "multi_thread")]
    async fn test_integration_50_0s() {
        // Build a config
        let config = IntegrationTestConfig {
            request_response_config: default_protocol_config(),
            num_participants: 100,
            num_participants_with_data: 50,
            request_timeout: Duration::from_secs(40),
            data_available_delay: Duration::from_secs(0),
        };

        // Run the test, making sure all the requests succeed
        let result = run_integration_test(config).await;
        assert_eq!(result.num_succeeded, 100);
    }

    /// Test the integration of the protocol when nobody has the data. Make sure we don't
    /// get any responses
    #[tokio::test(flavor = "multi_thread")]
    async fn test_integration_0() {
        // Build a config
        let config = IntegrationTestConfig {
            request_response_config: default_protocol_config(),
            num_participants: 100,
            num_participants_with_data: 0,
            request_timeout: Duration::from_secs(40),
            data_available_delay: Duration::from_secs(0),
        };

        // Run the test
        let result = run_integration_test(config).await;

        // Make sure all the requests succeeded
        assert_eq!(result.num_succeeded, 0);
    }

    /// Test the integration of the protocol when one node has the data after
    /// a delay of 1s
    #[tokio::test(flavor = "multi_thread")]
    async fn test_integration_1_1s() {
        // Build a config
        let config = IntegrationTestConfig {
            request_response_config: default_protocol_config(),
            num_participants: 100,
            num_participants_with_data: 1,
            request_timeout: Duration::from_secs(40),
            data_available_delay: Duration::from_secs(2),
        };

        // Run the test
        let result = run_integration_test(config).await;

        // Make sure all the requests succeeded
        assert_eq!(result.num_succeeded, 100);
    }

    /// Test that we can join an existing request for the same data and get the same (single) response
    #[tokio::test(flavor = "multi_thread")]
    async fn test_join_existing_request() {
        // Build a config
        let config = default_protocol_config();

        // Create two participants
        let mut participants = Vec::new();

        for (sender, receiver, (public_key, private_key)) in create_participants(2) {
            // For each, create a new [`RequestResponse`] protocol
            let protocol = RequestResponse::new(
                config.clone(),
                sender.clone(),
                receiver,
                sender,
                TestDataSource {
                    take_data: true,
                    has_data: true,
                    data_available_time: Instant::now() + Duration::from_secs(2),
                    taken: Arc::new(AtomicBool::new(false)),
                },
            );

            // Add the participants to the list
            participants.push((protocol, public_key, private_key));
        }

        // Take the first participant
        let one = Arc::new(participants.remove(0));

        // Create the request that they should all be able to join on
        let request = TestRequest(vec![rand::thread_rng().r#gen(); 100]);

        // Create a join set to wait for all the tasks to finish
        let mut join_set = JoinSet::new();

        // Make 10 requests with the same hash
        for _ in 0..10 {
            // Clone the first participant
            let one_clone = Arc::clone(&one);

            // Clone the request
            let request_clone = request.clone();

            // Spawn a task to request the data
            join_set.spawn(async move {
                // Create a new, signed request message
                let request_message =
                    RequestMessage::new_signed(&one_clone.1, &one_clone.2, &request_clone)?;

                // Start requesting it
                one_clone
                    .0
                    .request(
                        request_message,
                        RequestType::Batched,
                        Duration::from_secs(20),
                        |_request, response| async move { Ok(response) },
                    )
                    .await?;

                Ok::<(), anyhow::Error>(())
            });
        }

        // Wait for all the tasks to finish, making sure they all succeed
        while let Some(result) = join_set.join_next().await {
            result
                .expect("failed to join task")
                .expect("failed to request data");
        }
    }

    /// The concrete protocol type used in tests
    type TestProtocol = RequestResponse<
        TestSender,
        mpsc::Receiver<Bytes>,
        TestRequest,
        TestSender,
        TestDataSource,
        BLSPubKey,
    >;

    /// Create a protocol per participant, where each participant may or may not have the data
    fn create_protocols(
        num: usize,
        has_data: bool,
    ) -> Vec<(TestProtocol, (BLSPubKey, BLSPrivKey))> {
        create_participants(num)
            .into_iter()
            .map(|(sender, receiver, keypair)| {
                let protocol = RequestResponse::new(
                    default_protocol_config(),
                    sender.clone(),
                    receiver,
                    sender,
                    TestDataSource {
                        has_data,
                        data_available_time: Instant::now(),
                        take_data: false,
                        taken: Arc::new(AtomicBool::new(false)),
                    },
                );
                (protocol, keypair)
            })
            .collect()
    }

    /// Test that a timed-out request deregisters its waiter (active-requests map leak regression)
    #[tokio::test(flavor = "multi_thread")]
    async fn test_waiter_cleanup_on_timeout() {
        // Nobody has the data, so the request must time out
        let mut protocols = create_protocols(2, false);
        let (protocol, (public_key, private_key)) = protocols.remove(0);

        let request_message =
            RequestMessage::new_signed(&public_key, &private_key, &TestRequest(vec![1, 2, 3]))
                .expect("failed to create request message");

        let result = protocol
            .request(
                request_message,
                RequestType::Batched,
                Duration::from_millis(250),
                |_request, response| async move { Ok(response) },
            )
            .await;
        assert!(matches!(result, Err(RequestError::Timeout)));

        // The waiter must have deregistered itself
        assert!(protocol.active_requests.read().is_empty());
    }

    /// Test that two concurrent requests for the same data can validate to different output
    /// types. With the previous type-erased validation this would have failed at runtime with
    /// a downcast error
    #[tokio::test(flavor = "multi_thread")]
    async fn test_join_with_different_output_types() {
        let protocols = create_protocols(2, true);
        let (requester, (public_key, private_key)) = &protocols[0];

        let request = TestRequest(vec![5; 100]);
        let expected = blake3::hash(&request.0).as_bytes().to_vec();

        let request_message_1 = RequestMessage::new_signed(public_key, private_key, &request)
            .expect("failed to create request message");
        let request_message_2 = RequestMessage::new_signed(public_key, private_key, &request)
            .expect("failed to create request message");

        // One caller validates to the raw bytes, the other to their length
        let (bytes, length) = tokio::join!(
            requester.request(
                request_message_1,
                RequestType::Batched,
                Duration::from_secs(20),
                |_request, response| async move { Ok(response) },
            ),
            requester.request(
                request_message_2,
                RequestType::Batched,
                Duration::from_secs(20),
                |_request, response| async move { Ok(response.len()) },
            )
        );

        assert_eq!(bytes.expect("bytes request failed"), expected);
        assert_eq!(length.expect("length request failed"), expected.len());
    }

    /// Test that an invalid response does not complete a request, but a later valid one does
    #[tokio::test(flavor = "multi_thread")]
    async fn test_invalid_then_valid_response() {
        let protocols = create_protocols(2, true);
        let (requester, (public_key, private_key)) = &protocols[0];

        // Reject the first response we see and accept any later one
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = Arc::clone(&attempts);
        let validation_fn = move |_request: &TestRequest, response: Vec<u8>| {
            let attempts = Arc::clone(&attempts_clone);
            async move {
                if attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    return Err(anyhow::anyhow!("rejecting the first response"));
                }
                Ok(response)
            }
        };

        let request = TestRequest(vec![7; 100]);
        let request_message = RequestMessage::new_signed(public_key, private_key, &request)
            .expect("failed to create request message");
        let response = requester
            .request(
                request_message,
                RequestType::Batched,
                Duration::from_secs(20),
                validation_fn,
            )
            .await
            .expect("request failed");

        assert_eq!(response, blake3::hash(&request.0).as_bytes().to_vec());
        assert!(attempts.load(std::sync::atomic::Ordering::SeqCst) >= 2);

        // The waiter must have deregistered itself on success as well
        assert!(requester.active_requests.read().is_empty());
    }
}
