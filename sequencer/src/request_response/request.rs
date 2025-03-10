use std::io::Cursor;

use anyhow::{Context, Result};
use async_trait::async_trait;
use byteorder::ReadBytesExt;
use request_response::{
    request::{Request as RequestTrait, Response as ResponseTrait},
    Serializable,
};

/// The outermost request type. This an enum that contains all the possible requests that the
/// sequencer can make.
#[derive(Debug, Clone)]
pub enum Request {
    Example,
}

/// Implement the `RequestTrait` trait for the `Request` type. This tells the request response
/// protocol how to validate the request and what the response type is.
#[async_trait]
impl RequestTrait for Request {
    type Response = Response;

    async fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// The outermost response type. This an enum that contains all the possible responses that the
/// sequencer can make.
#[derive(Debug, Clone)]
pub enum Response {
    Example,
}

/// Implement the `ResponseTrait` trait for the `Response` type. This tells the request response
/// protocol how to validate the response and what the request type is.
#[async_trait]
impl ResponseTrait<Request> for Response {
    async fn validate(&self, request: &Request) -> Result<()> {
        // Match the type of the response and request
        match (self, request) {
            (Response::Example, Request::Example) => Ok(()),
        }
    }
}

/// Implement the `Serializable` trait for the `Request` type. This tells the request response
/// protocol how to serialize and deserialize the request
impl Serializable for Request {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        match self {
            Request::Example => Ok(vec![0]),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Create a cursor so we can read the bytes in order
        let mut cursor = Cursor::new(bytes);

        // Read the first byte, the request type
        let request_type = cursor.read_u8().with_context(|| "invalid request type")?;

        // Deserialize the request based on the type
        match request_type {
            0 => Ok(Request::Example),
            _ => Err(anyhow::anyhow!("invalid request type")),
        }
    }
}

/// Implement the `Serializable` trait for the `Response` type. This tells the request response
/// protocol how to serialize and deserialize the response.
impl Serializable for Response {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        match self {
            Response::Example => Ok(vec![0]),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Create a cursor so we can read the bytes in order
        let mut cursor = Cursor::new(bytes);

        // Read the first byte, the response type
        let response_type = cursor.read_u8().with_context(|| "invalid response type")?;

        // Deserialize the response based on the type
        match response_type {
            0 => Ok(Response::Example),
            _ => Err(anyhow::anyhow!("invalid response type")),
        }
    }
}
