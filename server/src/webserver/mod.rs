use std::{future::Future, net::SocketAddr, pin::Pin};

use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service,
};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::Sender,
        oneshot::{self, error::RecvError},
    },
};
use tracing::{debug, error};

mod tokiort;

pub async fn run(addr: SocketAddr, request_tx: Sender<crate::ChannelItem>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    debug!("Webserver listening on http://{}", addr);

    let service = Service::new(request_tx);
    loop {
        let (stream, _) = listener.accept().await?;
        let io = tokiort::TokioIo::new(stream);

        tokio::spawn({
            let service = service.clone();
            async move {
                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    error!("Failed to serve connection: {:?}", err);
                }
            }
        });
    }
}

#[derive(Debug, Clone)]
struct Service {
    request_tx: Sender<crate::ChannelItem>,
}

impl Service {
    fn new(request_tx: Sender<crate::ChannelItem>) -> Self {
        Self { request_tx }
    }
}

impl service::Service<Request<Incoming>> for Service {
    type Response = Response<Full<Bytes>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let request_tx = self.request_tx.clone();
        Box::pin(async move {
            let (head, body) = req.into_parts();

            let request = crate::IncomingRequest {
                method: head.method,
                uri: head.uri,
                // here we use `iter` instead of `into_iter` to avoid having to deal with `Option<HeaderName>` on repeated names
                headers: head
                    .headers
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                body: body.collect().await?.to_bytes(),
            };

            let (oneshot_tx, oneshot_rx) = oneshot::channel();
            if request_tx.send((request, oneshot_tx)).await.is_err() {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::default())?);
            }

            let response = oneshot_rx.await?;

            let mut builder = Response::builder().status(response.status);
            for (k, v) in response.headers {
                builder = builder.header(k, v);
            }
            Ok(builder.body(Full::from(response.body))?)
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    Http(#[from] http::Error),
    Hyper(#[from] hyper::Error),
    ChannelClosed(#[from] RecvError),
}
