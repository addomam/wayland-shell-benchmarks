#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python="${PYTHON:-python3}"
sha256sum iced-shell/target/release/iced-shell iced-shell/target/profiling/iced-shell > comparison/results/cpu/final-binaries.sha256
"$python" comparison/bench.py --only iced --functional --output comparison/results/cpu/functional.json
"$python" comparison/cpu_regressions.py
"$python" comparison/bench.py --runs 10 --sample-seconds 30 --output comparison/results/cpu/after.json
"$python" comparison/cpu_summarize.py
