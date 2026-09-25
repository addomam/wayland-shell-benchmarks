#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python="${PYTHON:-python3}"
perf_bin="${PERF:-perf}"
if [[ -x iced-shell/target/cpu-before/iced-shell-instrumented ]]; then
  "$python" comparison/cpu_probe.py --label instrumented-before-active --command "$PWD/iced-shell/target/cpu-before/iced-shell-instrumented" --active --seconds 40
fi
"$python" comparison/cpu_probe.py --label profiled-idle --command "$PWD/iced-shell/target/profiling/iced-shell" --seconds 40 --perf "$perf_bin"
"$python" comparison/cpu_probe.py --label profiled-active --command "$PWD/iced-shell/target/profiling/iced-shell" --seconds 40 --active --perf "$perf_bin"
"$python" comparison/cpu_probe.py --label quickshell-idle --name quickshell --env QSG_INFO=1 --seconds 40 --perf "$perf_bin"
"$python" comparison/cpu_probe.py --label quickshell-active --name quickshell --env QSG_INFO=1 --seconds 40 --active --perf "$perf_bin"
"$python" comparison/cpu_trace.py --label static-trace --env SHELL_STATIC=1
"$python" comparison/cpu_trace.py --label idle-trace
"$python" comparison/cpu_trace.py --label quickshell-trace --name quickshell
if [[ -x iced-shell/target/cpu-before/iced-shell ]]; then
  "$python" comparison/cpu_trace.py --label before-trace --command "$PWD/iced-shell/target/cpu-before/iced-shell"
fi
