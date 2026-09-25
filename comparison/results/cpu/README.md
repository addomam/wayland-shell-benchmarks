# Raw CPU investigation evidence

- `before.json`: unmodified release, three launches, 30-second idle/active samples.
- `instrumented-before*`: baseline scheduling with counters; original binaries were retained locally under ignored `iced-shell/target/cpu-before/` (not included in a fresh clone) and hashed in `baseline-binaries.sha256`.
- `static-*`, `clock-*`, `workspaces-only`, `watcher-only`, `normal-*`, `active-no-clock`, `launcher-only`: isolated event/render/CPU experiments. JSON includes raw quarter-second memory samples and `/proc` context-switch deltas; logs include ten-second counter intervals.
- `profiled-*`, `quickshell-*`: independent perf windows; `.perf.data` holds samples, `.perf-report.txt` expanded call stacks, `.perf-stat.txt` hardware/software counters. A no-samples static profile is expected.
- `*-trace`: fresh runs under strace, with settled-window syscall-entry summaries. Traced CPU timings are not benchmark results. Initial sibling-attach attempts produced empty `instrumented-before.strace*` files under Yama; later traces launch the process as strace's child.
- `after.json`: rotating-order release benchmark; no profiler or diagnostic counters. One Quickshell timeout is retained; `after-quickshell-retry.json` supplies the tenth complete reference sample. `after-prewatch-fix.json` is an interrupted earlier pass, excluded from final statistics.
- `functional.json`, `regressions.json`, `output-events.json`, `watch-paths.json`, `preview.json`: live graphical regression evidence. The intermediate failed input-region test is retained separately; it led to replacing a removed periodic size retry with the native-created callback.
- `*-instrumentation.patch`: changes against the exact crates.io 0.20.1 sources. Empty `*-preexisting-cache.patch` files verify the starting Cargo cache was unmodified.

Do not compare UI-loop dispatch counters with whole-process context switches as if they were the same quantity. `perf` on this host records user-space-only events; `context-switches:u` is zero even while `/proc` records real scheduling. No kernel policy was changed.

`final-idle-trace.json` checks the final cached watcher implementation. `control-quickshell.json` records 100 consecutive commands with corrected nonces. `web-active-transient.json` is a separate activation-transient diagnostic. `final-desktop.json` records cleanup and restoration.
