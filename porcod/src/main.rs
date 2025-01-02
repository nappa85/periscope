use std::{fs, io, net::SocketAddr, path::PathBuf, time::Duration};

use clap::Parser;
use porcod::{grpc, webserver};
use regex::Regex;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::sync::mpsc::channel;

/// PORCO daemon
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// grpc bind address
    #[arg(short = 'A', long, default_value_t = SocketAddr::from(([0, 0, 0, 0], 50051)))]
    grpc_addr: SocketAddr,

    /// grpc public certificate (pem format)
    #[arg(short = 'C', long)]
    grpc_certs: Option<PathBuf>,

    /// grpc private key
    #[arg(short = 'K', long)]
    grpc_private_key: Option<PathBuf>,

    /// webserver bind address
    #[arg(short = 'a', long, default_value_t = SocketAddr::from(([0, 0, 0, 0], 80)))]
    webserver_addr: SocketAddr,

    /// webserver public certificate (pem format)
    #[arg(short = 'c', long)]
    webserver_certs: Option<PathBuf>,

    /// webserver private key
    #[arg(short = 'k', long)]
    webserver_private_key: Option<PathBuf>,

    /// webserver incoming filters
    #[arg(short = 'f', long)]
    webserver_filters: Vec<Regex>,

    /// webserver timeout in seconds
    #[arg(short = 't', long, default_value_t = 60)]
    webserver_timeout: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let (tx, rx) = channel(1);

    tokio::select! {
        res = webserver::run(
            args.webserver_addr,
            args.webserver_certs.zip(args.webserver_private_key).map(load_certs).transpose()?,
            args.webserver_filters,
            Duration::from_secs(args.webserver_timeout),
            tx
        ) => res,
        res = grpc::run(
            args.grpc_addr,
            args.grpc_certs.zip(args.grpc_private_key).map(load_certs).transpose()?,
            rx
        ) => res,
    }
}

pub fn load_certs(
    (certs, private_key): (PathBuf, PathBuf),
) -> io::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    Ok((load_public_certs(certs)?, load_private_key(private_key)?))
}

// Load public certificate from file.
pub fn load_public_certs(filename: PathBuf) -> io::Result<Vec<CertificateDer<'static>>> {
    // Open certificate file.
    let certfile = fs::File::open(filename)?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    rustls_pemfile::certs(&mut reader).collect()
}

// Load private key from file.
pub fn load_private_key(filename: PathBuf) -> io::Result<PrivateKeyDer<'static>> {
    // Open keyfile.
    let keyfile = fs::File::open(filename)?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    rustls_pemfile::private_key(&mut reader).map(|key| key.unwrap())
}
