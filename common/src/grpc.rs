use std::str::FromStr;

use http::{HeaderName, HeaderValue, Method, StatusCode, Uri};
use hyper::body::Bytes;
use tonic::Status;

tonic::include_proto!("common");

impl From<(u64, crate::IncomingRequest)> for IncomingRequest {
    fn from((id, request): (u64, crate::IncomingRequest)) -> Self {
        let crate::IncomingRequest {
            method,
            uri,
            headers,
            body,
        } = request;

        let method = method.as_str().to_owned();
        let uri = uri.to_string();
        let headers = headers.into_iter().map(Header::from).collect();
        let body = body.to_vec();

        Self {
            id,
            uri,
            method,
            headers,
            body,
        }
    }
}

impl TryFrom<IncomingRequest> for crate::IncomingRequest {
    type Error = Status;

    fn try_from(value: IncomingRequest) -> Result<Self, Self::Error> {
        let IncomingRequest {
            id: _,
            method,
            uri,
            headers,
            body,
        } = value;

        let method = Method::from_bytes(method.as_bytes())
            .map_err(|_| Status::invalid_argument("Invalid method"))?;
        let uri = Uri::from_str(&uri).map_err(|_| Status::invalid_argument("Invalid uri"))?;
        let headers = headers
            .into_iter()
            .map(<(HeaderName, HeaderValue)>::try_from)
            .collect::<Result<_, _>>()?;
        let body = Bytes::from(body);

        Ok(Self {
            method,
            uri,
            headers,
            body,
        })
    }
}

impl From<(u64, crate::OutgoingResponse)> for OutgoingResponse {
    fn from((id, response): (u64, crate::OutgoingResponse)) -> Self {
        let crate::OutgoingResponse {
            status,
            headers,
            body,
        } = response;

        let status = status.as_u16() as u32;
        let headers = headers.into_iter().map(Header::from).collect();
        let body = body.to_vec();

        Self {
            id,
            status,
            headers,
            body,
        }
    }
}

impl TryFrom<OutgoingResponse> for crate::OutgoingResponse {
    type Error = Status;

    fn try_from(value: OutgoingResponse) -> Result<Self, Self::Error> {
        let OutgoingResponse {
            id: _,
            status,
            headers,
            body,
        } = value;

        let status =
            u16::try_from(status).map_err(|_| Status::invalid_argument("Invalid status"))?;
        let status = StatusCode::from_u16(status)
            .map_err(|_| Status::invalid_argument("Invalid status code"))?;
        let headers = headers
            .into_iter()
            .map(<(HeaderName, HeaderValue)>::try_from)
            .collect::<Result<_, _>>()?;
        let body = Bytes::from(body);

        Ok(Self {
            status,
            headers,
            body,
        })
    }
}

impl From<(HeaderName, HeaderValue)> for Header {
    fn from((k, v): (HeaderName, HeaderValue)) -> Self {
        Self {
            name: k.as_str().as_bytes().to_owned(),
            value: v.as_bytes().to_owned(),
        }
    }
}

impl TryFrom<Header> for (HeaderName, HeaderValue) {
    type Error = Status;

    fn try_from(value: Header) -> Result<Self, Self::Error> {
        match (
            HeaderName::from_bytes(&value.name),
            HeaderValue::from_bytes(&value.value),
        ) {
            (Err(_), _) => Err(Status::invalid_argument("Invalid header name")),
            (_, Err(_)) => Err(Status::invalid_argument("Invalid header value")),
            (Ok(k), Ok(v)) => Ok((k, v)),
        }
    }
}
