#!/usr/bin/env python3
"""Small ordinary-window lifecycle smoke test after the event-source refactor."""

import json, time
import bench, interactions

spec = next(
    x
    for x in json.loads((bench.ROOT / "comparison/implementations.json").read_text())
    if x["name"] == "iced"
)
spec["command"].append("--preview")
before = bench.query("monitors")
focused = bench.query("activeworkspace")["id"]
r = bench.Runtime(spec, bench.ROOT / "comparison/results/cpu/preview")


def wait(count):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        clients = [c for c in bench.query("clients") if c["pid"] == r.process.pid]
        if len(clients) == count:
            return clients
        if r.process.poll() is not None:
            raise RuntimeError("preview exited")
        time.sleep(0.02)
    raise TimeoutError("preview client count " + str(count))


try:
    bars = wait(len(before))
    r.command("toggle launcher")
    shown = wait(len(before) + 1)
    r.command("toggle launcher")
    hidden = wait(len(before))
    result = {
        "bars": bars,
        "launcher_shown": len(shown),
        "launcher_hidden": len(hidden),
        "passed": True,
    }
finally:
    r.close()
    interactions.restore_workspaces(before, focused)
(bench.ROOT / "comparison/results/cpu/preview.json").write_text(
    json.dumps(result, indent=2)
)
print("preview passed", flush=True)
