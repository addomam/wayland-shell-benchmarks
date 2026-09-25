#!/usr/bin/env python3
"""Summarize preserved CPU experiments without modifying the historical report."""

import json, statistics, re, math
from pathlib import Path

D = Path(__file__).resolve().parent / "results/cpu"


def measurement_rows(file, name):
    data = json.loads((D / file).read_text())["samples"]
    retry = D / "after-quickshell-retry.json"
    if file == "after.json" and retry.exists():
        data += json.loads(retry.read_text())["samples"]
    return [s for s in data if s["name"] == name]


lines = [
    "# CPU measurements",
    "",
    "CPU is percent of one core. Memory is summed process-tree MiB. Values below are medians.",
    "",
    "| Version | n | Startup ms | Idle PSS | Active PSS | Idle CPU % | Active CPU % |",
    "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
]
for file, name, label in [
    ("before.json", "iced", "iced before"),
    ("after.json", "iced", "iced after"),
    ("after.json", "quickshell", "Quickshell"),
    ("after.json", "web", "WebKit"),
]:
    if not (D / file).exists():
        continue
    rows = measurement_rows(file, name)
    if not rows:
        continue
    values = [statistics.median(s["startup_layer_ms"] for s in rows)]
    for metric in ["median_pss_mib", "cpu_percent_one_core"]:
        for state in ["idle", "active"]:
            values.append(statistics.median(s[state][metric] for s in rows))
    lines.append(
        "| "
        + label
        + " | "
        + str(len(rows))
        + " | "
        + " | ".join(f"{v:.3f}" for v in values)
        + " |"
    )
lines += [
    "",
    "## Memory, process counts and latency tails",
    "",
    "| Version | Idle RSS MiB | Active RSS MiB | Processes idle/active | Startup p95 ms | Launcher control median / p95 ms |",
    "| --- | ---: | ---: | --- | ---: | ---: |",
]
for file, name, label in [
    ("before.json", "iced", "iced before"),
    ("after.json", "iced", "iced after"),
    ("after.json", "quickshell", "Quickshell"),
    ("after.json", "web", "WebKit"),
]:
    if not (D / file).exists():
        continue
    rows = measurement_rows(file, name)
    if not rows:
        continue
    p95 = lambda values: sorted(values)[math.ceil(len(values) * 0.95) - 1]
    rss = [
        statistics.median(s[state]["median_rss_mib"] for s in rows)
        for state in ["idle", "active"]
    ]
    counts = [
        statistics.median(
            sample["processes"] for s in rows for sample in s[state]["samples"]
        )
        for state in ["idle", "active"]
    ]
    control = [s["launcher_control_layer_ms"] for s in rows]
    lines.append(
        f"| {label} | {rss[0]:.1f} | {rss[1]:.1f} | {counts[0]:g}/{counts[1]:g} | {p95([s['startup_layer_ms'] for s in rows]):.1f} | {statistics.median(control):.1f} / {p95(control):.1f} |"
    )
lines += [
    "",
    "## Isolated experiments",
    "",
    "Rates sum all surfaces. Dispatch returns are UI-loop iterations, not total process thread wakeups. First diagnostic interval (startup) is excluded.",
    "",
    "| Experiment | CPU % | Dispatches/s | Views/s | Redraw requests/s | Presents/s | Thread context switches/s |",
    "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
]
for name in [
    "instrumented-before",
    "instrumented-before-active",
    "static-initial",
    "static-complete",
    "clock-minute",
    "clock-second",
    "workspaces-only",
    "watcher-only",
    "normal-idle",
    "normal-active",
    "active-no-clock",
    "launcher-only",
    "backend-gl",
    "profiled-idle",
    "profiled-active",
    "quickshell-idle",
    "quickshell-active",
]:
    path = D / (name + ".json")
    if not path.exists():
        continue
    x = json.loads(path.read_text())
    samples = x.get("diagnostics", [])[1:]
    seconds = sum(s["seconds"] for s in samples)
    values = [f"{x['resources']['cpu_percent_one_core']:.3f}"]
    values += [
        f"{sum(s[k] for s in samples) / seconds:.3f}" if seconds else "—"
        for k in ["wakeups", "views", "redraw_requests", "presents"]
    ]
    values += [
        f"{sum(x['context_switches'].values()) / x['resources']['duration_s']:.3f}"
    ]
    lines.append("| " + name + " | " + " | ".join(values) + " |")
lines += [
    "",
    "## perf stat",
    "",
    "Separate 30-second windows. This host restricts perf to user-space events; its `context-switches:u` zero is not a valid scheduling count. Thread context switches above come from `/proc/PID/task/*/status`.",
    "",
    "| Experiment | Task-clock ms | Cycles | Instructions | Instructions/s |",
    "| --- | ---: | ---: | ---: | ---: |",
]
for path in sorted(D.glob("*.perf-stat.txt")):
    text = path.read_text()
    vals = {}
    for k in ["task-clock", "cycles", "instructions"]:
        m = re.search(r"([\d,.]+)\s+(?:msec\s+)?" + re.escape(k), text)
        if m:
            vals[k] = float(m[1].replace(",", ""))
    if len(vals) == 3:
        lines.append(
            f"| {path.name.removesuffix('.perf-stat.txt')} | {vals['task-clock']:.3f} | {vals['cycles']:.0f} | {vals['instructions']:.0f} | {vals['instructions'] / 30:.0f} |"
        )
(D.parent.parent / "CPU_TABLES.md").write_text("\n".join(lines) + "\n")
print("\n".join(lines))
