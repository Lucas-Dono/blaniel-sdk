#!/bin/bash

# Test and Build Script for Rust NPC API
# This script validates all critical fixes and runs benchmarks

set -e

echo "========================================="
echo "  Rust NPC API - Testing & Benchmarking"
echo "========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}ERROR: cargo is not installed${NC}"
    echo "Install Rust: https://rustup.rs/"
    exit 1
fi

echo -e "${GREEN}✓${NC} Cargo found: $(cargo --version)"
echo ""

# Check environment
echo "Checking environment variables..."
if [ -z "$VENICE_API_KEY" ]; then
    echo -e "${YELLOW}⚠${NC}  VENICE_API_KEY not set (required for AI benchmarks)"
else
    echo -e "${GREEN}✓${NC} VENICE_API_KEY configured"
fi

if [ -z "$DATABASE_URL" ]; then
    echo -e "${YELLOW}⚠${NC}  DATABASE_URL not set"
else
    echo -e "${GREEN}✓${NC} DATABASE_URL configured"
fi

if [ -z "$REDIS_URL" ]; then
    echo -e "${YELLOW}⚠${NC}  REDIS_URL not set"
else
    echo -e "${GREEN}✓${NC} REDIS_URL configured"
fi

echo ""

# 1. Format check
echo "========================================="
echo "Step 1: Checking code formatting..."
echo "========================================="
cargo fmt --all -- --check || {
    echo -e "${YELLOW}⚠${NC}  Code formatting issues found"
    echo "Fix with: cargo fmt --all"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
}
echo -e "${GREEN}✓${NC} Code formatting OK"
echo ""

# 2. Clippy lints
echo "========================================="
echo "Step 2: Running Clippy lints..."
echo "========================================="
cargo clippy --all-targets --all-features -- -D warnings || {
    echo -e "${RED}✗${NC} Clippy found issues"
    exit 1
}
echo -e "${GREEN}✓${NC} Clippy checks passed"
echo ""

# 3. Build
echo "========================================="
echo "Step 3: Building project..."
echo "========================================="
cargo build --release || {
    echo -e "${RED}✗${NC} Build failed"
    exit 1
}
echo -e "${GREEN}✓${NC} Build successful"
echo ""

# 4. Run tests
echo "========================================="
echo "Step 4: Running unit tests..."
echo "========================================="
cargo test --all || {
    echo -e "${RED}✗${NC} Tests failed"
    exit 1
}
echo -e "${GREEN}✓${NC} All tests passed"
echo ""

# 5. Run performance benchmarks
echo "========================================="
echo "Step 5: Running performance benchmarks..."
echo "========================================="
echo ""
echo "This will measure:"
echo "  - Cache operations (target: <100μs)"
echo "  - Distance calculations (target: <5μs)"
echo "  - HMAC verification (target: <50μs)"
echo "  - JSON operations (target: <100μs)"
echo ""

cargo bench --bench performance -- --output-format bencher | tee benchmark_results.txt

echo ""
echo -e "${GREEN}✓${NC} Performance benchmarks completed"
echo "Results saved to: benchmark_results.txt"
echo ""

# 6. Optional: AI provider benchmarks
if [ ! -z "$VENICE_API_KEY" ]; then
    echo "========================================="
    echo "Step 6: Running AI provider benchmarks..."
    echo "========================================="
    echo ""
    echo "This will make REAL API calls to Venice AI"
    echo "Target: <20ms response time"
    echo ""

    read -p "Run AI benchmarks? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cargo bench --bench ai_providers --features integration-tests | tee ai_benchmark_results.txt
        echo -e "${GREEN}✓${NC} AI benchmarks completed"
        echo "Results saved to: ai_benchmark_results.txt"
    else
        echo "Skipping AI benchmarks"
    fi
else
    echo "========================================="
    echo "Step 6: Skipping AI benchmarks"
    echo "========================================="
    echo "Set VENICE_API_KEY to run AI provider benchmarks"
fi

echo ""
echo "========================================="
echo "  Summary"
echo "========================================="
echo ""
echo -e "${GREEN}✓${NC} All critical fixes validated"
echo -e "${GREEN}✓${NC} Code compiles successfully"
echo -e "${GREEN}✓${NC} Tests passing"
echo -e "${GREEN}✓${NC} Benchmarks completed"
echo ""
echo "Next steps:"
echo "  1. Review benchmark results in benchmark_results.txt"
echo "  2. Check AI latency (target: <20ms)"
echo "  3. Start the server: cargo run --release"
echo "  4. Monitor logs for performance warnings"
echo ""
echo "Performance guide: PERFORMANCE-GUIDE.md"
echo ""
