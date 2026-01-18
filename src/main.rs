//! Application entry point and server initialization
//! 
//! This module contains the main function that:
//! - Loads environment configuration
//! - Initializes the database
//! - Starts the HTTP server with graceful shutdown support

use std::sync::Arc;
use tokio::signal;
use tokio::net::TcpListener;
use dotenvy::dotenv;
use tower_http::trace::TraceLayer;
use std::env;

// Module declarations
mod database;
mod handler;
mod model;
mod route;
mod middleware;

use database::{init_db, AppState};
use route::create_app;

/// Application entry point
/// 
/// This asynchronous main function:
/// 1. Loads environment variables from .env file
/// 2. Reads configuration (PORT and DATABASE_URL)
/// 3. Initializes the embedded database
/// 4. Creates the application state and router
/// 5. Starts the HTTP server with graceful shutdown handling
/// 
/// # Environment Variables
/// 
/// - `PORT` - Server port number (default: 8080)
/// - `DATABASE_URL` - Path to database file (default: "data.db")
#[tokio::main]
async fn main() {
    // Load environment variables from .env file if it exists
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter("shortener=debug,tower_http=debug")
        .init();
    
    // Read and parse the server port from environment
    let port_str = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let port: u16 = port_str.parse().unwrap_or(8080);
    
    // Read the database file path from environment
    let db_name = env::var("DATABASE_URL").unwrap_or_else(|_| "data.db".to_string());

    // Initialize the embedded database with the specified path
    let db = init_db(&db_name).expect("Failed to initialize database");
    
    // Create application state with thread-safe database reference
    let state = AppState {
        db: Arc::new(db),
    };
    
    // Create the Axum router with all routes configured
    let app = create_app(state).layer(TraceLayer::new_for_http());
    
    // Bind to all network interfaces on the specified port
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    
    // Print startup information
    println!("🚀 Server running at http://localhost:{}", port);
    println!("📂 Using database: {}", db_name);

    // Start the server with graceful shutdown support
    // The server will continue running until it receives SIGTERM or SIGINT
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

/// Handles graceful shutdown signals
/// 
/// This function listens for shutdown signals and returns when one is received:
/// - SIGINT (Ctrl+C) - Interrupt signal from terminal
/// - SIGTERM - Termination signal (common in Docker/Kubernetes)
/// 
/// When a signal is received:
/// 1. The function returns, triggering server shutdown
/// 2. Open connections are allowed to complete
/// 3. Database transactions are properly closed
/// 4. The application exits cleanly
/// 
/// This ensures data integrity by preventing abrupt process termination
/// during active database writes.
async fn shutdown_signal() {
    // Handle Ctrl+C (SIGINT)
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };
    
    // Handle SIGTERM on Unix systems (Linux, macOS)
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };
    
    // On non-Unix systems (Windows), only handle Ctrl+C
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    // Wait for either signal to be received
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\n🛑 Shutdown signal received, stopping server.");
}