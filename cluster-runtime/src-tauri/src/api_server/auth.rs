//! Bearer-token auth middleware for the `/v1` routes.

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use super::ApiContext;

/// Whether an `Authorization` header value carries the expected bearer token.
pub(crate) fn token_matches(header: Option<&str>, expected: &str) -> bool {
    match header.and_then(|v| v.strip_prefix("Bearer ")).map(str::trim) {
        Some(token) => token == expected,
        None => false,
    }
}

pub async fn require_token(
    State(ctx): State<ApiContext>,
    req: Request,
    next: Next,
) -> Response {
    let header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if token_matches(header, ctx.token.as_str()) {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing or invalid bearer token" })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::token_matches;

    #[test]
    fn accepts_matching_bearer_token() {
        assert!(token_matches(Some("Bearer secret"), "secret"));
        assert!(token_matches(Some("Bearer  secret "), "secret"));
    }

    #[test]
    fn rejects_missing_or_wrong_token() {
        assert!(!token_matches(None, "secret"));
        assert!(!token_matches(Some("secret"), "secret"));
        assert!(!token_matches(Some("Bearer other"), "secret"));
        assert!(!token_matches(Some("Basic secret"), "secret"));
    }
}
