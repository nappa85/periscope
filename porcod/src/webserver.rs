use std::{future::Future, net::SocketAddr, pin::Pin, sync::Arc, time::Duration};

use http::{header::HOST, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service,
};
use hyper_util::rt::TokioIo;
use regex::Regex;
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::Sender,
        oneshot::{self, error::RecvError},
    },
    time::{error::Elapsed, timeout},
};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error};

use crate::tls::Tls;

pub async fn run(
    addr: SocketAddr,
    cert: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
    filters: Vec<Regex>,
    timeout: Duration,
    request_tx: Sender<crate::ChannelItem>,
) -> anyhow::Result<()> {
    // Set a process wide default crypto provider.
    #[cfg(feature = "ring")]
    let _ = rustls::crypto::ring::default_provider().install_default();
    #[cfg(feature = "aws-lc-rs")]
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let tls_acceptor = cert
        .map(|(certs, key)| {
            ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .map(|mut server_config| {
                    server_config.alpn_protocols =
                        vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];
                    TlsAcceptor::from(Arc::new(server_config))
                })
        })
        .transpose()?;

    let listener = TcpListener::bind(&addr).await?;
    debug!("Webserver listening on http://{}", addr);

    let service = Service::new(filters, timeout, request_tx);
    loop {
        let (stream, _) = listener.accept().await?;

        tokio::spawn({
            let tls_acceptor = tls_acceptor.clone();
            let service = service.clone();
            async move {
                let io = match tls_acceptor {
                    Some(tls_acceptor) => match tls_acceptor.accept(stream).await {
                        Ok(stream) => Tls::Rustls { stream },
                        Err(err) => {
                            error!("failed to perform tls handshake: {err}");
                            return;
                        }
                    },
                    None => Tls::None { stream },
                };

                if let Err(err) = http1::Builder::new()
                    .serve_connection(TokioIo::new(io), service)
                    .await
                {
                    error!("Failed to serve connection: {err}");
                }
            }
        });
    }
}

#[derive(Debug, Clone)]
struct Service {
    filters: Vec<Regex>,
    timeout: Duration,
    request_tx: Sender<crate::ChannelItem>,
}

impl Service {
    fn new(filters: Vec<Regex>, timeout: Duration, request_tx: Sender<crate::ChannelItem>) -> Self {
        Self {
            filters,
            timeout,
            request_tx,
        }
    }
}

impl service::Service<Request<Incoming>> for Service {
    type Response = Response<Full<Bytes>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        debug!("Received request {req:?}");

        // filter outside the future to avoid filters lifetime and avoid cloning channel if unneeded all at once
        let request_tx = filtert_req(&self.filters, req.uri().path())
            .then(|| (self.timeout, self.request_tx.clone()));

        Box::pin(async move {
            let Some((call_timeout, request_tx)) = request_tx else {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::default())?);
            };

            let (head, body) = req.into_parts();

            let request = common::IncomingRequest {
                method: head.method,
                uri: head.uri,
                // here we use `iter` instead of `into_iter` to avoid having to deal with `Option<HeaderName>` on repeated names
                headers: head
                    .headers
                    .iter()
                    .filter_map(|(k, v)| {
                        // avoid sending HOST header
                        (k == HOST).then_some((k.clone(), v.clone()))
                    })
                    .collect(),
                body: body.collect().await?.to_bytes(),
            };

            let (oneshot_tx, oneshot_rx) = oneshot::channel();
            if request_tx.send((request, oneshot_tx)).await.is_err() {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::default())?);
            }

            let response = timeout(call_timeout, oneshot_rx).await??;

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
    Timeout(#[from] Elapsed),
    ChannelClosed(#[from] RecvError),
}

fn filtert_req(filters: &[Regex], path: &str) -> bool {
    filters.is_empty() || filters.iter().any(|regex| regex.is_match(path))
}
