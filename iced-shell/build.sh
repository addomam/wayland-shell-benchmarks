#!/usr/bin/env bash
set -euo pipefail
cargo build --release --locked
cargo test --release --locked
cargo fmt --check
