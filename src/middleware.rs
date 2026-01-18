use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
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
) -> Result<Response, StatusCode> {
    // Check if AUTHORIZATION env var is set
    // We use var instead of var_os to ensure it's a valid unicode string
    if let Ok(auth_secret) = env::var("AUTHORIZATION") {
        // If the env var exists but is empty, we might want to skip auth or enforce empty auth?
        // The requirement says "jika di env di set authorization key maka perlu di cek"
        // usually implies if it's present.
        if !auth_secret.is_empty() {
             match headers.get("Authorization") {
                Some(header_value) => {
                    match header_value.to_str() {
                        Ok(header_str) => {
                            if header_str != auth_secret {
                                return Err(StatusCode::UNAUTHORIZED);
                            }
                        }
                        Err(_) => return Err(StatusCode::UNAUTHORIZED),
                    }
                }
                None => return Err(StatusCode::UNAUTHORIZED),
            }
        }
    }
    
    // If env var is not set or empty, or auth matches, proceed
    Ok(next.run(request).await)
}
