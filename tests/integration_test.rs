//! Integration tests for the URL shortener API
//! 
//! These tests verify the entire application stack including:
//! - HTTP routing
//! - Request/response handling
//! - Database operations
//! - Error handling

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::NamedTempFile;
use tower::ServiceExt;

// Import from the main crate
use shortener::database::{init_db, AppState};
use shortener::route::create_app;

/// Helper function to create a test application with a temporary database
fn setup_test_app() -> (axum::Router, NamedTempFile) {
    // Create a temporary database file
    let temp_db = NamedTempFile::new().expect("Failed to create temp file");
    let db_path = temp_db.path().to_str().unwrap();
    
    // Initialize database
    let db = init_db(db_path).expect("Failed to initialize test database");
    let state = AppState {
        db: Arc::new(db),
    };
    
    // Create the app
    let app = create_app(state);
    
    (app, temp_db)
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
async fn test_create_short_url_success() {
    let (app, _temp_db) = setup_test_app();
    
    // Create request payload
    let payload = json!({
        "url": "https://example.com/test",
        "ref_id": "test_user",
        "custom_id": "test123"
    });
    
    // Send POST request
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
    
    // Verify response
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let body = response_json(response.into_body()).await;
    assert_eq!(body["id"], "test123");
    assert_eq!(body["original_url"], "https://example.com/test");
    assert!(body["short_url"].as_str().unwrap().contains("test123"));
}

#[tokio::test]
async fn test_create_short_url_without_ref_id() {
    let (app, _temp_db) = setup_test_app();
    
    // Create request without ref_id
    let payload = json!({
        "url": "https://example.com/public"
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
    
    let body = response_json(response.into_body()).await;
    assert_eq!(body["original_url"], "https://example.com/public");
    assert!(body["id"].as_str().unwrap().len() == 6); // Random 6-char ID
}

#[tokio::test]
async fn test_create_short_url_duplicate_custom_id() {
    let (app, _temp_db) = setup_test_app();
    
    let payload = json!({
        "url": "https://example.com/first",
        "custom_id": "duplicate"
    });
    
    // First creation should succeed
    let response = app
        .clone()
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
    
    // Second creation with same ID should fail
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
    
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_redirect_url_success() {
    let (app, _temp_db) = setup_test_app();
    
    // First, create a short URL
    let create_payload = json!({
        "url": "https://example.com/redirect-test",
        "custom_id": "redirect123"
    });
    
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/urls")
                .header("content-type", "application/json")
                .body(Body::from(create_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    // Now test the redirect
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/redirect123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "https://example.com/redirect-test"
    );
}

#[tokio::test]
async fn test_redirect_url_not_found() {
    let (app, _temp_db) = setup_test_app();
    
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_urls_with_ref_id() {
    let (app, _temp_db) = setup_test_app();
    
    // Create multiple URLs with the same ref_id
    for i in 1..=3 {
        let payload = json!({
            "url": format!("https://example.com/url{}", i),
            "ref_id": "list_test_user",
            "custom_id": format!("list{}", i)
        });
        
        app.clone()
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
    }
    
    // List URLs for this ref_id
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/urls?ref_id=list_test_user&page=1&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response_json(response.into_body()).await;
    assert_eq!(body["total_fetched"], 3);
    assert_eq!(body["page"], 1);
    assert_eq!(body["limit"], 10);
}

#[tokio::test]
async fn test_list_urls_without_ref_id() {
    let (app, _temp_db) = setup_test_app();
    
    // Create URLs
    for i in 1..=5 {
        let payload = json!({
            "url": format!("https://example.com/all{}", i)
        });
        
        app.clone()
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
    }
    
    // List all URLs
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/urls?page=1&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response_json(response.into_body()).await;
    assert_eq!(body["total_fetched"], 5);
}

#[tokio::test]
async fn test_list_urls_pagination() {
    let (app, _temp_db) = setup_test_app();
    
    // Create 15 URLs
    for i in 1..=15 {
        let payload = json!({
            "url": format!("https://example.com/page{}", i),
            "ref_id": "pagination_user"
        });
        
        app.clone()
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
    }
    
    // Get first page
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/urls?ref_id=pagination_user&page=1&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    let body = response_json(response.into_body()).await;
    assert_eq!(body["total_fetched"], 10);
    assert_eq!(body["page"], 1);
    
    // Get second page
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/urls?ref_id=pagination_user&page=2&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    let body = response_json(response.into_body()).await;
    assert_eq!(body["total_fetched"], 5);
    assert_eq!(body["page"], 2);
}

#[tokio::test]
async fn test_delete_url_with_ref_id_success() {
    let (app, _temp_db) = setup_test_app();
    
    // Create a URL
    let payload = json!({
        "url": "https://example.com/delete-test",
        "ref_id": "delete_user",
        "custom_id": "delete123"
    });
    
    app.clone()
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
    
    // Delete with correct ref_id
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/delete123?ref_id=delete_user")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response_json(response.into_body()).await;
    assert_eq!(body["deleted_id"], "delete123");
}

#[tokio::test]
async fn test_delete_url_without_ref_id() {
    let (app, _temp_db) = setup_test_app();
    
    // Create a URL without ref_id
    let payload = json!({
        "url": "https://example.com/public-delete",
        "custom_id": "public123"
    });
    
    app.clone()
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
    
    // Delete without ref_id verification
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/public123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_delete_url_wrong_ref_id() {
    let (app, _temp_db) = setup_test_app();
    
    // Create a URL
    let payload = json!({
        "url": "https://example.com/protected",
        "ref_id": "owner123",
        "custom_id": "protected123"
    });
    
    app.clone()
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
    
    // Try to delete with wrong ref_id
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/protected123?ref_id=wrong_user")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_url_not_found() {
    let (app, _temp_db) = setup_test_app();
    
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
