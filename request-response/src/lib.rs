//! This crate contains a general request-response protocol. It is used to send requests to
//! a set of recipients and wait for responses.

use std::{
    any::Any,
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Weak},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
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
    time::{sleep, timeout},
};
use tokio_util::task::AbortOnDropHandle;
use tracing::{debug, error, trace, warn};
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

/// A type alias for the outgoing requests map
pub type OutgoingRequestsMap<Req> =
    Arc<RwLock<HashMap<RequestHash, Weak<OutgoingRequestInner<Req>>>>>;

/// A type alias for the list of tasks that are responding to requests
pub type IncomingRequests<K> = NamedSemaphore<K>;

/// A type alias for the list of tasks that are validating incoming responses
pub type IncomingResponses = NamedSemaphore<()>;

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
    /// The maximum amount of time we will spend trying to validate a response. This is used to prevent
    /// an attack where a malicious participant sends us a bunch of requests that take a long time to
    /// validate.
    pub incoming_response_timeout: Duration,
    /// The batch size for outgoing requests. This is the number of request messages that we will
    /// send out at a time for a single request before waiting for the [`request_batch_interval`].
    pub request_batch_size: usize,
    /// The time to wait (per request) between sending out batches of request messages
    pub request_batch_interval: Duration,
    /// The maximum (global) number of incoming requests that can be processed at any given time.
    pub max_incoming_requests: usize,
    /// The maximum number of incoming requests that can be processed for a single key at any given time.
    pub max_incoming_requests_per_key: usize,
    /// The maximum (global) number of incoming responses that can be processed at any given time.
    /// We need this because responses coming in need to be validated [asynchronously] that they
    /// satisfy the request they are responding to
    pub max_incoming_responses: usize,
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
        // Create the outgoing requests map
        let outgoing_requests = OutgoingRequestsMap::default();

        // Create the inner implementation
        let inner = Arc::new(RequestResponseInner {
            config,
            sender,
            recipient_source,
            data_source,
            outgoing_requests,
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

/// A type alias for an `Arc<dyn Any + Send + Sync + 'static>`
type ThreadSafeAny = Arc<dyn Any + Send + Sync + 'static>;

/// A type alias for the future that validates a response
type ResponseValidationFuture =
    Pin<Box<dyn Future<Output = Result<ThreadSafeAny, anyhow::Error>> + Send + Sync + 'static>>;

/// A type alias for the function that returns the above future
type ResponseValidationFn<R> =
    Box<dyn Fn(&R, <R as Request>::Response) -> ResponseValidationFuture + Send + Sync + 'static>;

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
    outgoing_requests: OutgoingRequestsMap<Req>,
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
        F: Fn(&Req, Req::Response) -> Fut + Send + Sync + 'static + Clone,
        Fut: Future<Output = anyhow::Result<O>> + Send + Sync + 'static,
        O: Send + Sync + 'static + Clone,
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

    /// Request something from the protocol and wait for the response. This function
    /// will join with an existing request for the same data (determined by `Blake3` hash),
    /// however both will make requests until the timeout is reached
    ///
    /// # Errors
    /// - If the request times out
    /// - If the channel is closed (this is an internal error)
    /// - If the request we sign is invalid
    pub async fn request<F, Fut, O>(
        self: &Arc<Self>,
        request_message: RequestMessage<Req, K>,
        request_type: RequestType,
        timeout_duration: Duration,
        response_validation_fn: F,
    ) -> std::result::Result<O, RequestError>
    where
        F: Fn(&Req, Req::Response) -> Fut + Send + Sync + 'static + Clone,
        Fut: Future<Output = anyhow::Result<O>> + Send + Sync + 'static,
        O: Send + Sync + 'static + Clone,
    {
        timeout(timeout_duration, async move {
            // Calculate the hash of the request
            let request_hash = blake3::hash(&request_message.request.to_bytes().map_err(|e| {
                RequestError::InvalidRequest(anyhow::anyhow!(
                    "failed to serialize request message: {e}"
                ))
            })?);

            let request = {
                // Get a write lock on the outgoing requests map
                let mut outgoing_requests_write = self.outgoing_requests.write();

                // Conditionally get the outgoing request, creating a new one if it doesn't exist or if
                // the existing one has been dropped and not yet removed
                if let Some(outgoing_request) = outgoing_requests_write
                    .get(&request_hash)
                    .and_then(Weak::upgrade)
                {
                    OutgoingRequest(outgoing_request)
                } else {
                    // Create a new broadcast channel for the response
                    let (sender, receiver) = async_broadcast::broadcast(1);

                    // Modify the response validation function to return an `Arc<dyn Any>`
                    let response_validation_fn =
                        Box::new(move |request: &Req, response: Req::Response| {
                            let fut = response_validation_fn(request, response);
                            Box::pin(
                                async move { fut.await.map(|ok| Arc::new(ok) as ThreadSafeAny) },
                            ) as ResponseValidationFuture
                        });

                    // Create a new outgoing request
                    let outgoing_request = OutgoingRequest(Arc::new(OutgoingRequestInner {
                        sender,
                        receiver,
                        response_validation_fn,
                        request: request_message.request.clone(),
                        outgoing_requests: Arc::clone(&self.outgoing_requests),
                        request_hash,
                    }));

                    // Write the new outgoing request to the map
                    outgoing_requests_write
                        .insert(request_hash, Arc::downgrade(&outgoing_request.0));

                    // Return the new outgoing request
                    outgoing_request
                }
            };

            // Create a request message and serialize it
            let message = Bytes::from(
                Message::Request(request_message.clone())
                    .to_bytes()
                    .map_err(|e| {
                        RequestError::InvalidRequest(anyhow::anyhow!(
                            "failed to serialize request message: {e}"
                        ))
                    })?,
            );

            // Create a place to put the handle for the batched sending task. We need this because
            // otherwise it gets dropped when the closure goes out of scope, instead of when the function
            // gets cancelled or returns
            let mut _batched_sending_task = None;

            // Match on the type of request
            if request_type == RequestType::Broadcast {
                trace!("Sending request {:?} to all participants", request_message,);

                // If the message is a broadcast request, just send it to all participants
                self.sender
                    .send_broadcast_message(&message)
                    .await
                    .map_err(|e| {
                        RequestError::Other(anyhow::anyhow!(
                            "failed to send broadcast message: {e}"
                        ))
                    })?;
            } else {
                // If the message is a batched request, we need to batch it with other requests

                // Get the recipients that the request should expect responses from. Shuffle them so
                // that we don't always send to the same recipients in the same order
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

                // Get the current time so we can check when the timeout has elapsed
                let start_time = Instant::now();

                // Spawn a task that sends out requests to the network
                let self_clone = Arc::clone(self);
                let batched_sending_handle = AbortOnDropHandle::new(spawn(async move {
                    // Create a bounded queue for the outgoing requests. We use this to make sure
                    // we have less than [`config.request_batch_size`] requests in flight at any time.
                    //
                    // When newer requests are added, older ones are removed from the queue. Because we use
                    // `AbortOnDropHandle`, the older ones will automatically get cancelled
                    let mut outgoing_requests =
                        BoundedVecDeque::new(self_clone.config.request_batch_size);

                    // While the timeout hasn't elapsed, send out requests to the network
                    while start_time.elapsed() < timeout_duration {
                        // Send out requests to the network in their own separate tasks
                        for recipient_batch in
                            recipients.chunks(self_clone.config.request_batch_size)
                        {
                            for recipient in recipient_batch {
                                // Clone ourselves, the message, and the recipient so they can be moved
                                let self_clone = Arc::clone(&self_clone);
                                let request_message_clone = request_message.clone();
                                let recipient_clone = recipient.clone();
                                let message_clone = Arc::clone(&message);

                                // Spawn the task that sends the request to the participant
                                let individual_sending_task = spawn(async move {
                                    trace!(
                                        "Sending request {:?} to {:?}",
                                        request_message_clone,
                                        recipient_clone
                                    );

                                    let _ = self_clone
                                        .sender
                                        .send_direct_message(&message_clone, recipient_clone)
                                        .await;
                                });

                                // Add the sending task to the queue
                                outgoing_requests
                                    .push(AbortOnDropHandle::new(individual_sending_task));
                            }

                            // After we send the batch out, wait the [`config.request_batch_interval`]
                            // before sending the next one
                            sleep(self_clone.config.request_batch_interval).await;
                        }
                    }
                }));

                // Store the handle so it doesn't get dropped
                _batched_sending_task = Some(batched_sending_handle);
            }

            // Wait for a response on the channel
            request
                .receiver
                .clone()
                .recv()
                .await
                .map_err(|_| RequestError::Other(anyhow!("channel was closed")))
        })
        .await
        .map_err(|_| RequestError::Timeout)
        .and_then(|result| result)
        .and_then(|result| {
            result.downcast::<O>().map_err(|e| {
                RequestError::Other(anyhow::anyhow!(
                    "failed to downcast response to expected type: {:?}",
                    e
                ))
            })
        })
        .map(|result| Arc::unwrap_or_clone(result))
    }

    /// The task responsible for receiving messages from the receiver and handling them
    async fn receiving_task(self: Arc<Self>, mut receiver: R) {
        // Upper bound the number of outgoing and incoming responses
        let mut incoming_requests = NamedSemaphore::new(
            self.config.max_incoming_requests_per_key,
            Some(self.config.max_incoming_requests),
        );
        let mut incoming_responses = NamedSemaphore::new(self.config.max_incoming_responses, None);

        // While the receiver is open, we receive messages and handle them
        loop {
            // Try to receive a message
            match receiver.receive_message().await {
                Ok(message) => {
                    // Deserialize the message, warning if it fails
                    let message = match Message::from_bytes(&message) {
                        Ok(message) => message,
                        Err(e) => {
                            warn!("Received invalid message: {e:#}");
                            continue;
                        },
                    };

                    // Handle the message based on its type
                    match message {
                        Message::Request(request_message) => {
                            self.handle_request(request_message, &mut incoming_requests);
                        },
                        Message::Response(response_message) => {
                            self.handle_response(response_message, &mut incoming_responses);
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
                warn!(
                    "Failed to process request from {}: too many requests from the same key are already being processed",
                    request_message.public_key
                );
                return;
            },
            Err(NamedSemaphoreError::GlobalLimitReached) => {
                warn!(
                    "Failed to process request from {}: too many requests are already being processed",
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
                    .await
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

    /// Handle a response sent to us
    fn handle_response(
        self: &Arc<Self>,
        response: ResponseMessage<Req>,
        incoming_responses: &mut IncomingResponses,
    ) {
        trace!("Handling response {:?}", response);

        // Get the entry in the map, ignoring it if it doesn't exist
        let Some(outgoing_request) = self
            .outgoing_requests
            .read()
            .get(&response.request_hash)
            .cloned()
            .and_then(|r| r.upgrade())
        else {
            return;
        };

        // Attempt to acquire a permit for the request. Warn if there are too many responses currently being processed
        let permit = incoming_responses.try_acquire(());
        let Ok(permit) = permit else {
            warn!("Failed to process response: too many responses are already being processed");
            return;
        };

        // Spawn a task to validate the response and send it to the requester (us)
        let response_validate_timeout = self.config.incoming_response_timeout;
        tokio::spawn(async move {
            if timeout(response_validate_timeout, async move {
                // Make sure the response is valid for the given request
                let validation_result = match (outgoing_request.response_validation_fn)(
                    &outgoing_request.request,
                    response.response,
                )
                .await
                {
                    Ok(validation_result) => validation_result,
                    Err(e) => {
                        debug!("Received invalid response: {e:#}");
                        return;
                    },
                };

                // Send the response to the requester (the user of [`RequestResponse::request`])
                let _ = outgoing_request.sender.try_broadcast(validation_result);

                // Drop the permit
                _ = permit;
                drop(permit);
            })
            .await
            .is_err()
            {
                warn!("Timed out while validating response");
            }
        });
    }
}

/// An outgoing request. This is what we use to track a request and its corresponding response
/// in the protocol
#[derive(Clone, Deref)]
pub struct OutgoingRequest<R: Request>(Arc<OutgoingRequestInner<R>>);

/// The inner implementation of an outgoing request
pub struct OutgoingRequestInner<R: Request> {
    /// The sender to use for the protocol
    sender: async_broadcast::Sender<ThreadSafeAny>,
    /// The receiver to use for the protocol
    receiver: async_broadcast::Receiver<ThreadSafeAny>,

    /// The request that we are waiting for a response to
    request: R,

    /// The function used to validate the response
    response_validation_fn: ResponseValidationFn<R>,

    /// A copy of the map of currently active, outgoing requests
    outgoing_requests: OutgoingRequestsMap<R>,
    /// The hash of the request. We need this so we can remove ourselves from the map
    request_hash: RequestHash,
}

impl<R: Request> Drop for OutgoingRequestInner<R> {
    fn drop(&mut self) {
        self.outgoing_requests.write().remove(&self.request_hash);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{atomic::AtomicBool, Mutex},
    };

    use async_trait::async_trait;
    use hotshot_types::signature_key::{BLSPrivKey, BLSPubKey};
    use rand::Rng;
    use tokio::{sync::mpsc, task::JoinSet};

    use super::*;

    /// This test makes sure that when all references to an outgoing request are dropped, it is
    /// removed from the outgoing requests map
    #[test]
    fn test_outgoing_request_drop() {
        // Create an outgoing requests map
        let outgoing_requests = OutgoingRequestsMap::default();

        // Create an outgoing request
        let (sender, receiver) = async_broadcast::broadcast(1);
        let outgoing_request = OutgoingRequest(Arc::new(OutgoingRequestInner {
            sender,
            receiver,
            request: TestRequest(vec![1, 2, 3]),
            response_validation_fn: Box::new(|_request, _response| {
                Box::pin(async move { Ok(Arc::new(()) as ThreadSafeAny) })
                    as ResponseValidationFuture
            }),
            outgoing_requests: Arc::clone(&outgoing_requests),
            request_hash: blake3::hash(&[1, 2, 3]),
        }));

        // Insert the outgoing request into the map
        outgoing_requests.write().insert(
            outgoing_request.request_hash,
            Arc::downgrade(&outgoing_request.0),
        );

        // Clone the outgoing request
        let outgoing_request_clone = outgoing_request.clone();

        // Drop the outgoing request
        drop(outgoing_request);

        // Make sure nothing has been removed
        assert_eq!(outgoing_requests.read().len(), 1);

        // Drop the clone
        drop(outgoing_request_clone);

        // Make sure it has been removed
        assert_eq!(outgoing_requests.read().len(), 0);
    }

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
    #[async_trait]
    impl Request for TestRequest {
        type Response = Vec<u8>;
        async fn validate(&self) -> Result<()> {
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
            incoming_response_timeout: Duration::from_secs(1),
            max_incoming_responses: 5,
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
                let request = TestRequest(vec![rand::thread_rng().gen(); 100]);

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
        let request = TestRequest(vec![rand::thread_rng().gen(); 100]);

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
}
