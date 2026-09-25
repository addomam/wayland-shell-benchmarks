# Historical Measured summary

These are the original five-run measurements, before the iced optimization. See [current CPU tables](CPU_TABLES.md) and [analysis](CPU_ANALYSIS.md). Dependency and source counts describe that earlier snapshot.

Warm OS caches, release Rust builds, five launches per implementation in rotating order. Median / nearest-rank p95 in milliseconds; with five samples p95 is the maximum. These are compositor observation boundaries, not photon or content-ready times.

| Implementation | Bar startup ms | Launcher control ms | Launcher pointer ms |
| --- | ---: | ---: | ---: |
| web | 388.9 / 408.6 | 75.4 / 76.8 | 16.7 / 36.1 |
| iced | 115.9 / 126.3 | 76.1 / 97.2 | 26.4 / 36.9 |
| quickshell | 166.2 / 167.7 | 85.4 / 202.1 | 36.3 / 37.0 |

Pointer timings use six clicks (three on each output) and include virtual-pointer tool invocation. Control-file timings include the 150 ms polling design.

| Implementation | Idle PSS MiB | Active PSS MiB | Idle CPU % one core | Active CPU % one core | Idle / active processes |
| --- | ---: | ---: | ---: | ---: | ---: |
| web | 439.6 | 511.0 | 4.57 | 17.93 | 50 / 50 |
| iced | 40.6 | 44.5 | 2.11 | 5.19 | 1 / 1 |
| quickshell | 93.1 | 122.7 | 0.97 | 1.53 | 1 / 1 |

PSS sums the application process tree, including WebKit children. Active shows all eight surfaces; idle shows two bars. Each sample window is five seconds. GPU allocations and compositor cost are not included. Raw RSS, samples, host load, commands, logs, and errors are retained in results/.
