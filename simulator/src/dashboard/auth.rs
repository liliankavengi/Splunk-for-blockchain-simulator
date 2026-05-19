use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Axum middleware that validates `Authorization: Bearer <token>`.
/// The expected token is captured via closure in `server.rs`.
pub async fn bearer_auth(
    expected: String,
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let provided = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let ok = matches!(provided, Some(ref s) if *s == format!("Bearer {}", expected));
    if ok {
        Ok(next.run(request).await)
    } else {
        Err((StatusCode::UNAUTHORIZED, "Unauthorized — provide Authorization: Bearer <password>"))
    }
}
