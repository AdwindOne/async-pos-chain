#!/bin/bash
echo "🔧 Building project..."
cargo build --release || exit 1
echo "🚀 Running node..."
./target/release/async-pos-chain
