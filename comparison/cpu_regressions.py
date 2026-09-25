#!/usr/bin/env python3
"""Reuse existing pointer, keyboard and frontend checks without overwriting historical results."""

import json
import bench, interactions, keyboard, frontend

spec = next(
    s
    for s in json.loads((bench.ROOT / "comparison/implementations.json").read_text())
    if s["name"] == "iced"
)
dest = bench.ROOT / "comparison/results/cpu"
result = {"checks": {}, "errors": []}
# Existing helpers use fixed result paths. Preserve them and copy fresh evidence here.
original = {
    p: p.read_bytes()
    for p in (bench.ROOT / "comparison/results").glob("*")
    if p.is_file()
}
try:
    for name, run in [
        ("interactions", lambda: interactions.test(spec, dest / "interactions")),
        ("keyboard", lambda: keyboard.test(spec)),
        ("frontend", lambda: frontend.local(spec)),
    ]:
        print(name, flush=True)
        try:
            result["checks"][name] = run()
        except Exception as e:
            result["errors"].append({"check": name, "error": str(e)})
    for path in (bench.ROOT / "comparison/results").glob("*"):
        if path.is_file() and (
            path not in original or path.read_bytes() != original[path]
        ):
            (dest / path.name).write_bytes(path.read_bytes())
finally:
    for path, data in original.items():
        if path.read_bytes() != data:
            path.write_bytes(data)
(dest / "regressions.json").write_text(json.dumps(result, indent=2))
print(result, flush=True)
if result["errors"]:
    raise SystemExit(1)
