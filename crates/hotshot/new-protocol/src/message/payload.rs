use hotshot_types::{
    data::{VidCommitment2, ViewNumber},
    vote::HasViewNumber,
};
use serde::{Deserialize, Serialize};

use crate::message::fetch::{Request, Response};

pub type PayloadFetchRequest = Request<PayloadRequestBody>;
pub type PayloadFetchResponse = Response<PayloadResponseBody>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
pub struct PayloadRequestBody;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
pub enum PayloadResponseBody {
    NotAvailable,
    TooLarge,
    Payload {
        commitment: VidCommitment2,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
pub enum PayloadFetchMessage {
    Req(PayloadFetchRequest),
    Res(PayloadFetchResponse),
}

impl HasViewNumber for PayloadFetchMessage {
    fn view_number(&self) -> ViewNumber {
        match self {
            Self::Req(r) => r.view_number(),
            Self::Res(r) => r.view_number(),
        }
    }
}
