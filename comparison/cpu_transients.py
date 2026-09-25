#!/usr/bin/env python3
"""Check activation transients without changing the shared benchmark's settling policy."""

import json, time
import bench

spec = next(
    x
    for x in json.loads((bench.ROOT / "comparison/implementations.json").read_text())
    if x["name"] == "web"
)
dest = bench.ROOT / "comparison/results/cpu/web-active-transient"
r = bench.Runtime(spec, dest)
try:
    r.bars()
    time.sleep(10)
    for s in r.layout["surfaces"]:
        s["visible"] = True
    bench.write_json(r.config, r.layout)
    r.wait(
        lambda xs: len(xs) == len(r.layout["surfaces"]) * len(bench.query("monitors"))
    )
    time.sleep(1)
    samples = [bench.resources(r, 5) for _ in range(6)]
    dest.with_suffix(".json").write_text(
        json.dumps(
            {
                "method": "Six consecutive five-second resource windows starting after the shared one-second active settling period; separate diagnostic, not pooled with final runs",
                "samples": samples,
            },
            indent=2,
        )
    )
    print([round(s["cpu_percent_one_core"], 3) for s in samples], flush=True)
finally:
    r.close()
