#!/usr/bin/env python3
"""Real inotify regression: nonstandard descriptor directory and symlink target edits."""

import json, tempfile, time
from pathlib import Path
import bench

spec = next(
    x
    for x in json.loads((bench.ROOT / "comparison/implementations.json").read_text())
    if x["name"] == "iced"
)
dest = bench.ROOT / "comparison/results/cpu"
r = bench.Runtime(spec, dest / "watch-paths")
result = {}


def wait_log(marker, start):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if marker in r.log_path.read_text()[start:]:
            return
        if r.process.poll() is not None:
            raise RuntimeError("runtime exited")
        time.sleep(0.02)
    raise TimeoutError(marker)


try:
    r.bars()
    with tempfile.TemporaryDirectory(
        prefix=".cpu-descriptors-", dir=bench.ROOT / "iced-shell"
    ) as tmp:
        actual = Path(tmp) / "actual.json"
        alias = Path(tmp) / "alias.json"
        page = {
            "kind": "bar",
            "title": "outside pages directory",
            "background": "#202332",
        }
        bench.write_json(actual, page)
        alias.symlink_to(actual.name)
        r.layout["surfaces"][0]["page"] = str(
            alias.relative_to(bench.ROOT / "iced-shell")
        )
        start = len(r.log_path.read_text())
        bench.write_json(r.config, r.layout)
        wait_log("CONFIG applied", start)
        time.sleep(0.3)
        before = {x["namespace"]: x["address"] for x in bench.layers(spec["namespace"])}
        page["title"] = "atomic symlink target reload"
        start = len(r.log_path.read_text())
        bench.write_json(actual, page)
        wait_log("CONTENT bar@", start)
        after = {x["namespace"]: x["address"] for x in bench.layers(spec["namespace"])}
        assert before == after, "descriptor reload recreated native surfaces"
        start = len(r.log_path.read_text())
        actual.write_text("{")
        wait_log("PAGE_ERROR", start)
        assert r.process.poll() is None
        page["title"] = "recovered descriptor"
        start = len(r.log_path.read_text())
        bench.write_json(actual, page)
        wait_log("CONTENT bar@", start)
        result = {
            "external_project_directory_reload": True,
            "symlink_target_atomic_replace": True,
            "malformed_descriptor_retained_host": True,
            "descriptor_recovery": True,
            "native_addresses_retained": True,
        }
finally:
    r.close()
(dest / "watch-paths.json").write_text(json.dumps(result, indent=2))
print(result, flush=True)
