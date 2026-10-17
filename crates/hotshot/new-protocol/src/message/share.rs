use hotshot_types::{
    data::{VidCommitment2, VidDisperseShare2, ViewNumber},
    traits::node_implementation::NodeType,
    vote::HasViewNumber,
};
use serde::{Deserialize, Serialize};

use crate::message::fetch::{Request, Response};

pub type ShareFetchRequest = Request<ShareRequestBody>;
pub type ShareFetchResponse<T> = Response<ShareResponseBody<T>>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
pub struct ShareRequestBody {
    pub commitment: VidCommitment2,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub enum ShareResponseBody<T: NodeType> {
    NotHeld,
    TooLarge,
    Busy,
    Share(Box<VidDisperseShare2<T>>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub enum ShareFetchMessage<T: NodeType> {
    Req(ShareFetchRequest),
    Res(ShareFetchResponse<T>),
}

impl<T: NodeType> HasViewNumber for ShareFetchMessage<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            Self::Req(r) => r.view_number(),
            Self::Res(r) => r.view_number(),
        }
    }
}
