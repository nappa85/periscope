use std::net::SocketAddr;

use tokio::sync::mpsc::Receiver;
use tonic::transport::Server;
use tracing::debug;

mod inner;

pub async fn run(addr: SocketAddr, request_rx: Receiver<crate::ChannelItem>) -> anyhow::Result<()> {
    let inner = inner::Inner::new(request_rx);

    debug!("gRPC listening on http://{}", addr);

    Server::builder()
        .add_service(inner::inner_server::InnerServer::new(inner))
        .serve(addr)
        .await?;

    Ok(())
}
