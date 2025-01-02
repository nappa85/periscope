use std::{net::SocketAddr, sync::Arc};

use hyper::server::conn::http2::Builder;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use tokio::{net::TcpListener, sync::mpsc::Receiver};
use tokio_rustls::TlsAcceptor;
use tonic::{body::boxed, service::Routes};
use tower::ServiceExt;
use tracing::{debug, error};

use crate::tls::Tls;

mod inner;

pub async fn run(
    addr: SocketAddr,
    cert: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
    request_rx: Receiver<crate::ChannelItem>,
) -> anyhow::Result<()> {
    let inner = inner::Inner::new(request_rx);
    let svc = Routes::new(inner::inner_server::InnerServer::new(inner));
    let http = Builder::new(TokioExecutor::new());
    let listener = TcpListener::bind(addr).await?;

    let tls_acceptor = cert
        .map(|(certs, key)| {
            ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .map(|mut server_config| {
                    server_config.alpn_protocols = vec![b"h2".to_vec()];
                    TlsAcceptor::from(Arc::new(server_config))
                })
        })
        .transpose()?;

    debug!("gRPC listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::spawn({
            let http = http.clone();
            let tls_acceptor = tls_acceptor.clone();
            let svc = svc.clone();
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

                if let Err(err) = http
                    .serve_connection(
                        TokioIo::new(io),
                        TowerToHyperService::new(
                            svc.map_request(|req: http::Request<_>| req.map(boxed)),
                        ),
                    )
                    .await
                {
                    error!("gRPC server error: {err}");
                }
            }
        });
    }
}
