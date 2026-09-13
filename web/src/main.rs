use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "analytics-web",
    about = "Minimal local UI for the analytics engine"
)]
struct Args {
    /// Bind address. Localhost only by default.
    #[arg(long, default_value = "127.0.0.1:3000")]
    bind: SocketAddr,
    /// Fixture directory (CSV models).
    #[arg(long)]
    fixtures: Option<PathBuf>,
    /// Directory for model-review overlay JSON files.
    #[arg(long)]
    overlays: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = analytics_web::Config {
        bind: args.bind,
        fixtures: args
            .fixtures
            .unwrap_or_else(analytics_web::default_fixtures),
        overlays: args
            .overlays
            .unwrap_or_else(analytics_web::default_overlays),
    };
    if let Err(err) = analytics_web::run(config).await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
