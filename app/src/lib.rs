//! Product app around the analytics engine. Working title: Octa.
//!
//! Two worlds live in this workspace:
//!
//! - `web/` (`analytics-web`): the local, login-free PoC review page. Throwaway.
//! - `app/` (this crate): the B2B product. Accounts, organisations, sessions,
//!   locale, jobs and the control-plane database belong here — never in `analytics`.
//!
//! Rules for this crate:
//!
//! - Use the engine only through the public `analytics` API
//!   (`analyze`, `analyze_with_packs`, `query_from_analysis`, overlay/publish once P0.4 lands).
//! - The core stays free of Axum, HTML, sessions and tenant filters.
//!   `tests/boundary.rs` guards that direction.
//! - Every user-visible string lives in `app/locales/<lang>.json` and ships in all active
//!   languages in the same change ([`i18n`]). Language, formats and time zones are separate
//!   axes ([`locale`]); storage is UTC, display follows the person.
//!
//! Roadmap (`docs/roadmap.md`): P0.1 empty shell, P0.2 locale stack.

pub mod i18n;
pub mod locale;
mod pages;
mod routes;
mod security;

use std::net::SocketAddr;

pub use routes::router;

/// Runtime configuration of the app shell.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address to bind. Loopback by default; TLS terminates in front of the app.
    pub bind: SocketAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: default_bind(),
        }
    }
}

/// Loopback, port 4000 — distinct from the PoC (`analytics-web`, 3000) so both can run.
pub fn default_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 4000))
}

/// Bind and serve until the process ends.
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let addr = listener.local_addr()?;
    eprintln!(
        "{} {} listening on http://{addr}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    axum::serve(listener, router()).await?;
    Ok(())
}
