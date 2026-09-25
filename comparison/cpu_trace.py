#!/usr/bin/env python3
"""Launch as strace's child: works with Yama without changing ptrace policy."""

import argparse, collections, json, re, time
from pathlib import Path
import bench

p = argparse.ArgumentParser()
p.add_argument("--name", default="iced")
p.add_argument("--label", required=True)
p.add_argument("--env", action="append", default=[])
p.add_argument("--command")
a = p.parse_args()
dest = bench.ROOT / "comparison/results/cpu" / a.label
spec = next(
    x
    for x in json.loads((bench.ROOT / "comparison/implementations.json").read_text())
    if x["name"] == a.name
)
spec.setdefault("env", {}).update(dict(x.split("=", 1) for x in a.env))
if a.command:
    spec["command"][0] = a.command
spec["command"] = [
    "strace",
    "-f",
    "-ttt",
    "-T",
    "-o",
    str(dest) + ".strace.txt",
] + spec["command"]
r = bench.Runtime(spec, dest)
try:
    if spec["env"].get("SHELL_STATIC") == "1":
        r.wait(lambda xs: len(xs) == 1)
    else:
        r.bars()
    time.sleep(12)
    start = time.time()
    time.sleep(30)
    end = time.time()
finally:
    r.close()
counts = collections.Counter()
threads = collections.defaultdict(collections.Counter)
for line in Path(str(dest) + ".strace.txt").read_text().splitlines():
    m = re.match(r"(\d+)\s+(\d+\.\d+)\s+(\w+)\(", line)
    if m and start <= float(m[2]) <= end:
        counts[m[3]] += 1
        threads[m[1]][m[3]] += 1
Path(str(dest) + ".json").write_text(
    json.dumps(
        {
            "start": start,
            "end": end,
            "syscall_entries": counts,
            "threads": threads,
            "note": "Entries begun in settled 30-second window; blocked calls begun before it are absent. Tracing perturbs timings; not used for CPU benchmark.",
        },
        indent=2,
    )
)
print(a.label, counts, flush=True)
