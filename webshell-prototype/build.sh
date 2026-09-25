#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --locked
cargo test --locked
