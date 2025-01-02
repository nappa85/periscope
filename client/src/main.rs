use clap::Parser;
use tonic::transport::Uri;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// homeassistant url
    #[arg(short, long)]
    homeassistant_url: Uri,

    /// periscope server url
    #[arg(short, long)]
    periscope_url: Uri,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    periscope_client::start(args.periscope_url, args.homeassistant_url).await
}
