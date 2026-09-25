#!/usr/bin/env python3
"""Run existing graphical regressions, preserving historical fixed-path artifacts."""

import json
from pathlib import Path
import subprocess
import sys
import bench

root = bench.ROOT
results = root / "comparison/results"
dest = results / "release"
dest.mkdir(exist_ok=True)
original = {
    p: p.read_bytes()
    for p in results.rglob("*")
    if p.is_file() and dest not in p.parents
}
checks = []
try:
    commands = [
        ["bench.py", "--functional", "--output", str(dest / "functional.json")],
        ["interactions.py"],
        ["frontend.py"],
        ["keyboard.py"],
        ["cpu_watch_test.py"],
        ["cpu_outputs.py"],
        ["cpu_preview.py"],
        ["cpu_control_test.py"],
    ]
    for args in commands:
        print("CHECK", args[0], flush=True)
        with (dest / (Path(args[0]).stem + "-runner.log")).open("w") as log:
            run = subprocess.run(
                [sys.executable, str(root / "comparison" / args[0]), *args[1:]],
                stdout=log,
                stderr=subprocess.STDOUT,
            )
        checks.append({"script": args[0], "exit_code": run.returncode})
        (dest / "checks.json").write_text(json.dumps(checks, indent=2))
        print("RESULT", args[0], run.returncode, flush=True)
finally:
    for path in list(results.rglob("*")):
        if not path.is_file() or dest in path.parents:
            continue
        content = path.read_bytes()
        if path.parent == results or path not in original or original[path] != content:
            target = dest / "regressions" / path.relative_to(results)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(content)
        if path in original:
            if content != original[path]:
                path.write_bytes(original[path])
        else:
            path.unlink()
    for path, content in original.items():
        if not path.exists():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
if any(c["exit_code"] for c in checks):
    raise SystemExit(1)
