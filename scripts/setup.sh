#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

command_exists() {
    command -v "$1" &>/dev/null
}

check_rust() {
    info "Checking Rust installation..."
    if command_exists rustc && command_exists cargo; then
        local version
        version=$(rustc --version)
        info "Rust found: $version"
    else
        error "Rust not found. Install via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi
}

check_postgresql() {
    info "Checking PostgreSQL..."
    if command_exists psql; then
        info "PostgreSQL client found: $(psql --version)"
    else
        warn "PostgreSQL client not found. Install it or use Docker (docker-compose up -d postgres)"
    fi
}

check_redis() {
    info "Checking Redis..."
    if command_exists redis-cli; then
        info "Redis CLI found: $(redis-cli --version)"
        if redis-cli ping &>/dev/null; then
            info "Redis server is running"
        else
            warn "Redis server not running. Start it or use Docker (docker-compose up -d redis)"
        fi
    else
        warn "Redis CLI not found. Install it or use Docker (docker-compose up -d redis)"
    fi
}

setup_env() {
    if [ ! -f .env ]; then
        if [ -f .env.example ]; then
            cp .env.example .env
            info "Created .env from .env.example"
            warn "Edit .env with your actual credentials before running the server"
        else
            error "No .env.example found"
        fi
    else
        info ".env already exists, skipping"
    fi
}

build_project() {
    info "Building project in release mode..."
    cargo build --release
    info "Build successful!"
}

run_tests() {
    info "Running tests..."
    cargo test --workspace --lib 2>/dev/null || warn "Some tests failed (may require database/redis)"
}

main() {
    echo ""
    echo "========================================="
    echo "  Blaniel NPC API - Setup"
    echo "========================================="
    echo ""

    check_rust
    check_postgresql
    check_redis
    setup_env

    echo ""
    read -p "Build the project now? [y/N] " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        build_project
        run_tests
    fi

    echo ""
    echo "========================================="
    echo "  Setup Complete!"
    echo "========================================="
    echo ""
    echo "Next steps:"
    echo "  1. Edit .env with your credentials"
    echo "  2. Start dependencies: docker-compose up -d postgres redis"
    echo "  3. Build & run: cargo run --release"
    echo "  4. Test health: curl http://localhost:3001/api/v1/health"
    echo ""
}

main "$@"
