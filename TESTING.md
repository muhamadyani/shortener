# Testing Guide

This document describes how to run tests and benchmarks for the URL Shortener application.

## 📋 Test Suite Overview

The project includes three types of tests:

1. **Integration Tests** - End-to-end API testing
2. **Benchmark Tests** - Performance measurement
3. **Stress Tests** - Load and concurrency testing

---

## 🧪 Integration Tests

Integration tests verify that all API endpoints work correctly.

### Run All Integration Tests

```bash
cargo test
```

### Run Specific Test

```bash
cargo test test_create_short_url_success
```

### Run with Output

```bash
cargo test -- --nocapture
```

### Test Coverage

The integration tests cover:

- ✅ Creating URLs with/without ref_id
- ✅ Creating URLs with custom IDs
- ✅ Duplicate ID handling
- ✅ URL redirection (307 Temporary Redirect)
- ✅ Listing URLs with pagination
- ✅ Listing URLs with/without ref_id filtering
- ✅ Deleting URLs with ownership verification
- ✅ Deleting URLs without ref_id
- ✅ Error handling (404, 403, 409)

---

## ⚡ Benchmark Tests

Benchmark tests measure performance of critical operations.

### Run All Benchmarks

```bash
cargo test --release bench -- --ignored --nocapture
```

### Run Specific Benchmark

```bash
# URL creation performance
cargo test --release bench_create_urls -- --ignored --nocapture

# Query performance
cargo test --release bench_list_urls -- --ignored --nocapture

# Database scaling
cargo test --release bench_database_scaling -- --ignored --nocapture

# Concurrent operations
cargo test --release bench_concurrent_operations -- --ignored --nocapture
```

### Benchmark Coverage

- **bench_create_urls**: Measures URL creation speed with/without ref_id
- **bench_list_urls**: Compares indexed vs full-scan query performance
- **bench_database_scaling**: Tests performance at 100, 1K, 10K, 50K URLs
- **bench_concurrent_operations**: Tests concurrent create operations

### Expected Performance

On a modern machine (M1/M2 Mac or equivalent):

- Create operations: 10,000+ ops/sec
- Indexed queries: 50,000+ ops/sec
- Full table scan: 1,000+ ops/sec

---

## 🔥 Stress Tests

Stress tests simulate real-world load scenarios.

### Prerequisites

Install at least one load testing tool:

```bash
# Option 1: wrk (recommended)
brew install wrk  # macOS
apt-get install wrk  # Linux

# Option 2: oha (Rust-based)
cargo install oha

# Option 3: Apache Bench (usually pre-installed)
which ab
```

### Run Stress Tests

1. **Start the server in release mode**:

   ```bash
   cargo run --release
   ```

2. **In another terminal, run the stress test**:
   ```bash
   ./stress_test.sh
   ```

### Custom Configuration

You can configure the stress test with environment variables:

```bash
# Custom settings
BASE_URL=http://localhost:8080 \
DURATION=60s \
CONNECTIONS=200 \
THREADS=8 \
./stress_test.sh

# Quick test (10 seconds, 50 connections)
DURATION=10s CONNECTIONS=50 ./stress_test.sh
```

### Stress Test Scenarios

The script includes 5 test scenarios:

1. **Redirect Performance** - Tests GET /{id} endpoint
2. **Create URL Performance** - Tests POST /api/urls endpoint
3. **List URLs Performance** - Tests GET /api/urls with pagination
4. **Mixed Load Test** - Concurrent read/write operations
5. **Database Growth Test** - Creates 10,000 URLs to test scaling

### Expected Results

On a modern machine (M1/M2 Mac):

- Redirect operations: 20,000+ req/sec
- Create operations: 5,000+ req/sec
- List operations: 15,000+ req/sec

---

## 📊 Monitoring During Tests

### Monitor System Resources

```bash
# Terminal 1: Run the server
cargo run --release

# Terminal 2: Monitor resources
htop  # or Activity Monitor on macOS

# Terminal 3: Run tests
./stress_test.sh
```

### Monitor Database Size

```bash
# Check database file size during stress test
watch -n 1 'ls -lh data.db'
```

### Check for Memory Leaks

```bash
# macOS
leaks --atExit -- cargo run --release

# Linux with valgrind
valgrind --leak-check=full cargo run --release
```

---

## 🎯 Performance Tuning Tips

### 1. Use Release Build

Always use `--release` for performance testing:

```bash
cargo run --release
cargo test --release
```

### 2. Optimize Database Location

For best performance, store the database on an SSD:

```bash
DATABASE_URL=/path/to/fast/ssd/data.db cargo run --release
```

### 3. Adjust File Descriptors (Unix/Linux)

If you get "too many open files" errors:

```bash
ulimit -n 10000
```

### 4. Monitor Disk I/O

Check if disk I/O is a bottleneck:

```bash
# macOS
sudo fs_usage -f filesys | grep data.db

# Linux
iotop
```

---

## 🐛 Troubleshooting

### Tests Fail to Compile

```bash
# Clean and rebuild
cargo clean
cargo build
cargo test
```

### Stress Test Can't Connect

```bash
# Verify server is running
curl http://localhost:8080/api/urls?page=1&limit=1

# Check the port
lsof -i :8080
```

### Out of Memory During Stress Test

Reduce concurrent connections:

```bash
CONNECTIONS=50 ./stress_test.sh
```

### Database Lock Errors

This usually indicates too many concurrent writes. The embedded database handles this automatically with retries, but you can reduce concurrency if needed.

---

## 📈 CI/CD Integration

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v2

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run tests
        run: cargo test

      - name: Run benchmarks
        run: cargo test --release bench -- --ignored --nocapture
```

---

## 📝 Test Reporting

### Generate Coverage Report

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage
```

### Save Benchmark Results

```bash
# Run benchmarks and save results
cargo test --release bench -- --ignored --nocapture > benchmark_results.txt

# Compare with previous results
diff benchmark_results_old.txt benchmark_results.txt
```

---

## 🎓 Best Practices

1. **Always test in release mode** for realistic performance numbers
2. **Run stress tests on a dedicated test server** to avoid affecting development
3. **Monitor system resources** during load testing
4. **Compare results** over time to detect performance regressions
5. **Test with realistic data** - use production-like URL patterns
6. **Document any performance issues** found during testing

---

## 🔗 Additional Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [wrk Documentation](https://github.com/wg/wrk)
- [oha Documentation](https://github.com/hatoo/oha)
- [Load Testing Best Practices](https://www.nginx.com/blog/load-testing-best-practices/)
