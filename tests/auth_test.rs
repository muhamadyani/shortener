use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;
use tower::ServiceExt;

use shortener::database::{init_db, AppState};
use shortener::route::create_app;

// Mutex to ensure tests that modify env vars don't run in parallel
static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn setup_test_app() -> (axum::Router, NamedTempFile) {
    let temp_db = NamedTempFile::new().expect("Failed to create temp file");
    let db_path = temp_db.path().to_str().unwrap();
    let db = init_db(db_path).expect("Failed to initialize test database");
    let state = AppState {
        db: Arc::new(db),
    };
    (create_app(state), temp_db)
}

/// Helper function to parse response body as JSON
async fn response_json(body: Body) -> Value {
    let bytes = body
        .collect()
        .await
        .expect("Failed to read response body")
        .to_bytes();
    
    serde_json::from_slice(&bytes).expect("Failed to parse JSON")
}

#[tokio::test]
async fn test_auth_middleware_enabled_valid_token() {
    let _guard = ENV_MUTEX.lock().unwrap();
    env::set_var("AUTHORIZATION", "secret_token");
    
    let (app, _temp_db) = setup_test_app();
    
    let payload = json!({
        "url": "https://example.com/auth-test-1"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/urls")
                .header("content-type", "application/json")
                .header("Authorization", "secret_token")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::CREATED);
    
    env::remove_var("AUTHORIZATION");
}

#[tokio::test]
async fn test_auth_middleware_enabled_invalid_token() {
    let _guard = ENV_MUTEX.lock().unwrap();
    env::set_var("AUTHORIZATION", "secret_token");
    
    let (app, _temp_db) = setup_test_app();
    
    let payload = json!({
        "url": "https://example.com/auth-test-2"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/urls")
                .header("content-type", "application/json")
                .header("Authorization", "wrong_token")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    
    let body = response_json(response.into_body()).await;
    assert_eq!(body["error"], "Unauthorized");
    assert_eq!(body["message"], "Invalid or missing authorization header");
    
    env::remove_var("AUTHORIZATION");
}

#[tokio::test]
async fn test_auth_middleware_enabled_no_token() {
    let _guard = ENV_MUTEX.lock().unwrap();
    env::set_var("AUTHORIZATION", "secret_token");
    
    let (app, _temp_db) = setup_test_app();
    
    let payload = json!({
        "url": "https://example.com/auth-test-3"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/urls")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    
    let body = response_json(response.into_body()).await;
    assert_eq!(body["error"], "Unauthorized");
    assert_eq!(body["message"], "Invalid or missing authorization header");
    
    env::remove_var("AUTHORIZATION");
}

#[tokio::test]
async fn test_auth_middleware_disabled() {
    let _guard = ENV_MUTEX.lock().unwrap();
    env::remove_var("AUTHORIZATION");
    
    let (app, _temp_db) = setup_test_app();
    
    let payload = json!({
        "url": "https://example.com/auth-test-4"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/urls")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::CREATED);
}
