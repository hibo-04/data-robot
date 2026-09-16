//! Baseline response headers — secure by default (`docs/web-app-construct.md`).
//!
//! HSTS arrives with TLS termination; the CSP loosens only when real pages exist,
//! and then explicitly per directive.

use axum::extract::Request;
use axum::http::header::{
    HeaderName, HeaderValue, CACHE_CONTROL, CONTENT_SECURITY_POLICY, REFERRER_POLICY,
    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::middleware::Next;
use axum::response::Response;

/// Headers set on every response, overriding whatever a handler produced.
pub(crate) const BASELINE: [(HeaderName, &str); 5] = [
    (X_CONTENT_TYPE_OPTIONS, "nosniff"),
    (X_FRAME_OPTIONS, "DENY"),
    (REFERRER_POLICY, "no-referrer"),
    (
        CONTENT_SECURITY_POLICY,
        "default-src 'none'; frame-ancestors 'none'",
    ),
    (CACHE_CONTROL, "no-store"),
];

pub(crate) async fn headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in BASELINE {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
    response
}
