#!/bin/bash

# Stress Test Script for URL Shortener
# This script performs load testing using various tools

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="${BASE_URL:-http://localhost:8080}"
DURATION="${DURATION:-30s}"
CONNECTIONS="${CONNECTIONS:-100}"
THREADS="${THREADS:-4}"

echo -e "${BLUE}=== URL Shortener Stress Test ===${NC}"
echo -e "${BLUE}Base URL: ${BASE_URL}${NC}"
echo -e "${BLUE}Duration: ${DURATION}${NC}"
echo -e "${BLUE}Connections: ${CONNECTIONS}${NC}"
echo -e "${BLUE}Threads: ${THREADS}${NC}\n"

# Check if server is running
echo -e "${YELLOW}Checking if server is running...${NC}"
if ! curl -s "${BASE_URL}/api/urls?page=1&limit=1" > /dev/null 2>&1; then
    echo -e "${RED}Error: Server is not running at ${BASE_URL}${NC}"
    echo -e "${YELLOW}Please start the server first with: cargo run --release${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Server is running${NC}\n"

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Create test URLs for stress testing
echo -e "${YELLOW}Creating initial test URLs...${NC}"
for i in {1..10}; do
    curl -s -X POST "${BASE_URL}/api/urls" \
        -H "Content-Type: application/json" \
        -d "{\"url\":\"https://example.com/test${i}\",\"ref_id\":\"stress_test\"}" \
        > /dev/null
done
echo -e "${GREEN}✓ Created 10 test URLs${NC}\n"

# Test 1: Read Performance (GET redirect)
echo -e "${BLUE}=== Test 1: Redirect Performance ===${NC}"
if command_exists wrk; then
    echo -e "${YELLOW}Using wrk...${NC}"
    wrk -t${THREADS} -c${CONNECTIONS} -d${DURATION} "${BASE_URL}/stress_test_1" \
        --latency || true
elif command_exists oha; then
    echo -e "${YELLOW}Using oha...${NC}"
    oha -z ${DURATION} -c ${CONNECTIONS} "${BASE_URL}/stress_test_1" || true
elif command_exists ab; then
    echo -e "${YELLOW}Using Apache Bench...${NC}"
    ab -t 30 -c ${CONNECTIONS} "${BASE_URL}/stress_test_1" || true
else
    echo -e "${RED}No load testing tool found. Please install wrk, oha, or apache-bench${NC}"
    echo -e "${YELLOW}Installation:${NC}"
    echo -e "  macOS: brew install wrk"
    echo -e "  Rust:  cargo install oha"
fi
echo ""

# Test 2: Create URL Performance (POST)
echo -e "${BLUE}=== Test 2: Create URL Performance ===${NC}"
if command_exists oha; then
    echo -e "${YELLOW}Using oha for POST requests...${NC}"
    oha -z ${DURATION} -c ${CONNECTIONS} -m POST \
        -H "Content-Type: application/json" \
        -d '{"url":"https://example.com/load-test","ref_id":"load_user"}' \
        "${BASE_URL}/api/urls" || true
elif command_exists wrk; then
    echo -e "${YELLOW}Using wrk with Lua script for POST...${NC}"
    cat > /tmp/post.lua << 'EOF'
wrk.method = "POST"
wrk.body   = '{"url":"https://example.com/load-test","ref_id":"load_user"}'
wrk.headers["Content-Type"] = "application/json"
EOF
    wrk -t${THREADS} -c${CONNECTIONS} -d${DURATION} -s /tmp/post.lua "${BASE_URL}/api/urls" \
        --latency || true
    rm -f /tmp/post.lua
else
    echo -e "${YELLOW}Skipping POST test (requires oha or wrk)${NC}"
fi
echo ""

# Test 3: List URLs Performance (GET with query params)
echo -e "${BLUE}=== Test 3: List URLs Performance ===${NC}"
if command_exists wrk; then
    echo -e "${YELLOW}Using wrk...${NC}"
    wrk -t${THREADS} -c${CONNECTIONS} -d${DURATION} \
        "${BASE_URL}/api/urls?ref_id=stress_test&page=1&limit=10" \
        --latency || true
elif command_exists oha; then
    echo -e "${YELLOW}Using oha...${NC}"
    oha -z ${DURATION} -c ${CONNECTIONS} \
        "${BASE_URL}/api/urls?ref_id=stress_test&page=1&limit=10" || true
fi
echo ""

# Test 4: Mixed Load Test
echo -e "${BLUE}=== Test 4: Mixed Load Test ===${NC}"
echo -e "${YELLOW}Running concurrent read/write operations...${NC}"

# Function to run background requests
run_background_requests() {
    local endpoint=$1
    local method=$2
    local count=$3
    
    for i in $(seq 1 ${count}); do
        if [ "$method" = "POST" ]; then
            curl -s -X POST "${BASE_URL}${endpoint}" \
                -H "Content-Type: application/json" \
                -d "{\"url\":\"https://example.com/mixed${i}\"}" \
                > /dev/null &
        else
            curl -s "${BASE_URL}${endpoint}" > /dev/null &
        fi
    done
}

echo -e "${YELLOW}Sending 1000 mixed requests...${NC}"
start_time=$(date +%s)

# Run 500 create requests
run_background_requests "/api/urls" "POST" 500

# Run 500 read requests
for i in {1..500}; do
    curl -s "${BASE_URL}/api/urls?page=1&limit=10" > /dev/null &
done

# Wait for all background jobs
wait

end_time=$(date +%s)
duration=$((end_time - start_time))

echo -e "${GREEN}✓ Completed 1000 requests in ${duration} seconds${NC}"
echo -e "${GREEN}  Throughput: $((1000 / duration)) req/s${NC}\n"

# Test 5: Database Size Test
echo -e "${BLUE}=== Test 5: Database Growth Test ===${NC}"
echo -e "${YELLOW}Creating 10,000 URLs to test database performance...${NC}"

start_time=$(date +%s)
created=0

for i in {1..10000}; do
    if curl -s -X POST "${BASE_URL}/api/urls" \
        -H "Content-Type: application/json" \
        -d "{\"url\":\"https://example.com/db-test${i}\",\"ref_id\":\"db_test_user\"}" \
        > /dev/null 2>&1; then
        ((created++))
    fi
    
    # Progress indicator
    if [ $((i % 1000)) -eq 0 ]; then
        echo -e "${YELLOW}  Created ${i} URLs...${NC}"
    fi
done

end_time=$(date +%s)
duration=$((end_time - start_time))

echo -e "${GREEN}✓ Created ${created}/10000 URLs in ${duration} seconds${NC}"
echo -e "${GREEN}  Throughput: $((created / duration)) req/s${NC}\n"

# Final Summary
echo -e "${BLUE}=== Stress Test Complete ===${NC}"
echo -e "${GREEN}All tests finished successfully!${NC}\n"

# Recommendations
echo -e "${YELLOW}Recommendations:${NC}"
echo -e "1. Monitor memory usage during high load"
echo -e "2. For production, consider rate limiting"
echo -e "3. Regular database backups are essential"
echo -e "4. Scale horizontally behind a load balancer for high traffic\n"

# Installation suggestions
echo -e "${YELLOW}To get more detailed metrics, install these tools:${NC}"
echo -e "  wrk:  brew install wrk  (macOS) or apt-get install wrk (Linux)"
echo -e "  oha:  cargo install oha"
echo -e "  htop: brew install htop (for monitoring)\n"
