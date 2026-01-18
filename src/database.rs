//! Database initialization and table definitions
//! 
//! This module handles the setup and configuration of the embedded redb database.
//! It defines the database tables and provides initialization functions.

use redb::{Database, TableDefinition};
use std::sync::Arc;

/// Main table for storing URL records
/// 
/// Key: Short URL ID (slug) as string
/// Value: JSON-serialized UrlRecord as string
/// 
/// Example:
/// - Key: "abc123"
/// - Value: '{"id":"abc123","original_url":"https://example.com",...}'
pub const TABLE_URLS: TableDefinition<&str, &str> = TableDefinition::new("urls_v1");

/// Index table for efficient querying by reference ID
/// 
/// This secondary index enables fast lookups and pagination of URLs belonging to a specific ref_id.
/// 
/// Key: Composite key in format "{ref_id}:{timestamp_micros}"
/// Value: JSON-serialized UrlRecord as string
/// 
/// Example:
/// - Key: "user_123:1705501234567890"
/// - Value: '{"id":"abc123","ref_id":"user_123",...}'
/// 
/// The timestamp in the key ensures chronological ordering and uniqueness.
pub const TABLE_REF_INDEX: TableDefinition<&str, &str> = TableDefinition::new("ref_index_v1");

/// Application state shared across all request handlers
/// 
/// This struct wraps the database instance in an Arc for thread-safe sharing
/// across async handlers in the Axum web framework.
#[derive(Clone)]
pub struct AppState {
    /// Thread-safe reference to the embedded database
    pub db: Arc<Database>,
}

/// Initializes the embedded database and creates required tables
/// 
/// This function:
/// 1. Creates or opens the database file at the specified path
/// 2. Opens both the main URLs table and the reference index table
/// 3. Commits the transaction to ensure tables are persisted
/// 
/// # Arguments
/// 
/// * `db_path` - File path where the database should be stored (e.g., "data.db")
/// 
/// # Returns
/// 
/// * `Ok(Database)` - Successfully initialized database instance
/// * `Err(redb::Error)` - Database initialization error
/// 
/// # Example
/// 
/// ```no_run
/// # use shortener::database::init_db;
/// let db = init_db("data.db").expect("Failed to initialize database");
/// ```
pub fn init_db(db_path: &str) -> Result<Database, redb::Error> {
    // Create or open the database file
    let db = Database::create(db_path)?;
    
    // Begin a write transaction to create tables
    let write_txn = db.begin_write()?;
    {
        // Open (or create if not exists) the main URLs table
        write_txn.open_table(TABLE_URLS)?;
        
        // Open (or create if not exists) the reference index table
        write_txn.open_table(TABLE_REF_INDEX)?;
    }
    
    // Commit the transaction to persist the table structures
    write_txn.commit()?;
    
    Ok(db)
}