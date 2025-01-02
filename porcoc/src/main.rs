use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};

use clap::Parser;
use tonic::transport::{Certificate, Uri};

/// PORCO client
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// private service url
    #[arg(short = 'u', long)]
    target_url: Uri,

    /// porco server url
    #[arg(short = 'U', long)]
    porcod_url: Uri,

    /// grpc public certificate (pem format)
    #[arg(short = 'C', long)]
    porcod_certs: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    porcoc::start(
        args.porcod_certs.map(load_certs).transpose()?,
        args.porcod_url,
        args.target_url,
    )
    .await
}

fn load_certs(filename: PathBuf) -> io::Result<Certificate> {
    // Open certificate file.
    let certfile = fs::File::open(filename)?;
    let mut reader = io::BufReader::new(certfile);
    let mut pem = Vec::with_capacity(1024);
    reader.read_to_end(&mut pem)?;

    // Load and return certificate.
    Ok(Certificate::from_pem(pem))
}
