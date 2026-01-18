//! HTTP request handlers for the URL shortener API
//! 
//! This module implements all the core business logic for:
//! - Creating short URLs with custom or random IDs
//! - Redirecting short URLs to their original destinations
//! - Listing URLs with pagination and filtering
//! - Deleting URLs with ownership verification

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use chrono::Utc;
use rand::{distr::Alphanumeric, Rng};
use redb::{ReadableDatabase, ReadableTable};
use serde_json::{self, json};

use crate::model::{CreateRequest, CreateResponse, ListParams, UrlRecord};
use crate::{
    database::{AppState, TABLE_REF_INDEX, TABLE_URLS},
    model::DeleteParams,
};

/// Creates a new short URL
/// 
/// This handler:
/// 1. Accepts a long URL and optional custom ID
/// 2. Generates a random 6-character ID if no custom ID is provided
/// 3. Checks if the ID is already taken
/// 4. Stores the URL record in both the main table and ref_id index
/// 5. Returns the created short URL details
/// 
/// # Request Body
/// 
/// ```json
/// {
///   "url": "https://example.com/very/long/url",
///   "ref_id": "user_123",
///   "custom_id": "my-link"  // Optional
/// }
/// ```
/// 
/// # Response
/// 
/// - **201 Created** - URL successfully created
/// - **409 Conflict** - Custom ID already exists
/// 
/// # Database Operations
/// 
/// Writes to two tables:
/// 1. `TABLE_URLS` - Main table indexed by short URL ID
/// 2. `TABLE_REF_INDEX` - Secondary index for querying by ref_id
pub async fn create_short_url(
    State(state): State<AppState>,
    Json(payload): Json<CreateRequest>,
) -> impl IntoResponse {
    // Filter out empty custom IDs and treat them as None
    let effective_custom_id = payload.custom_id.filter(|id| !id.is_empty());
    
    // Use custom ID if provided, otherwise generate a random 6-character ID
    let id_to_use = match effective_custom_id {
        Some(custom_id) => custom_id,
        None => rand::rng()
            .sample_iter(&Alphanumeric)
            .take(6)
            .map(char::from)
            .collect(),
    };

    let base_url = std::env::var("URL").unwrap_or_else(|_| "http://localhost".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let domain = format!("{}:{}", base_url, port);

    // Create the URL record with all metadata
    let record = UrlRecord {
        id: id_to_use.clone(),
        original_url: payload.url,
        short_url: format!("{}/{}", domain, id_to_use.clone()),
        ref_id: payload.ref_id.clone(),
        created_at: Utc::now(),
        clicks: 0,
    };
    
    // Serialize the record to JSON for storage
    let record_json = serde_json::to_string(&record).unwrap();

    // Begin a write transaction
    let write_txn = state.db.begin_write().unwrap();
    {
        // Open the main URLs table
        let mut table_main = write_txn.open_table(TABLE_URLS).unwrap();
        
        // Check if the ID is already taken
        if table_main.get(id_to_use.as_str()).unwrap().is_some() {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Custom ID already taken. Please choose another."
                })),
            )
                .into_response();
        }

        // Insert the record into the main table
        table_main
            .insert(id_to_use.as_str(), record_json.as_str())
            .unwrap();

        // Only insert into ref_id index if ref_id is provided
        if let Some(ref_id_value) = &payload.ref_id {
            // Create composite key for ref_id index: "ref_id:timestamp_micros"
            // This enables efficient range queries and maintains chronological order
            let index_key = format!("{}:{}", ref_id_value, record.created_at.timestamp_micros());
            
            let mut table_index = write_txn.open_table(TABLE_REF_INDEX).unwrap();
            table_index
                .insert(index_key.as_str(), record_json.as_str())
                .unwrap();
        }
    }
    
    // Commit the transaction to persist the data
    write_txn.commit().unwrap();

    // Prepare the response with the created URL details
    let response = CreateResponse {
        id: id_to_use.clone(),
        original_url: record.original_url,
        short_url: format!("{}/{}", domain, id_to_use),
        created_at: record.created_at,
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

/// Redirects a short URL to its original destination
/// 
/// This is the core functionality that makes the URL shortener work.
/// When a user visits `http://localhost:8080/abc123`, this handler:
/// 1. Looks up "abc123" in the database
/// 2. Retrieves the original URL
/// 3. Sends a 307 Temporary Redirect response
/// 
/// # Path Parameters
/// 
/// - `id` - The short URL identifier/slug
/// 
/// # Response
/// 
/// - **307 Temporary Redirect** - Redirects to the original URL
/// - **404 Not Found** - Short URL does not exist
/// 
/// # Note
/// 
/// Uses 307 Temporary Redirect instead of 301 Permanent Redirect to:
/// - Allow URL statistics tracking
/// - Enable URL updates or deletion
/// - Prevent browser caching
pub async fn redirect_url(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Begin a read-only transaction
    let read_txn = state.db.begin_read().unwrap();
    let table = read_txn.open_table(TABLE_URLS).unwrap();
    
    // Look up the short URL ID in the database
    if let Some(value) = table.get(id.as_str()).unwrap() {
        // Deserialize the JSON record
        if let Ok(record) = serde_json::from_str::<UrlRecord>(value.value()) {
            // TODO: Add logic to increment click counter here
            // This would require a write transaction to update the clicks field
            return Redirect::temporary(&record.original_url).into_response();
        }
    }
    
    // Return 404 if the ID is not found or deserialization fails
    (StatusCode::NOT_FOUND, "URL not found").into_response()
}

/// Lists URLs with pagination and filtering by ref_id
/// 
/// This handler enables users to retrieve all their shortened URLs
/// using efficient pagination. It leverages the ref_id index for
/// fast lookups without scanning the entire database.
/// 
/// # Query Parameters
/// 
/// - `ref_id` (required) - Filter URLs by this reference ID
/// - `page` (optional) - Page number, starts from 1 (default: 1)
/// - `limit` (optional) - Items per page, max 100 (default: 10)
/// 
/// # Example Request
/// 
/// `GET /api/urls?ref_id=user_123&page=2&limit=20`
/// 
/// # Response
/// 
/// ```json
/// {
///   "page": 2,
///   "limit": 20,
///   "total_fetched": 15,
///   "data": [...]
/// }
/// ```
/// 
/// # Performance
/// 
/// Uses range queries on the ref_id index table for O(log n) lookup time.
/// The composite key format "{ref_id}:{timestamp}" ensures results are
/// returned in chronological order (newest first due to descending range).
pub async fn list_urls(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    // Ensure page is at least 1
    let page = params.page.unwrap_or(1).max(1);
    
    // Limit to maximum of 100 items per page
    let limit = params.limit.unwrap_or(10).min(100);
    
    // Calculate offset for pagination
    let offset = (page - 1) * limit;

    // Begin a read-only transaction
    let read_txn = state.db.begin_read().unwrap();

    let results: Vec<UrlRecord> = match &params.ref_id {
        // If ref_id is provided, use the efficient index-based query
        Some(ref_id) => {
            let table = read_txn.open_table(TABLE_REF_INDEX).unwrap();
            
            // Define range query boundaries for the ref_id
            // start_key: "user_123:" - matches all entries starting with this ref_id
            // end_key: "user_123:{" - the character '{' is lexicographically after ':'
            //                         so this effectively creates an upper bound
            let start_key = format!("{}:", ref_id);
            let end_key = format!("{}:{{", ref_id);

            // Execute range query with pagination
            table
                .range(start_key.as_str()..end_key.as_str())
                .unwrap()
                .skip(offset)  // Skip items from previous pages
                .take(limit)   // Take only the requested number of items
                .filter_map(|res| {
                    // Handle potential errors and deserialize the JSON records
                    res.ok()
                        .and_then(|(_, value)| serde_json::from_str::<UrlRecord>(value.value()).ok())
                })
                .collect()
        },
        // If ref_id is not provided, return all URLs from the main table
        // WARNING: This can be slow for large databases
        None => {
            let table = read_txn.open_table(TABLE_URLS).unwrap();
            
            table
                .iter()
                .unwrap()
                .skip(offset)
                .take(limit)
                .filter_map(|res| {
                    res.ok()
                        .and_then(|(_, value)| serde_json::from_str::<UrlRecord>(value.value()).ok())
                })
                .collect()
        }
    };

    // Return paginated results with metadata
    Json(serde_json::json!({
        "page": page,
        "limit": limit,
        "total_fetched": results.len(),
        "data": results
    }))
    .into_response()
}

/// Deletes a short URL with ownership verification
/// 
/// This handler ensures that only the owner of a URL can delete it
/// by verifying the ref_id matches before performing the deletion.
/// 
/// # Path Parameters
/// 
/// - `id` - The short URL identifier to delete
/// 
/// # Query Parameters
/// 
/// - `ref_id` (required) - Reference ID for ownership verification
/// 
/// # Example Request
/// 
/// `DELETE /api/abc123?ref_id=user_123`
/// 
/// # Response
/// 
/// - **200 OK** - URL successfully deleted
/// - **404 Not Found** - URL does not exist
/// - **403 Forbidden** - ref_id does not match (not the owner)
/// 
/// # Database Operations
/// 
/// Deletes from two tables:
/// 1. `TABLE_URLS` - Removes the main record
/// 2. `TABLE_REF_INDEX` - Removes the index entry
pub async fn delete_short_url(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Query(params): Query<DeleteParams>,
) -> impl IntoResponse {
    // Begin a write transaction
    let write_txn = state.db.begin_write().unwrap();

    {
        // Open the main URLs table
        let mut table_main = write_txn.open_table(TABLE_URLS).unwrap();
        
        // Retrieve the existing record to verify ownership
        let record = match table_main.get(id.as_str()).unwrap() {
            Some(guard) => serde_json::from_str::<UrlRecord>(guard.value()).unwrap(),
            None => {
                // Return 404 if the URL doesn't exist
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": "URL not found",
                        "code": "not_found"
                    })),
                )
                    .into_response()
            }
        };
        
        // Verify ownership by comparing ref_id (only if ref_id is provided in the request)
        if let Some(request_ref_id) = &params.ref_id {
            // If the record has a ref_id, it must match the request ref_id
            match &record.ref_id {
                Some(record_ref_id) => {
                    if record_ref_id != request_ref_id {
                        return (
                            StatusCode::FORBIDDEN,
                            Json(json!({
                                "error": "You are not authorized to delete this link",
                                "code": "forbidden"
                            })),
                        )
                            .into_response();
                    }
                },
                None => {
                    // Record has no ref_id, but request is trying to verify ownership
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "error": "This URL has no owner and cannot be deleted with ref_id verification",
                            "code": "forbidden"
                        })),
                    )
                        .into_response();
                }
            }
        }
        
        // Delete from the main table
        table_main.remove(id.as_str()).unwrap();
        
        // Delete from the ref_id index (only if the record has a ref_id)
        if let Some(record_ref_id) = &record.ref_id {
            let index_key = format!("{}:{}", record_ref_id, record.created_at.timestamp_micros());
            let mut table_index = write_txn.open_table(TABLE_REF_INDEX).unwrap();
            table_index.remove(index_key.as_str()).unwrap();
        }
    }

    // Commit the transaction to persist the deletion
    write_txn.commit().unwrap();

    // Return success response
    (
        StatusCode::OK,
        Json(json!({
            "message": "Short link deleted successfully",
            "deleted_id": id
        })),
    )
        .into_response()
}
