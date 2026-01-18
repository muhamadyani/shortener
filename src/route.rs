//! Route definitions for the URL shortener API
//! 
//! This module configures all HTTP routes and maps them to their respective handlers.
//! It creates the Axum router with the application state.

use axum::routing::{delete, get};
use axum::Router;

use crate::database::AppState;
use crate::handler::{create_short_url, delete_short_url, list_urls, redirect_url};

use axum::middleware;
use crate::middleware::auth_middleware;

/// Creates and configures the Axum application router with all routes
/// 
/// # Route Definitions
/// 
/// - `GET /{id}` - Redirects to the original URL (public endpoint)
/// - `GET /api/urls` - Lists URLs with pagination (requires ref_id query param)
/// - `POST /api/urls` - Creates a new short URL
/// - `DELETE /api/{id}` - Deletes a short URL (requires ref_id for authorization)
/// 
/// # Arguments
/// 
/// * `state` - Application state containing the shared database instance
/// 
/// # Returns
/// 
/// Configured Axum Router ready to handle requests
/// 
/// # Example Usage
/// 
/// ```no_run
/// # use std::sync::Arc;
/// # use shortener::database::{init_db, AppState};
/// # use shortener::route::create_app;
/// # let db = init_db("data.db").unwrap();
/// let state = AppState { db: Arc::new(db) };
/// let app = create_app(state);
/// // axum::serve(listener, app).await.unwrap();
/// ```
pub fn create_app(state: AppState) -> Router {
    // API routes that require authorization check
    let api_routes = Router::new()
        .route("/urls", get(list_urls).post(create_short_url))
        .route("/{id}", delete(delete_short_url))
        .layer(middleware::from_fn(auth_middleware));

    Router::new()
        // Public redirect endpoint - converts short URL to original URL
        .route("/{id}", get(redirect_url))
        // Mount API routes under /api
        .nest("/api", api_routes)
        // Inject the application state into all handlers
        .with_state(state)
}
