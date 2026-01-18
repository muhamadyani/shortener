//! Data models for the URL shortener application
//! 
//! This module defines all the data structures used throughout the application,
//! including request/response models and database record structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a URL record stored in the database
/// 
/// This structure contains all information about a shortened URL including:
/// - The unique identifier (slug)
/// - Original and shortened URLs
/// - Reference ID for ownership tracking
/// - Creation timestamp
/// - Click tracking counter
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UrlRecord {
    /// Unique identifier/slug for the shortened URL (e.g., "abc123" or custom ID)
    pub id: String,
    
    /// The original long URL that was shortened
    pub original_url: String,
    
    /// The complete shortened URL (e.g., "http://localhost:8080/abc123")
    pub short_url: String,
    
    /// Reference ID to identify the owner of this URL
    /// Used for authorization and filtering URLs by user/owner
    /// Optional - if not provided, the URL is publicly accessible without owner tracking
    pub ref_id: Option<String>,
    
    /// Timestamp when this URL record was created
    pub created_at: DateTime<Utc>,
    
    /// Number of times this short URL has been accessed
    /// Defaults to 0 if not present during deserialization
    #[serde(default)]
    pub clicks: u64,
}

/// Request payload for creating a new short URL
/// 
/// # Example
/// ```json
/// {
///   "url": "https://example.com/very/long/url",
///   "ref_id": "user_123",
///   "custom_id": "my-link"  // Optional
/// }
/// ```
#[derive(Deserialize)]
pub struct CreateRequest {
    /// The original URL to be shortened
    pub url: String,
    
    /// Optional reference ID to identify the owner of this URL
    /// If not provided, the URL will be created without owner tracking
    pub ref_id: Option<String>,
    
    /// Optional custom slug/ID for the shortened URL
    /// If not provided, a random 6-character ID will be generated
    pub custom_id: Option<String>,
}

/// Response returned after successfully creating a short URL
/// 
/// # Example
/// ```json
/// {
///   "id": "abc123",
///   "short_url": "http://localhost:8080/abc123",
///   "original_url": "https://example.com/very/long/url",
///   "created_at": "2026-01-17T13:40:00Z"
/// }
/// ```
#[derive(Serialize)]
pub struct CreateResponse {
    /// The unique identifier/slug of the created short URL
    pub id: String,
    
    /// The complete shortened URL
    pub short_url: String,
    
    /// The original URL that was shortened
    pub original_url: String,
    
    /// Timestamp when the URL was created
    pub created_at: DateTime<Utc>,
}

/// Query parameters for listing URLs with pagination
/// 
/// # Example
/// Query string: `?ref_id=user_123&page=2&limit=20`
#[derive(Deserialize)]
pub struct ListParams {
    /// Optional reference ID to filter URLs by owner
    /// If not provided, returns all URLs (use with caution in production)
    pub ref_id: Option<String>,
    
    /// Page number for pagination (starts from 1)
    /// Defaults to 1 if not provided
    pub page: Option<usize>,
    
    /// Number of items per page
    /// Defaults to 10 if not provided, maximum is 100
    pub limit: Option<usize>,
}

/// Query parameters for deleting a URL
/// 
/// Used to verify ownership before deletion
#[derive(serde::Deserialize)]
pub struct DeleteParams {
    /// Optional reference ID to verify that the requester owns this URL
    /// If not provided, deletion is allowed without ownership verification (use with caution)
    pub ref_id: Option<String>,
}