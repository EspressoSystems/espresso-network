use std::{
    io::{Cursor, Read, Write},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use hotshot_types::traits::signature_key::SignatureKey;

use super::{request::Request, RequestHash, Serializable};

/// The outer message type for the request-response protocol. Can either be a request or a response
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Message<R: Request, K: SignatureKey> {
    /// A request
    Request(RequestMessage<R, K>),
    /// A response
    Response(ResponseMessage<R>),
}

/// A request message, which includes the requester's public key, the request's signature, a timestamp, and the request itself
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct RequestMessage<R: Request, K: SignatureKey> {
    /// The requester's public key
    pub public_key: K,
    /// The requester's signature over the [the actual request content + timestamp]
    pub signature: K::PureAssembledSignatureType,
    /// The timestamp of when the request was sent (in seconds since the Unix epoch). We use this to
    /// ensure that the request is not old, which is useful for preventing replay attacks.
    pub timestamp_unix_seconds: u64,
    /// The actual request data. This is from the application
    pub request: R,
}

/// A response message, which includes the hash of the request we're responding to and the response itself.
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct ResponseMessage<R: Request> {
    /// The hash of the application-specific request we're responding to. The hash is a free way
    /// to identify the request and weed out any potential incompatibilities
    pub request_hash: RequestHash,
    /// The actual response content
    pub response: R::Response,
}

impl<R: Request, K: SignatureKey> RequestMessage<R, K> {
    /// Create a new signed request message from a request
    ///
    /// # Errors
    /// - If the request's content cannot be serialized
    /// - If the request cannot be signed
    ///
    /// # Panics
    /// - If time is not monotonic
    pub fn new_signed(public_key: &K, private_key: &K::PrivateKey, request: &R) -> Result<Self>
    where
        <K as SignatureKey>::SignError: 'static,
    {
        // Get the current timestamp
        let timestamp_unix_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_secs();

        // Concatenate the content and timestamp
        let content_to_sign = [
            request
                .to_bytes()
                .with_context(|| "failed to serialize request content")?
                .as_slice(),
            timestamp_unix_seconds.to_le_bytes().as_slice(),
            b"espresso-request-response",
        ]
        .concat();

        // Sign the actual request content (+ a namespace) with the private key
        let signature =
            K::sign(private_key, &content_to_sign).with_context(|| "failed to sign message")?;

        // Return the newly signed request message
        Ok(RequestMessage {
            public_key: public_key.clone(),
            signature,
            timestamp_unix_seconds,
            request: request.clone(),
        })
    }

    /// Validate the [`RequestMessage`], checking the signature and the timestamp and
    /// calling the request's application-specific validation function
    ///
    /// # Errors
    /// - If the request's signature is invalid
    /// - If the request is too old
    ///
    /// # Panics
    /// - If time is not monotonic
    pub async fn validate(&self, incoming_request_ttl: Duration) -> Result<()> {
        // Make sure the request is not too old
        if self
            .timestamp_unix_seconds
            .saturating_add(incoming_request_ttl.as_secs())
            < SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_secs()
        {
            return Err(anyhow::anyhow!("request is too old"));
        }
        // Check the signature over the request content and timestamp
        if !self.public_key.validate(
            &self.signature,
            &[
                self.request.to_bytes()?,
                self.timestamp_unix_seconds.to_le_bytes().to_vec(),
                b"espresso-request-response".to_vec(),
            ]
            .concat(),
        ) {
            return Err(anyhow::anyhow!("invalid request signature"));
        }

        // Call the request's application-specific validation function
        self.request.validate().await
    }
}

/// A blanket implementation of the [`Serializable`] trait for any [`Message`]
impl<R: Request, K: SignatureKey> Serializable for Message<R, K> {
    /// Converts any [`Message`] to bytes if the content is also [`Serializable`]
    fn to_bytes(&self) -> Result<Vec<u8>> {
        // Create a buffer for the bytes
        let mut bytes = Vec::new();

        // Convert the message to bytes based on the type. By default it is just type-prefixed
        match self {
            Message::Request(request_message) => {
                // Write the type (request)
                bytes.push(0);

                // Write the request content
                bytes.extend_from_slice(request_message.to_bytes()?.as_slice());
            },
            Message::Response(response_message) => {
                // Write the type (response)
                bytes.push(1);

                // Write the response content
                bytes.extend_from_slice(response_message.to_bytes()?.as_slice());
            },
        };

        Ok(bytes)
    }

    /// Convert bytes to a [`Message`]
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Create a cursor so we can easily read the bytes in order
        let mut bytes = Cursor::new(bytes);

        // Get the message type
        let type_byte = bytes.read_u8()?;

        // Deserialize the message based on the type
        match type_byte {
            0 => {
                // Read the `RequestMessage`
                Ok(Message::Request(RequestMessage::from_bytes(&read_to_end(
                    &mut bytes,
                )?)?))
            },
            1 => {
                // Read the `ResponseMessage`
                Ok(Message::Response(ResponseMessage::from_bytes(
                    &read_to_end(&mut bytes)?,
                )?))
            },
            _ => Err(anyhow::anyhow!("invalid message type")),
        }
    }
}

