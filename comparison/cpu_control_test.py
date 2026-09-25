#!/usr/bin/env python3
"""Exercise consecutive identical commands with guaranteed-distinct transport nonces."""

import json
import bench

spec = next(
    s
    for s in json.loads((bench.ROOT / "comparison/implementations.json").read_text())
    if s["name"] == "quickshell"
)
dest = bench.ROOT / "comparison/results/cpu/control-quickshell"
r = bench.Runtime(spec, dest)
result = {"commands": 0, "errors": []}
try:
    r.bars()
    for i in range(100):
        r.command("toggle launcher")
        expected = i % 2 == 0
        r.wait(lambda xs: any("launcher@" in x["namespace"] for x in xs) == expected)
        result["commands"] += 1
except Exception as e:
    result["errors"].append(str(e))
finally:
    r.close()
dest.with_suffix(".json").write_text(json.dumps(result, indent=2))
print(result, flush=True)
if result["errors"]:
    raise SystemExit(1)
