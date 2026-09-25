# CPU measurements

A subsequent [release validation](RELEASE_VALIDATION.md) rebuilt all three versions and repeated ten rotating measurements each. This document preserves the original optimization investigation.

CPU is percent of one core. Memory is summed process-tree MiB. Values below are medians.

| Version | n | Startup ms | Idle PSS | Active PSS | Idle CPU % | Active CPU % |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| iced before | 3 | 127.544 | 41.158 | 46.320 | 2.057 | 5.308 |
| iced after | 10 | 127.108 | 42.380 | 43.099 | 0.332 | 0.332 |
| Quickshell | 10 | 172.661 | 93.392 | 121.030 | 0.961 | 1.498 |
| WebKit | 10 | 407.248 | 441.912 | 512.070 | 3.388 | 5.598 |

## Memory, process counts and latency tails

| Version | Idle RSS MiB | Active RSS MiB | Processes idle/active | Startup p95 ms | Launcher control median / p95 ms |
| --- | ---: | ---: | --- | ---: | ---: |
| iced before | 68.6 | 73.8 | 1/1 | 128.2 | 160.8 / 162.7 |
| iced after | 69.8 | 70.6 | 1/1 | 139.0 | 32.4 / 33.3 |
| Quickshell | 130.4 | 159.7 | 1/1 | 188.3 | 107.7 / 151.1 |
| WebKit | 1702.9 | 1823.2 | 50/50 | 428.7 | 128.2 / 170.1 |

## Isolated experiments

Rates sum all surfaces. Dispatch returns are UI-loop iterations, not total process thread wakeups. First diagnostic interval (startup) is excluded.

| Experiment | CPU % | Dispatches/s | Views/s | Redraw requests/s | Presents/s | Thread context switches/s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| instrumented-before | 2.089 | 49.179 | 13.340 | 13.340 | 13.340 | 163.855 |
| instrumented-before-active | 5.268 | 62.949 | 53.399 | 53.399 | 53.399 | 331.643 |
| static-initial | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.100 |
| static-complete | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.124 |
| clock-minute | 0.000 | 0.100 | 0.029 | 0.029 | 0.029 | 0.455 |
| clock-second | 0.299 | 7.000 | 2.000 | 2.000 | 2.000 | 24.031 |
| workspaces-only | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.099 |
| watcher-only | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.100 |
| normal-idle | 0.299 | 6.500 | 2.000 | 2.000 | 2.000 | 22.268 |
| normal-active | 0.348 | 7.175 | 8.000 | 2.000 | 2.000 | 24.036 |
| active-no-clock | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.099 |
| launcher-only | 0.324 | 8.000 | 0.000 | 2.000 | 2.000 | 24.730 |
| backend-gl | 0.273 | 5.300 | 2.000 | 2.000 | 2.000 | 19.368 |
| profiled-idle | 0.323 | 7.157 | 2.000 | 2.000 | 2.000 | 24.050 |
| profiled-active | 0.348 | 7.136 | 8.000 | 2.000 | 2.000 | 24.809 |
| quickshell-idle | 0.899 | — | — | — | — | 74.140 |
| quickshell-active | 1.518 | — | — | — | — | 144.944 |

## perf stat

Separate 30-second windows. This host restricts perf to user-space events; its `context-switches:u` zero is not a valid scheduling count. Thread context switches above come from `/proc/PID/task/*/status`.

| Experiment | Task-clock ms | Cycles | Instructions | Instructions/s |
| --- | ---: | ---: | ---: | ---: |
| instrumented-before | 718.750 | 431776773 | 151167198 | 5038907 |
| profiled-active | 112.750 | 69973797 | 29156447 | 971882 |
| profiled-idle | 102.960 | 63860660 | 24286803 | 809560 |
| quickshell-active | 515.270 | 304751980 | 107679253 | 3589308 |
| quickshell-idle | 313.120 | 184253568 | 64603358 | 2153445 |
| static-initial | 0.410 | 129702 | 66222 | 2207 |