impl<R: Request, K: SignatureKey> Serializable for RequestMessage<R, K> {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        // Create a buffer for the bytes
        let mut bytes = Vec::new();

        // Write the public key (length-prefixed)
        write_length_prefixed(&mut bytes, &self.public_key.to_bytes())?;

        // Write the signature (length-prefixed)
        write_length_prefixed(&mut bytes, &bincode::serialize(&self.signature)?)?;

        // Write the timestamp
        bytes.write_all(&self.timestamp_unix_seconds.to_le_bytes())?;

        // Write the actual request
        bytes.write_all(self.request.to_bytes()?.as_slice())?;

        Ok(bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Create a cursor so we can easily read the bytes in order
        let mut bytes = Cursor::new(bytes);

        // Read the public key (length-prefixed)
        let public_key = K::from_bytes(&read_length_prefixed(&mut bytes)?)?;

        // Read the signature (length-prefixed)
        let signature = bincode::deserialize(&read_length_prefixed(&mut bytes)?)?;

        // Read the timestamp as a [`u64`]
        let timestamp = bytes.read_u64::<LittleEndian>()?;

        // Deserialize the request
        let request = R::from_bytes(&read_to_end(&mut bytes)?)?;

        Ok(Self {
            public_key,
            signature,
            timestamp_unix_seconds: timestamp,
            request,
        })
    }
}

impl<R: Request> Serializable for ResponseMessage<R> {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        // Create a buffer for the bytes
        let mut bytes = Vec::new();

        // Write the request hash as bytes
        bytes.write_all(self.request_hash.as_bytes())?;

        // Write the response content
        bytes.write_all(self.response.to_bytes()?.as_slice())?;

        Ok(bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Create a buffer for the bytes
        let mut bytes = Cursor::new(bytes);

        // Read the request hash as a [`blake3::Hash`]
        let mut request_hash_bytes = [0; 32];
        bytes.read_exact(&mut request_hash_bytes)?;
        let request_hash = RequestHash::from(request_hash_bytes);

        // Read the response content to the end
        let response = R::Response::from_bytes(&read_to_end(&mut bytes)?)?;

        Ok(Self {
            request_hash,
            response,
        })
    }
}

/// A helper function to write a length-prefixed value to a writer
fn write_length_prefixed<W: Write>(writer: &mut W, value: &[u8]) -> Result<()> {
    // Write the length of the value as a u32
    writer.write_u32::<LittleEndian>(
        u32::try_from(value.len()).with_context(|| "value was too large")?,
    )?;

    // Write the (already serialized) value
    writer.write_all(value)?;
    Ok(())
}

/// A helper function to read a length-prefixed value from a reader
fn read_length_prefixed<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
    // Read the length of the value as a u32
    let length = reader.read_u32::<LittleEndian>()?;

    // Read the value
    let mut value = vec![0; length as usize];
    reader.read_exact(&mut value)?;
    Ok(value)
}

