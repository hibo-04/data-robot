mod error;
mod html;
mod overlay;
mod routes;
mod state;

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::serve;

pub use routes::router;
pub use state::AppState;

pub struct Config {
    pub bind: SocketAddr,
    pub fixtures: PathBuf,
    pub overlays: PathBuf,
}

pub fn default_fixtures() -> PathBuf {
    let from_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures");
    if from_crate.is_dir() {
        from_crate
    } else {
        PathBuf::from("fixtures")
    }
}

pub fn default_overlays() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../out/overlays")
}

pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fixtures = config.fixtures.canonicalize().unwrap_or(config.fixtures);
    let state = AppState::new(fixtures.clone(), config.overlays);
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    eprintln!(
        "analytics-web listening on http://{}  fixtures {}",
        config.bind,
        fixtures.display()
    );
    serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    fn test_state() -> AppState {
        AppState::new(
            default_fixtures(),
            std::env::temp_dir().join("analytics-web-test-overlays"),
        )
    }

    #[tokio::test]
    async fn home_lists_fixtures() {
        let app = router(test_state());
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains("ecommerce_clean"), "{body}");
        assert!(body.contains("crm"));
    }

    #[tokio::test]
    async fn source_page_analyzes_fixture() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/source/ecommerce_clean?packs=generic")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains("Relationships"), "{body}");
        assert!(body.contains("KPIs"));
        assert!(body.contains("Identities"));
        assert!(body.contains("Accept") || body.contains("Reject"));
    }
}
