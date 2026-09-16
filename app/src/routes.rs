use axum::extract::RawQuery;
use axum::http::header::{HeaderMap, ACCEPT_LANGUAGE, CONTENT_LANGUAGE, VARY};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::get;
use axum::Router;

use crate::locale::{resolve_language, Language, LanguageSignals, LocaleStack};
use crate::{pages, security};

/// The whole route table of the shell. Everything not listed here is a 404.
pub fn router() -> Router {
    Router::new()
        .route("/", get(hello))
        .route("/healthz", get(healthz))
        .fallback(not_found)
        .layer(axum::middleware::from_fn(security::headers))
}

/// Liveness only. Machine-facing, no version or build details.
async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Hello screen in the effective language. Anonymous for now: no user, no org, so the signals
/// are the `?lang=` share hint and the browser; stored preferences take over in P1.5.
async fn hello(RawQuery(query): RawQuery, headers: HeaderMap) -> Response {
    let stack = LocaleStack::anonymous(resolve_language(LanguageSignals {
        user: None,
        org_default: None,
        url_hint: query.as_deref().and_then(lang_hint),
        accept_language: headers.get(ACCEPT_LANGUAGE).and_then(|v| v.to_str().ok()),
    }));
    (
        [
            (CONTENT_LANGUAGE, stack.language.tag()),
            (VARY, ACCEPT_LANGUAGE.as_str()),
        ],
        Html(pages::hello(&stack)),
    )
        .into_response()
}

/// `lang=` from a raw query string. Lenient: anything unknown is simply no hint.
/// No `Query<T>` extractor, because its rejection would answer with untranslated English.
fn lang_hint(query: &str) -> Option<Language> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == "lang")
        .and_then(|(_, value)| Language::parse(value))
}

/// Fail closed: unknown routes answer with an empty 404 — no copy, no hints.
async fn not_found() -> impl IntoResponse {
    StatusCode::NOT_FOUND
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{lang_hint, router};
    use crate::locale::Language;
    use crate::security::BASELINE;

    async fn get(uri: &str) -> axum::response::Response {
        get_with(uri, &[]).await
    }

    async fn get_with(uri: &str, headers: &[(&str, &str)]) -> axum::response::Response {
        let mut request = Request::builder().uri(uri);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        router()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn body_text(response: axum::response::Response) -> String {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn healthz_reports_ok() {
        let response = get("/healthz").await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body, serde_json::json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn hello_falls_back_to_german_without_any_signal() {
        let response = get("/").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-language"], "de");
        assert_eq!(response.headers()["vary"], "accept-language");
        assert!(response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html"));
        let body = body_text(response).await;
        assert!(body.contains("<html lang=\"de\">"), "{body}");
        assert!(body.contains("<h1>Hallo.</h1>"));
    }

    #[tokio::test]
    async fn hello_follows_the_browser_language() {
        let response = get_with("/", &[("accept-language", "en-GB,en;q=0.9,de;q=0.5")]).await;
        assert_eq!(response.headers()["content-language"], "en");
        let body = body_text(response).await;
        assert!(body.contains("<html lang=\"en\">"), "{body}");
        assert!(body.contains("<h1>Hello.</h1>"));

        let response = get_with("/", &[("accept-language", "fr-FR,fr;q=0.9")]).await;
        assert_eq!(
            response.headers()["content-language"],
            "de",
            "unknown browser language → fallback"
        );
    }

    #[tokio::test]
    async fn hello_switches_via_share_hint_over_browser() {
        let response = get_with("/?lang=en", &[("accept-language", "de")]).await;
        assert_eq!(response.headers()["content-language"], "en");
        assert!(body_text(response).await.contains("<h1>Hello.</h1>"));

        let response = get_with("/?lang=de", &[("accept-language", "en")]).await;
        assert_eq!(response.headers()["content-language"], "de");

        let response = get_with("/?lang=klingon&x=1", &[("accept-language", "en")]).await;
        assert_eq!(
            response.headers()["content-language"],
            "en",
            "unknown hint is ignored"
        );
    }

    #[test]
    fn lang_hint_is_lenient() {
        assert_eq!(lang_hint("lang=en"), Some(Language::En));
        assert_eq!(lang_hint("a=1&lang=DE-at&b"), Some(Language::De));
        assert_eq!(lang_hint("lang="), None);
        assert_eq!(lang_hint("lang=fr"), None);
        assert_eq!(lang_hint("language=en"), None);
        assert_eq!(lang_hint("%zz&&="), None);
    }

    #[tokio::test]
    async fn unknown_routes_are_empty_404() {
        for uri in [
            "/login",
            "/api/overlays/123",
            "/healthz/extra",
            "/index.html",
        ] {
            let response = get(uri).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            assert!(bytes.is_empty(), "{uri} leaked a body: {bytes:?}");
        }
    }

    #[tokio::test]
    async fn every_response_carries_baseline_headers() {
        for uri in ["/", "/healthz", "/does-not-exist"] {
            let response = get(uri).await;
            for (name, value) in BASELINE {
                let got = response
                    .headers()
                    .get(&name)
                    .unwrap_or_else(|| panic!("{uri}: missing header {name}"));
                assert_eq!(got, value, "{uri}: {name}");
            }
        }
    }
}
