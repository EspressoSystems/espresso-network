use hotshot_types::{data::ViewNumber, vote::HasViewNumber};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
pub struct Request<T> {
    view: ViewNumber,
    body: T,
}

impl<T> Request<T> {
    pub fn new(view: ViewNumber, body: T) -> Self {
        Self { view, body }
    }

    pub fn body(&self) -> &T {
        &self.body
    }

    pub fn into_body(self) -> T {
        self.body
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
pub struct Response<T> {
    view: ViewNumber,
    body: T,
}

impl<T> Response<T> {
    pub fn new(view: ViewNumber, body: T) -> Self {
        Self { view, body }
    }

    pub fn body(&self) -> &T {
        &self.body
    }

    pub fn into_body(self) -> T {
        self.body
    }
}

impl<T> HasViewNumber for Request<T> {
    fn view_number(&self) -> ViewNumber {
        self.view
    }
}

impl<T> HasViewNumber for Response<T> {
    fn view_number(&self) -> ViewNumber {
        self.view
    }
}
