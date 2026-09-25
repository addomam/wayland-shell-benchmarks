#!/usr/bin/env python3
"""Extract sampled self-overhead; inclusive rows must not be added together."""

from pathlib import Path
import collections, json, re

D = Path(__file__).resolve().parent / "results/cpu"
result = {}
for p in D.glob("*.perf-report.txt"):
    text = p.read_text()
    rows = []
    for line in text.splitlines():
        m = re.match(r"\s*([\d.]+)%\s+([\d.]+)%\s+(.+?)\s+\[(.)\]\s+(.*)", line)
        if m:
            rows.append(
                {
                    "inclusive_percent": float(m[1]),
                    "self_percent": float(m[2]),
                    "object": m[3].split()[-1],
                    "symbol": m[5],
                }
            )
    dso = collections.defaultdict(float)
    for row in rows:
        dso[row["object"]] += row["self_percent"]
    sample = re.search(r"# Samples: ([\d,]+)", text)
    result[p.name] = {
        "samples": int(sample[1].replace(",", "")) if sample else 0,
        "self_by_object": dict(sorted(dso.items(), key=lambda kv: -kv[1])),
        "symbols": sorted(rows, key=lambda row: -row["self_percent"]),
    }
(D / "profile-summary.json").write_text(json.dumps(result, indent=2))
for name, r in result.items():
    print(name, r["samples"], {k: round(v, 2) for k, v in r["self_by_object"].items()})
