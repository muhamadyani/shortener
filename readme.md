# 🚀 Rust URL Shortener

A high-performance URL shortener service built with **Axum** (Web Framework) and **redb** (Embedded Key-Value Store), designed for speed and ease of deployment without external database dependencies.

## 🛠️ Key Features

- **Embedded Database**: Uses `redb` (single `.db` file), no need to install Postgres or MySQL.
- **Custom ID**: Supports custom slugs/IDs for URLs.
- **Optional Ownership Tracking**: `ref_id` is optional, allowing both public and tracked URLs.
- **Pagination & Indexing**: High-performance URL list search based on `ref_id` using index tables.
- **Graceful Shutdown**: Handles `SIGTERM` and `SIGINT` signals to maintain data integrity when the server stops.
- **Type Safety**: Input and output validation using Serde.

## 📂 API Documentation

### 1. Create Short URL

Create a new short URL.

- **URL**: `POST /api/urls`
- **Body**:
  ```json
  {
    "url": "https://google.com",
    "ref_id": "user_123", // Optional - for ownership tracking
    "custom_id": "my-link" // Optional - custom slug
  }
  ```
- **Response (201 Created)**:
  ```json
  {
    "id": "my-link",
    "short_url": "http://localhost:8080/my-link",
    "original_url": "https://google.com",
    "created_at": "2026-01-17T13:40:00Z"
  }
  ```

### 2. Redirect URL

Redirect to the original URL based on the ID.

- **URL**: `GET /{id}`
- **Response**: `307 Temporary Redirect`

### 3. List URLs (with Pagination)

Retrieve a list of URLs. If `ref_id` is provided, filters by owner. If not provided, returns all URLs.

- **URL**: `GET /api/urls?ref_id=user_123&page=1&limit=10`
- **Query Params**:
  - `ref_id` (Optional): Reference ID to filter URLs by owner. If omitted, returns all URLs.
  - `page` (Default: 1): Page number.
  - `limit` (Default: 10, Max: 100): Number of items per page.

### 4. Delete URL

Delete a link based on ID. If `ref_id` is provided, verifies ownership before deletion.

- **URL**: `DELETE /api/{id}?ref_id=user_123`
- **Query Params**:
  - `ref_id` (Optional): Reference ID for ownership verification. If omitted, deletes without verification.
- **Response (200 OK)**:
  ```json
  {
    "message": "Short link deleted successfully",
    "deleted_id": "my-link"
  }
  ```

## ⚙️ Local Setup

1. **Clone repository & install dependencies**: Ensure you have Rust & Cargo installed.
2. **Environment Configuration**: Create a `.env` file in the root folder:
   ```env
   PORT=8080
   DATABASE_URL=data.db
   ```
3. **Run Server**:
   ```bash
   cargo run
   ```

## 🚢 Deployment Guide

### Option 1: Docker (Recommended)

Use multi-stage build to produce a very small binary (~10-20MB).

1. **Create Dockerfile**:

   ```dockerfile
   # Build Stage
   FROM rust:1.75-slim as builder
   WORKDIR /app
   COPY . .
   RUN cargo build --release

   # Run Stage
   FROM debian:bookworm-slim
   WORKDIR /app
   COPY --from=builder /app/target/release/your_project_name ./server
   # Copy .env if needed, or use env vars in Docker
   EXPOSE 8080
   CMD ["./server"]
   ```

2. **Build & Run**:
   ```bash
   docker build -t rust-url-shortener .
   docker run -p 8080:8080 -v $(pwd)/data:/app -e DATABASE_URL=/app/data.db rust-url-shortener
   ```
   > **Note**: Ensure you use the `-v` volume so that the `data.db` file persists when the container is restarted.

### Option 2: Binary Deployment (VPS/Linux)

1. **Compile for Linux target locally**:
   ```bash
   cargo build --release
   ```
2. **Move the binary** from `target/release/project_name` to your VPS server.
3. **Use Systemd** to run the application as a background service:

   ```ini
   [Unit]
   Description=Rust URL Shortener Service
   After=network.target

   [Service]
   Type=simple
   User=youruser
   WorkingDirectory=/home/youruser/app
   ExecStart=/home/youruser/app/server
   Restart=always
   Environment=PORT=8080
   Environment=DATABASE_URL=data.db

   [Install]
   WantedBy=multi-user.target
   ```

## 🧪 Testing & Performance

This project includes comprehensive test suites:

### Quick Test

```bash
# Run all integration tests
cargo test

# Run with detailed output
cargo test -- --nocapture
```

### Stress Testing

```bash
# Start server
cargo run --release

# In another terminal, run stress tests
./stress_test.sh
```

### Benchmark Tests

```bash
# Run performance benchmarks
cargo test --release bench -- --ignored --nocapture
```

For detailed testing instructions, see **[TESTING.md](TESTING.md)**.

### Test Coverage

- ✅ 12 integration tests covering all API endpoints
- ✅ Benchmark tests for performance measurement
- ✅ Stress test scripts for load testing
- ✅ Support for wrk, oha, and Apache Bench

## 🛠️ Maintenance & Backup

Since this application uses `redb`, your database is a single file (e.g., `data.db`).

- **Backup**: Simply copy the `data.db` file to a secure location.
- **Integrity**: Thanks to the `shutdown_signal` implementation in `main.rs`, the database will close transactions safely when the process is stopped, preventing data corruption.
