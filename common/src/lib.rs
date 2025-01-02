use http::{HeaderName, HeaderValue, Method, StatusCode, Uri};
use hyper::body::Bytes;

pub mod grpc;

#[derive(Debug, Clone)]
pub struct IncomingRequest {
    pub method: Method,
    pub uri: Uri,
    pub headers: Vec<(HeaderName, HeaderValue)>,
    pub body: Bytes,
}

#[derive(Debug, Clone)]
pub struct OutgoingResponse {
    pub status: StatusCode,
    pub headers: Vec<(HeaderName, HeaderValue)>,
    pub body: Bytes,
}
