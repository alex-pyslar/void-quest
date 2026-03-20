#!/usr/bin/env bash
# VoidQuest build script for WSL / Linux
set -e

echo "=== VoidQuest Build Script ==="

# Check for Rust
if ! command -v cargo &>/dev/null; then
    echo "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "Rust version: $(rustc --version)"

# Build
echo ""
echo "Building release binaries..."
cargo build --release

echo ""
echo "=== Build complete ==="
echo "  Server: ./target/release/vq-server"
echo "  Client: ./target/release/vq-client"
echo ""
echo "Usage:"
echo "  Terminal 1:  ./target/release/vq-server"
echo "  Terminal 2+: ./target/release/vq-client [host:port]"
echo ""
echo "Or use cargo directly:"
echo "  cargo run --bin vq-server"
echo "  cargo run --bin vq-client"
