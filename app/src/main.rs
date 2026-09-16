use std::net::SocketAddr;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "octa-app",
    about = "Product app shell around the analytics engine (working title: Octa)"
)]
struct Args {
    /// Bind address. Loopback only by default.
    #[arg(long, default_value_t = octa_app::default_bind())]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = octa_app::Config { bind: args.bind };
    if let Err(err) = octa_app::run(config).await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
