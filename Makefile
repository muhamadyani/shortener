# Development
dev:
	cargo watch -q -c -i "data.db" -x run

# Production
start:
	cargo run --release

# Build
build:
	cargo build --release

# Testing
test:
	cargo test

test-verbose:
	cargo test -- --nocapture

test-integration:
	cargo test --test integration_test

# Benchmarks
bench:
	cargo test --release bench -- --ignored --nocapture

bench-create:
	cargo test --release bench_create_urls -- --ignored --nocapture

bench-list:
	cargo test --release bench_list_urls -- --ignored --nocapture

bench-scaling:
	cargo test --release bench_database_scaling -- --ignored --nocapture

bench-concurrent:
	cargo test --release bench_concurrent_operations -- --ignored --nocapture

# Stress Testing
stress:
	./stress_test.sh

stress-quick:
	DURATION=10s CONNECTIONS=50 ./stress_test.sh

stress-heavy:
	DURATION=60s CONNECTIONS=200 THREADS=8 ./stress_test.sh

# Cleanup
clean:
	cargo clean
	rm -f data.db

# Check
check:
	cargo check
	cargo clippy

# Help
help:
	@echo "Available targets:"
	@echo "  dev              - Run in development mode with auto-reload"
	@echo "  start            - Run in production mode"
	@echo "  build            - Build release binary"
	@echo ""
	@echo "Testing:"
	@echo "  test             - Run all tests"
	@echo "  test-verbose     - Run tests with output"
	@echo "  test-integration - Run integration tests only"
	@echo ""
	@echo "Benchmarks:"
	@echo "  bench            - Run all benchmarks"
	@echo "  bench-create     - Benchmark URL creation"
	@echo "  bench-list       - Benchmark listing/queries"
	@echo "  bench-scaling    - Benchmark database scaling"
	@echo "  bench-concurrent - Benchmark concurrent operations"
	@echo ""
	@echo "Stress Testing:"
	@echo "  stress           - Run stress test (default settings)"
	@echo "  stress-quick     - Quick stress test (10s, 50 connections)"
	@echo "  stress-heavy     - Heavy stress test (60s, 200 connections)"
	@echo ""
	@echo "Utilities:"
	@echo "  clean            - Clean build artifacts and database"
	@echo "  check            - Run cargo check and clippy"
	@echo "  help             - Show this help message"

.PHONY: dev start build test test-verbose test-integration bench bench-create bench-list bench-scaling bench-concurrent stress stress-quick stress-heavy clean check help