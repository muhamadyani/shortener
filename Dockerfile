# Build stage
FROM rust:1-bookworm as builder

WORKDIR /app

# Copy manifests first to cache dependencies
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
# This creates a cached layer for dependencies so we don't rebuild them when source changes
RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY src ./src

# Build the application
# Touch main.rs to ensure rebuild of the application code
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/shortener /usr/local/bin/shortener

# Create directory for data
RUN mkdir -p /app/data

# Set environment variables
ENV PORT=8080
ENV DATABASE_URL=/app/data/data.db
ENV URL=http://localhost

# Expose the port
EXPOSE 8080

# Run the binary
CMD ["shortener"]
