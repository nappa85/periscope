use std::net::SocketAddr;

use clap::Parser;
use periscope_server::{grpc, webserver};
use tokio::sync::mpsc::channel;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// grpc bind address
    #[arg(short, long, default_value_t = SocketAddr::from(([0, 0, 0, 0], 50051)))]
    grpc_addr: SocketAddr,

    /// webserver bind address
    #[arg(short, long, default_value_t = SocketAddr::from(([0, 0, 0, 0], 80)))]
    webserver_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let (tx, rx) = channel(1);

    tokio::select! {
        res = webserver::run(args.webserver_addr, tx) => res,
        res = grpc::run(args.grpc_addr, rx) => res,
    }
}
