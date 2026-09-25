# Release validation

This pass rebuilds and exercises the three versions intended for readers: the WebKit/GTK runtime, optimized iced/wgpu runtime, and Quickshell/QML runtime. It does not introduce another optimization or alter the workload.

## Build and graphical checks

All three builds passed. WebKit and iced each passed four Rust tests and formatting checks; Quickshell passed its configuration/output-expansion tests. The WebKit development dependencies were installed from the npm lockfile before testing Vite HMR. The iced adapter still emits two upstream unused-import warnings.

All eight graphical check groups completed with exit status zero:

| Group | Coverage |
| --- | --- |
| Shared functional | All three: layer bars/exclusive zones, config changes, add/remove/page-change host survival, invalid-config retention, launcher/menu/input-overlay mapping |
| Pointer | All three: Apps/Close clicks, transparent and interactive input regions on both output scales |
| Frontend | Local content reload for all three; WebKit Vite CSS HMR with native surface addresses retained |
| Keyboard/workspace | All three: filtering, Escape, actual workspace-button clicks; WebKit web-process recovery with host survival |
| iced watcher | Nonstandard descriptor paths, symlink targets, atomic replacement, malformed content retention/recovery |
| iced outputs | Temporary real Hyprland headless-output addition/removal |
| iced preview | Ordinary xdg windows and launcher show/hide lifecycle |
| Quickshell control | 100 consecutive launcher toggles |

The captured launcher images were inspected: each displays the typed `Files` filter and only the matching entry. Physical cable hotplug was not tested. The output-add/remove test uses a compositor-created headless output. Earlier GNOME preview observations are not presented as newly repeated GNOME tests.

[Build records](results/release/build-checks.json), [suite statuses](results/release/checks.json), [functional results](results/release/functional.json), and [raw evidence guide](results/release/README.md) identify the checks and artifacts. Existing scripts temporarily use fixed output paths; the new wrapper preserves the historical files and copies fresh artifacts into the release directory.

## Benchmark method

The benchmark uses ten rotating starts per implementation, 30-second idle and active windows, the same two-second idle and one-second active settling intervals, direct release executable launches, and 10 ms compositor observation. No diagnostic counters/profilers run during the benchmark. Active means all eight surfaces are visible, without injected interaction; it is not a typing or animation stress test.

Startup measures layer geometry/nonzero alpha, not completed frontend painting or displayed photons. Control-file launcher latency is distinct from the real pointer tests. Different control transports, UTC/local clock formatting, renderer backends, and process isolation remain documented differences. CPU sums the application process tree as a percentage of one core. Compositor CPU/GPU memory are excluded; RSS double-counts shared mappings and PSS apportions them.

The host, compositor and output configuration are captured in the raw JSON. Runs use normal desktop power management and warm caches, not a frequency-locked laboratory environment. Do not interpret small changes between campaigns as improvements in unchanged code.

## Fresh results and comparison with the earlier campaign

All **30 runs completed with zero benchmark failures**. [Full tables](RELEASE_TABLES.md) include RSS, process counts, latency tails, and host load; [raw samples](results/release/performance.json) preserve the individual runs.

| Implementation | Startup ms | Idle / active PSS MiB | Idle / active CPU % |
| --- | ---: | ---: | ---: |
| Optimized iced | 126.8 | 42.4 / 43.1 | 0.33 / 0.33 |
| Quickshell | 168.2 | 94.4 / 122.2 | 0.96 / 1.49 |
| WebKit | 407.5 | 441.2 / 511.2 | 3.39 / 5.51 |

These reproduce the earlier optimized campaign closely: CPU was 0.332/0.332%, 0.961/1.498%, and 3.388/5.598%, respectively. Startup was 127.1, 172.7, and 407.2 ms. Quickshell PSS increased by roughly 1 MiB; other median memory differences are small. No runtime change was required. The optimization's large reduction relative to the original iced baseline survives this fresh pass.

Control-launcher medians are 32.4 ms iced, 112.4 ms Quickshell, and 44.5 ms WebKit. WebKit's previous median was 128.2 ms. Its code/control polling is unchanged, and this measurement begins after a fixed-duration resource window: timer phase and scheduling can change the observed delay. This is not evidence of a new WebKit input-latency optimization. Real pointer samples remain separate in the regression artifacts. Quickshell's control p95 is 202.4 ms; the slower successful sample remains included.

The original CPU investigation's Quickshell timeout remains documented; this new campaign did not reproduce it. This validates the released prototypes on this machine, not a framework-wide ranking or proof that all races are eliminated. Quickshell/WebKit have not received the same event-driven cleanup as iced.

## Provenance and limitations

The repository had no usable Git history in either the workspace or host view, so the old iced source could not be recovered from a commit. Local old binaries are not part of the release. The [CPU investigation](CPU_ANALYSIS.md) retains the baseline measurements and explanation of the polling/redraw optimization; the current release focuses on the three buildable implementations.

Source hashes, release binary hashes, and the resolved Quickshell Nix store path are saved with the new results. These identify this build without requiring publication of host-specific binaries. All three remain runnable through the root README commands. Final checks confirmed the source and binary hashes were unchanged throughout measurement, all 30 resource windows per state lasted at least 30 seconds, and documentation links resolve. No prototype processes or layers remained. Original workspaces (eDP-1: 1, DP-3: 3) and eDP-1 focus were restored; final monitor/layer JSON is included with the evidence.

Portal activation warnings and a WebKit child Vulkan-device probe warning appeared again, as they did in the previous measurement logs. They did not prevent the graphical tests or mapping; this pass does not establish a new renderer attribution for every WebKit subprocess. The earlier detailed backend analysis remains separate.

This is functional/performance revalidation, not a new supply-chain audit. The original report's dependency/advisory inventory remains dated historical evidence.

## Repeat

From the root, after the [build commands](../README.md#build-and-try-it):

```sh
nix develop path:./comparison -c python3 comparison/release_check.py
nix develop path:./comparison -c python3 comparison/bench.py --runs 10 --sample-seconds 30 --output local-results/performance.json
nix develop path:./comparison -c python3 comparison/release_summary.py local-results/performance.json --output local-results/summary.md
```

The summary tool reads only the supplied file; it does not silently pool old or supplemental runs. Archive `comparison/results/release/` before repeating the graphical wrapper if you want to retain this campaign unchanged.
