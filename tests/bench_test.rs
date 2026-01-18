//! Benchmark tests for critical operations
//! 
//! Run with: cargo test --release -- --nocapture bench

use std::sync::Arc;
use std::time::Instant;
use tempfile::NamedTempFile;

use shortener::database::{init_db, AppState};
use shortener::model::{CreateRequest, ListParams};
use shortener::handler::{create_short_url, list_urls};

use axum::{
    extract::{Query, State},
    Json,
};

/// Benchmark helper to measure execution time
fn benchmark<F>(name: &str, iterations: usize, mut f: F) 
where
    F: FnMut(),
{
    let start = Instant::now();
    
    for _ in 0..iterations {
        f();
    }
    
    let duration = start.elapsed();
    let avg_ms = duration.as_millis() as f64 / iterations as f64;
    let ops_per_sec = (iterations as f64 / duration.as_secs_f64()) as u64;
    
    println!("  {} ({} iterations)", name, iterations);
    println!("    Total time: {:?}", duration);
    println!("    Avg time: {:.3}ms", avg_ms);
    println!("    Throughput: {} ops/sec\n", ops_per_sec);
}

#[tokio::test]
#[ignore] // Run explicitly with: cargo test bench --release -- --ignored --nocapture
async fn bench_create_urls() {
    println!("\n=== Benchmark: Create URLs ===\n");
    
    let temp_db = NamedTempFile::new().unwrap();
    let db = init_db(temp_db.path().to_str().unwrap()).unwrap();
    let state = AppState { db: Arc::new(db) };
    
    // Benchmark with ref_id
    let iterations = 1000;
    benchmark("Create with ref_id", iterations, || {
        let state_clone = state.clone();
        let req = CreateRequest {
            url: "https://example.com/bench".to_string(),
            ref_id: Some("bench_user".to_string()),
            custom_id: None,
        };
        
        tokio::runtime::Handle::current().block_on(async {
            let _ = create_short_url(State(state_clone), Json(req)).await;
        });
    });
    
    // Benchmark without ref_id
    benchmark("Create without ref_id", iterations, || {
        let state_clone = state.clone();
        let req = CreateRequest {
            url: "https://example.com/public".to_string(),
            ref_id: None,
            custom_id: None,
        };
        
        tokio::runtime::Handle::current().block_on(async {
            let _ = create_short_url(State(state_clone), Json(req)).await;
        });
    });
}

#[tokio::test]
#[ignore]
async fn bench_list_urls() {
    println!("\n=== Benchmark: List URLs ===\n");
    
    let temp_db = NamedTempFile::new().unwrap();
    let db = init_db(temp_db.path().to_str().unwrap()).unwrap();
    let state = AppState { db: Arc::new(db) };
    
    // Create 1000 URLs first
    println!("  Preparing: Creating 1000 URLs...");
    for i in 0..1000 {
        let req = CreateRequest {
            url: format!("https://example.com/list{}", i),
            ref_id: Some("list_bench_user".to_string()),
            custom_id: None,
        };
        create_short_url(State(state.clone()), Json(req)).await;
    }
    println!("  Done!\n");
    
    // Benchmark list with ref_id (indexed query)
    let iterations = 1000;
    benchmark("List with ref_id (indexed)", iterations, || {
        let state_clone = state.clone();
        let params = ListParams {
            ref_id: Some("list_bench_user".to_string()),
            page: Some(1),
            limit: Some(10),
        };
        
        tokio::runtime::Handle::current().block_on(async {
            let _ = list_urls(State(state_clone), Query(params)).await;
        });
    });
    
    // Benchmark list without ref_id (full scan)
    benchmark("List without ref_id (full scan)", iterations, || {
        let state_clone = state.clone();
        let params = ListParams {
            ref_id: None,
            page: Some(1),
            limit: Some(10),
        };
        
        tokio::runtime::Handle::current().block_on(async {
            let _ = list_urls(State(state_clone), Query(params)).await;
        });
    });
}

#[tokio::test]
#[ignore]
async fn bench_database_scaling() {
    println!("\n=== Benchmark: Database Scaling ===\n");
    
    let temp_db = NamedTempFile::new().unwrap();
    let db = init_db(temp_db.path().to_str().unwrap()).unwrap();
    let state = AppState { db: Arc::new(db) };
    
    // Test performance at different database sizes
    let sizes = [100, 1000, 10000, 50000];
    
    for &size in &sizes {
        println!("  Testing with {} URLs in database...", size);
        
        // Fill database
        let start = Instant::now();
        for i in 0..size {
            let req = CreateRequest {
                url: format!("https://example.com/scale{}", i),
                ref_id: Some("scale_user".to_string()),
                custom_id: None,
            };
            create_short_url(State(state.clone()), Json(req)).await;
        }
        let fill_time = start.elapsed();
        println!("    Fill time: {:?}", fill_time);
        
        // Measure query performance
        let start = Instant::now();
        let params = ListParams {
            ref_id: Some("scale_user".to_string()),
            page: Some(1),
            limit: Some(10),
        };
        list_urls(State(state.clone()), Query(params)).await;
        let query_time = start.elapsed();
        println!("    Query time: {:?}", query_time);
        println!();
    }
}

#[tokio::test]
#[ignore]
async fn bench_concurrent_operations() {
    println!("\n=== Benchmark: Concurrent Operations ===\n");
    
    let temp_db = NamedTempFile::new().unwrap();
    let db = init_db(temp_db.path().to_str().unwrap()).unwrap();
    let state = Arc::new(AppState { db: Arc::new(db) });
    
    let num_tasks = 100;
    let ops_per_task = 10;
    
    println!("  Running {} concurrent tasks with {} ops each...", num_tasks, ops_per_task);
    
    let start = Instant::now();
    
    let mut handles = vec![];
    
    for task_id in 0..num_tasks {
        let state_clone = state.clone();
        
        let handle = tokio::spawn(async move {
            for op_id in 0..ops_per_task {
                let req = CreateRequest {
                    url: format!("https://example.com/concurrent-{}-{}", task_id, op_id),
                    ref_id: Some(format!("user_{}", task_id)),
                    custom_id: None,
                };
                create_short_url(State(state_clone.as_ref().clone()), Json(req)).await;
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
    
    let duration = start.elapsed();
    let total_ops = num_tasks * ops_per_task;
    let ops_per_sec = total_ops as f64 / duration.as_secs_f64();
    
    println!("  Total operations: {}", total_ops);
    println!("  Total time: {:?}", duration);
    println!("  Throughput: {:.0} ops/sec\n", ops_per_sec);
}

#[test]
fn bench_summary() {
    println!("\n{}", "=".repeat(60));
    println!("Benchmark Test Suite");
    println!("{}", "=".repeat(60));
    println!("\nTo run benchmarks, use:");
    println!("  cargo test --release bench -- --ignored --nocapture");
    println!("\nAvailable benchmarks:");
    println!("  • bench_create_urls         - URL creation performance");
    println!("  • bench_list_urls           - Query performance with/without index");
    println!("  • bench_database_scaling    - Performance at different DB sizes");
    println!("  • bench_concurrent_operations - Concurrent access patterns");
    println!("\n{}\n", "=".repeat(60));
}
