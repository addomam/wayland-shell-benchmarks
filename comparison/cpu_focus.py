#!/usr/bin/env python3
"""Check automatic launcher input focus before interpreting animation workload."""

import copy, json, subprocess, time
from pathlib import Path
import bench, interactions

spec = next(
    x
    for x in json.loads((bench.ROOT / "comparison/implementations.json").read_text())
    if x["name"] == "iced"
)
results = []
for name, command in [
    (
        "before",
        str(bench.ROOT / "iced-shell/target/cpu-before/iced-shell-instrumented"),
    ),
    ("after", spec["command"][0]),
]:
    s = copy.deepcopy(spec)
    s["command"][0] = command
    s["env"] = {"SHELL_DIAGNOSTICS": "1"}
    dest = bench.ROOT / "comparison/results/cpu" / ("autofocus-" + name)
    r = bench.Runtime(s, dest)
    try:
        r.bars()
        r.command("toggle launcher")
        items = r.wait(lambda xs: any("launcher@" in x["namespace"] for x in xs))
        time.sleep(1)
        win = next(x for x in items if "launcher@" in x["namespace"])
        interactions.screenshot(win, Path(str(dest) + "-before.png"))
        subprocess.run(
            [interactions.WLRCTL, "keyboard", "type", "Files"],
            check=True,
            capture_output=True,
        )
        time.sleep(0.4)
        interactions.screenshot(win, Path(str(dest) + "-typed.png"))
        results.append({"name": name, "layers": items})
    finally:
        r.close()
(bench.ROOT / "comparison/results/cpu/autofocus.json").write_text(
    json.dumps(results, indent=2)
)
