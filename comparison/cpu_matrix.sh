#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python="${PYTHON:-python3}"
probe() { "$python" comparison/cpu_probe.py --seconds 40 "$@"; }
probe --label static-complete --env SHELL_CLOCK=0 --env SHELL_NO_WATCH=1 --env SHELL_NO_WORKSPACES=1
probe --label clock-second --env SHELL_NO_WATCH=1 --env SHELL_NO_WORKSPACES=1
probe --label clock-minute --env SHELL_CLOCK=60 --env SHELL_NO_WATCH=1 --env SHELL_NO_WORKSPACES=1 --seconds 70
probe --label workspaces-only --env SHELL_CLOCK=0 --env SHELL_NO_WATCH=1
probe --label watcher-only --env SHELL_CLOCK=0 --env SHELL_NO_WORKSPACES=1
probe --label normal-idle
probe --label normal-active --active
probe --label active-no-clock --active --env SHELL_CLOCK=0
probe --label launcher-only --visible launcher --env SHELL_CLOCK=0
