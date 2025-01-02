use http::{HeaderName, HeaderValue, Method, StatusCode, Uri};
use hyper::body::Bytes;
use tokio::sync::oneshot::Sender;

// here we're using `mpsc` instead of `oneshot` because we need `Clone`
pub type ChannelItem = (IncomingRequest, Sender<OutgoingResponse>);

pub mod grpc;
pub mod webserver;

#[derive(Debug, Clone)]
pub struct IncomingRequest {
    method: Method,
    uri: Uri,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: Bytes,
}

#[derive(Debug, Clone)]
pub struct OutgoingResponse {
    status: StatusCode,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: Bytes,
}