/// A helper function to read to the end of the reader
fn read_to_end<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
    let mut value = Vec::new();
    reader.read_to_end(&mut value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use hotshot_types::signature_key::BLSPubKey;
    use rand::Rng;

    use super::*;

    // A testing implementation of the [`Serializable`] trait for [`Vec<u8>`]
    impl Serializable for Vec<u8> {
        fn to_bytes(&self) -> Result<Vec<u8>> {
            Ok(self.clone())
        }
        fn from_bytes(bytes: &[u8]) -> Result<Self> {
            Ok(bytes.to_vec())
        }
    }

    /// A testing implementation of the [`Request`] trait for [`Vec<u8>`]
    #[async_trait]
    impl Request for Vec<u8> {
        type Response = Vec<u8>;

        async fn validate(&self) -> Result<()> {
            Ok(())
        }
    }

    /// Tests that properly signed requests are validated correctly and that invalid requests
    /// (bad timestamp/signature) are rejected
    #[tokio::test]
    async fn test_request_validation() {
        // Create some RNG
        let mut rng = rand::thread_rng();

        for _ in 0..100 {
            // Create a random keypair
            let (public_key, private_key) =
                BLSPubKey::generated_from_seed_indexed([1; 32], rng.gen::<u64>());

            // Create a valid request with some random content
            let mut request = RequestMessage::new_signed(
                &public_key,
                &private_key,
                &vec![rng.gen::<u8>(); rng.gen_range(1..10000)],
            )
            .expect("Failed to create signed request");

            let (should_be_valid, request_ttl) = match rng.gen_range(0..4) {
                0 => (true, Duration::from_secs(1)),

                1 => {
                    // Alter the requests's actual content
                    request.request[0] = !request.request[0];

                    // It should not be valid anymore
                    (false, Duration::from_secs(1))
                },

                2 => {
                    // Alter the timestamp
                    request.timestamp_unix_seconds += 1000;

                    // It should not be valid anymore
                    (false, Duration::from_secs(1))
                },

                3 => {
                    // Change the request ttl to be 0. This should make the request
                    // invalid immediately
                    (true, Duration::from_secs(0))
                },

                _ => unreachable!(),
            };

            // Validate the request
            assert_eq!(request.validate(request_ttl).await.is_ok(), should_be_valid);
        }
    }

    /// Tests that messages are serialized and deserialized correctly
    #[test]
    fn test_message_parity() {
        for _ in 0..100 {
            // Create some RNG
            let mut rng = rand::thread_rng();

            // Generate a random message type
            let is_request = rng.gen::<u8>() % 2 == 0;

            // The request content will be a random vector of bytes
            let request = vec![rng.gen::<u8>(); rng.gen_range(0..10000)];

            // Create a message
            let message = if is_request {
                // Create a random keypair
                let (public_key, private_key) =
                    BLSPubKey::generated_from_seed_indexed([1; 32], rng.gen::<u64>());

                // Create a new signed request
                let request = RequestMessage::new_signed(&public_key, &private_key, &request)
                    .expect("Failed to create signed request");

                Message::Request(request)
            } else {
                // Create a response message
                Message::Response(ResponseMessage {
                    request_hash: blake3::hash(&request),
                    response: vec![rng.gen::<u8>(); rng.gen_range(0..10000)],
                })
            };

            // Serialize the message
            let serialized = message.to_bytes().expect("Failed to serialize message");

            // Deserialize the message
            let deserialized =
                Message::from_bytes(&serialized).expect("Failed to deserialize message");

            // Assert that the deserialized message is the same as the original message
            assert_eq!(message, deserialized);
        }
    }

    /// Tests that length-prefixed values are read and written correctly
    #[test]
    fn test_length_prefix_parity() {
        // Create some RNG
        let mut rng = rand::thread_rng();

        for _ in 0..100 {
            // Create a buffer to test over
            let mut bytes = Vec::new();

            // Generate the value to test over
            let value = vec![rng.gen::<u8>(); rng.gen_range(0..10000)];

            // Write the length-prefixed value
            write_length_prefixed(&mut bytes, &value).unwrap();

            // Create a reader from the bytes
            let mut reader = Cursor::new(bytes);

            // Read the length-prefixed value
            let value = read_length_prefixed(&mut reader).unwrap();
            assert_eq!(value, value);
        }
    }
}
