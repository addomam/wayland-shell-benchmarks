#!/usr/bin/env python3
"""Summarize a new benchmark without mixing historical or supplemental samples."""

import argparse
import json
import math
from pathlib import Path
import statistics

parser = argparse.ArgumentParser()
parser.add_argument("input", type=Path)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
data = json.loads(args.input.read_text())
lines = [
    "# Release validation measurements",
    "",
    "Medians; CPU is percent of one core, memory is summed process-tree MiB. Startup ends at compositor-observed layer mapping, not first painted frame.",
    "",
    "| Implementation | n | Startup ms | Idle PSS | Active PSS | Idle CPU % | Active CPU % |",
    "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
]
for name in ["iced", "quickshell", "web"]:
    rows = [s for s in data["samples"] if s["name"] == name]
    if not rows:
        continue
    values = [statistics.median(s["startup_layer_ms"] for s in rows)]
    values += [
        statistics.median(s[state][metric] for s in rows)
        for metric in ["median_pss_mib", "cpu_percent_one_core"]
        for state in ["idle", "active"]
    ]
    lines.append(
        f"| {name} | {len(rows)} | " + " | ".join(f"{v:.3f}" for v in values) + " |"
    )
lines += [
    "",
    "| Implementation | Idle / active RSS MiB | Processes idle / active | Startup p95 ms | Control launcher median / p95 ms | Host busy idle / active % |",
    "| --- | ---: | ---: | ---: | ---: | ---: |",
]
for name in ["iced", "quickshell", "web"]:
    rows = [s for s in data["samples"] if s["name"] == name]
    if not rows:
        continue

    def median(values):
        return statistics.median(values)

    def p95(values):
        return sorted(values)[math.ceil(len(values) * 0.95) - 1]

    rss = [
        median(s[state]["median_rss_mib"] for s in rows) for state in ["idle", "active"]
    ]
    counts = [
        median(sample["processes"] for s in rows for sample in s[state]["samples"])
        for state in ["idle", "active"]
    ]
    host = [
        median(s[state]["host_busy_percent"] for s in rows)
        for state in ["idle", "active"]
    ]
    control = [s["launcher_control_layer_ms"] for s in rows]
    lines.append(
        f"| {name} | {rss[0]:.1f} / {rss[1]:.1f} | {counts[0]:g} / {counts[1]:g} | {p95([s['startup_layer_ms'] for s in rows]):.1f} | {median(control):.1f} / {p95(control):.1f} | {host[0]:.2f} / {host[1]:.2f} |"
    )
lines += [
    "",
    f"Recorded failures: {len(data['errors'])}. Inspect the input JSON for all raw samples and errors.",
    "",
]
args.output.write_text("\n".join(lines))
