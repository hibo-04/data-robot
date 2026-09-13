use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

use crate::html;

pub struct WebError {
    pub status: StatusCode,
    pub message: String,
}

impl WebError {
    pub fn bad(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub fn from_analytics(err: analytics::AnalyticsError) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: err.to_string(),
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let body = html::error_page(self.status.as_u16(), &self.message);
        (self.status, Html(body)).into_response()
    }
}
