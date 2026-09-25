#!/usr/bin/env python3
"""Settled diagnostic run. Perf/strace execute separately to avoid perturbing CPU windows."""

import argparse, json, signal, subprocess, time
from pathlib import Path
import bench


def switches(pid):
    total = {"voluntary_ctxt_switches": 0, "nonvoluntary_ctxt_switches": 0}
    for p in Path(f"/proc/{pid}/task").glob("*/status"):
        try:
            for line in p.read_text().splitlines():
                k, _, v = line.partition(":")
                if k in total:
                    total[k] += int(v)
        except FileNotFoundError:
            pass
    return total


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--name", default="iced")
    p.add_argument("--label", required=True)
    p.add_argument("--seconds", type=float, default=40)
    p.add_argument("--active", action="store_true")
    p.add_argument("--visible", default="")
    p.add_argument("--command")
    p.add_argument("--env", action="append", default=[])
    p.add_argument("--perf")
    p.add_argument("--trace", action="store_true")
    a = p.parse_args()
    dest = bench.ROOT / "comparison/results/cpu" / a.label
    spec = next(
        x
        for x in json.loads(
            (bench.ROOT / "comparison/implementations.json").read_text()
        )
        if x["name"] == a.name
    )
    spec.setdefault("env", {}).update(dict(v.split("=", 1) for v in a.env))
    if a.name == "iced":
        spec["env"]["SHELL_DIAGNOSTICS"] = "1"
    if a.command:
        spec["command"][0] = a.command
    if spec["env"].get("SHELL_STATIC") == "1":
        spec["env"]["SHELL_STATIC_OUTPUT"] = bench.query("monitors")[0]["name"]
    r = bench.Runtime(spec, dest)
    try:
        if a.active or a.visible:
            r.bars()
            for s in r.layout["surfaces"]:
                if a.active or s["id"] in a.visible.split(","):
                    s["visible"] = True
            bench.write_json(r.config, r.layout)
            expected = sum(s.get("visible", True) for s in r.layout["surfaces"]) * len(
                bench.query("monitors")
            )
            r.wait(lambda xs: len(xs) == expected)
        elif spec["env"].get("SHELL_STATIC") == "1":
            r.wait(lambda xs: len(xs) == 1)
        else:
            r.bars()
        time.sleep(12)
        before = switches(r.process.pid)
        data = {
            "label": a.label,
            "env": spec["env"],
            "pid": r.process.pid,
            "layers": bench.layers(spec["namespace"]),
        }
        data["resources"] = bench.resources(r, a.seconds)
        after = switches(r.process.pid)
        data["context_switches"] = {k: after[k] - v for k, v in before.items()}
        if a.perf:
            for mode in ["stat", "record"]:
                cmd = [a.perf, mode]
                if mode == "stat":
                    cmd += [
                        "-e",
                        "task-clock,context-switches,cpu-migrations,page-faults,cycles,instructions",
                        "-o",
                        str(dest) + ".perf-stat.txt",
                    ]
                else:
                    cmd += [
                        "-F",
                        "999",
                        "--call-graph",
                        "dwarf",
                        "-o",
                        str(dest) + ".perf.data",
                    ]
                cmd += ["-p", str(r.process.pid), "--", "sleep", "30"]
                result = subprocess.run(cmd, capture_output=True, text=True)
                Path(str(dest) + f".perf-{mode}-stderr.txt").write_text(result.stderr)
                data["perf_" + mode + "_returncode"] = result.returncode
                if mode == "record" and result.returncode == 0:
                    report = subprocess.run(
                        [a.perf, "report", "--stdio", "-i", str(dest) + ".perf.data"],
                        capture_output=True,
                        text=True,
                    )
                    Path(str(dest) + ".perf-report.txt").write_text(
                        report.stdout + report.stderr
                    )
        if a.trace:
            trace = subprocess.Popen(
                [
                    "strace",
                    "-f",
                    "-c",
                    "-p",
                    str(r.process.pid),
                    "-o",
                    str(dest) + ".strace-summary.txt",
                ],
                stderr=subprocess.PIPE,
            )
            time.sleep(10)
            trace.send_signal(signal.SIGINT)
            _, stderr = trace.communicate(timeout=5)
            Path(str(dest) + ".strace-summary-stderr.txt").write_bytes(stderr or b"")
            trace = subprocess.Popen(
                [
                    "strace",
                    "-f",
                    "-tt",
                    "-T",
                    "-e",
                    "trace=epoll_wait,epoll_pwait,poll,ppoll,read,recvmsg,futex",
                    "-p",
                    str(r.process.pid),
                    "-o",
                    str(dest) + ".strace.txt",
                ],
                stderr=subprocess.PIPE,
            )
            time.sleep(3)
            trace.send_signal(signal.SIGINT)
            _, stderr = trace.communicate(timeout=5)
            Path(str(dest) + ".strace-stderr.txt").write_bytes(stderr or b"")
        data["diagnostics"] = [
            json.loads(l.split(" ", 1)[1])
            for l in r.log_path.read_text().splitlines()
            if l.startswith("DIAGNOSTICS ")
        ]
        dest.with_suffix(".json").write_text(json.dumps(data, indent=2))
        print(a.label, data["resources"]["cpu_percent_one_core"], flush=True)
    finally:
        r.close()


if __name__ == "__main__":
    main()
