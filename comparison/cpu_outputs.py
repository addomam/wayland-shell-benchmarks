#!/usr/bin/env python3
"""Exercise Wayland output events using a temporary Hyprland headless output."""

import json, time
import bench

spec = next(
    x
    for x in json.loads((bench.ROOT / "comparison/implementations.json").read_text())
    if x["name"] == "iced"
)
before = bench.query("monitors")
r = bench.Runtime(spec, bench.ROOT / "comparison/results/cpu/output-events")
created = []
result = {}
try:
    r.bars()
    result["create_reply"] = bench.ipc("output create headless")
    time.sleep(2)
    added = [
        m["name"]
        for m in bench.query("monitors")
        if m["name"] not in {x["name"] for x in before}
    ]
    created = added
    if len(added) != 1:
        raise RuntimeError(
            "Expected one temporary headless output: " + result["create_reply"]
        )
    key = spec["namespace"] + "bar@" + added[0]
    result["added_layers"] = r.wait(lambda xs: any(x["namespace"] == key for x in xs))
    result["remove_reply"] = bench.ipc("output remove " + added[0])
    created = []
    result["removed_layers"] = r.wait(
        lambda xs: not any(x["namespace"] == key for x in xs)
    )
    result["passed"] = True
finally:
    for name in created:
        bench.ipc("output remove " + name)
    r.close()
(bench.ROOT / "comparison/results/cpu/output-events.json").write_text(
    json.dumps(result, indent=2)
)
print(result, flush=True)
