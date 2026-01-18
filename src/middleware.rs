use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::env;

/// Middleware to check for Authorization header
/// 
/// This middleware checks if the `AUTHORIZATION` environment variable is set.
/// If it is set, it verifies that the request contains an `Authorization` header
/// with the matching value.
/// 
/// If the environment variable is not set, the check is skipped.
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Check if AUTHORIZATION env var is set
    // We use var instead of var_os to ensure it's a valid unicode string
    if let Ok(auth_secret) = env::var("AUTHORIZATION") {
        // If the env var exists but is empty, we might want to skip auth or enforce empty auth?
        // The requirement says "jika di env di set authorization key maka perlu di cek"
        // usually implies if it's present.
        if !auth_secret.is_empty() {
            let unauthorized_response = || {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "Unauthorized",
                        "message": "Invalid or missing authorization header"
                    })),
                ).into_response()
            };

             match headers.get("Authorization") {
                Some(header_value) => {
                    match header_value.to_str() {
                        Ok(header_str) => {
                            if header_str != auth_secret {
                                return Err(unauthorized_response());
                            }
                        }
                        Err(_) => return Err(unauthorized_response()),
                    }
                }
                None => return Err(unauthorized_response()),
            }
        }
    }
    
    // If env var is not set or empty, or auth matches, proceed
    Ok(next.run(request).await)
}
