use std::{collections::TryReserveError, convert::Infallible, io, marker::PhantomData};

use async_trait::async_trait;
use cbor4ii::core::error::DecodeError;
use futures::prelude::*;
use libp2p::{
    request_response::{self, Codec},
    StreamProtocol,
};
use serde::{de::DeserializeOwned, Serialize};

/// `Behaviour` type alias for the `Cbor` codec
pub type Behaviour<Req, Resp> = request_response::Behaviour<Cbor<Req, Resp>>;

/// Forked `cbor` codec with altered request/response sizes
pub struct Cbor<Req, Resp> {
    /// Phantom data
    phantom: PhantomData<(Req, Resp)>,
    /// Maximum request size in bytes
    request_size_maximum: u64,
    /// Maximum response size in bytes
    response_size_maximum: u64,
}

impl<Req, Resp> Default for Cbor<Req, Resp> {
    fn default() -> Self {
        Cbor {
            phantom: PhantomData,
            request_size_maximum: 20 * 1024 * 1024,
            response_size_maximum: 20 * 1024 * 1024,
        }
    }
}

impl<Req, Resp> Cbor<Req, Resp> {
    /// Create a new `Cbor` codec with the given request and response sizes
    #[must_use]
    pub fn new(request_size_maximum: u64, response_size_maximum: u64) -> Self {
        Cbor {
            phantom: PhantomData,
            request_size_maximum,
            response_size_maximum,
        }
    }
}

impl<Req, Resp> Clone for Cbor<Req, Resp> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[async_trait]
impl<Req, Resp> Codec for Cbor<Req, Resp>
where
    Req: Send + Serialize + DeserializeOwned,
    Resp: Send + Serialize + DeserializeOwned,
{
    type Protocol = StreamProtocol;
    type Request = Req;
    type Response = Resp;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Req>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();

        io.take(self.request_size_maximum)
            .read_to_end(&mut vec)
            .await?;

        cbor4ii::serde::from_slice(vec.as_slice()).map_err(decode_into_io_error)
    }

    async fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Resp>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();

        io.take(self.response_size_maximum)
            .read_to_end(&mut vec)
            .await?;

        cbor4ii::serde::from_slice(vec.as_slice()).map_err(decode_into_io_error)
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data: Vec<u8> =
            cbor4ii::serde::to_vec(Vec::new(), &req).map_err(encode_into_io_error)?;

        io.write_all(data.as_ref()).await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        resp: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data: Vec<u8> =
            cbor4ii::serde::to_vec(Vec::new(), &resp).map_err(encode_into_io_error)?;

        io.write_all(data.as_ref()).await?;

        Ok(())
    }
}

/// Convert a `cbor4ii::serde::DecodeError` into an `io::Error`
fn decode_into_io_error(err: cbor4ii::serde::DecodeError<Infallible>) -> io::Error {
    match err {
        cbor4ii::serde::DecodeError::Core(DecodeError::Read(e)) => {
            io::Error::new(io::ErrorKind::Other, e.to_string())
        },
        cbor4ii::serde::DecodeError::Core(e @ DecodeError::Unsupported { .. }) => {
            io::Error::new(io::ErrorKind::Unsupported, e.to_string())
        },
        cbor4ii::serde::DecodeError::Core(e @ DecodeError::Eof { .. }) => {
            io::Error::new(io::ErrorKind::UnexpectedEof, e.to_string())
        },
        cbor4ii::serde::DecodeError::Core(e) => {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        },
        cbor4ii::serde::DecodeError::Custom(e) => {
            io::Error::new(io::ErrorKind::Other, e.to_string())
        },
    }
}

/// Convert a `cbor4ii::serde::EncodeError` into an `io::Error`
fn encode_into_io_error(err: cbor4ii::serde::EncodeError<TryReserveError>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}
