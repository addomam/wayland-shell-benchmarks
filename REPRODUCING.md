# Reproducing the experiment

Use the [root build commands](README.md#build-and-try-it) first. Run from the repository root inside the graphical session, with `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`, and `HYPRLAND_INSTANCE_SIGNATURE` inherited. Do not start the harness from an unrelated SSH environment without arranging display/session access.

## What is pinned and what is not

Each project commits a `flake.lock`; Rust commits `Cargo.lock`, and the optional WebKit Vite workflow commits `package-lock.json`. Nix pins the development libraries/tools and Quickshell executable. Rust is built with Cargo inside that shell; downloads may be needed. The compositor, kernel, graphics driver, fonts supplied by the desktop, monitor layout, CPU power policy, and desktop activity also influence results.

The recorded host is an Intel i7-1068NG7 MacBook with Iris Plus graphics, NixOS, Hyprland 0.56.2, and two ~60 Hz outputs: eDP-1 at scale 2 and DP-3 at scale 1. See [environment evidence](comparison/results/environment.txt) and [renderer/profile details](comparison/CPU_ANALYSIS.md). Do not expect identical numbers on another machine. Hyprland workspace actions use the Lua dispatcher supported by this recorded version; older Hyprland versions may need an IPC adaptation.

Application flakes declare x86_64-linux and aarch64-linux; only x86_64 was exercised. The comparison-tools flake currently targets x86_64-linux. A working hardware driver is required; accidental software rendering changes the experiment substantially.

## Build checks without opening panels

```sh
nix develop path:./webshell-prototype -c bash -c 'cd webshell-prototype && cargo test --locked'
nix develop path:./iced-shell -c bash -c 'cd iced-shell && cargo test --release --locked && cargo fmt --check'
nix develop path:./quickshell-shell -c bash -c 'cd quickshell-shell && node test-config.js'
```

These validate code/configuration properties. They do not establish layer-shell, focus, input masking, or presentation behavior.

## Graphical tests and current benchmark

Stop other copies of these prototypes first. These tests open/close surfaces, reserve screen space, switch workspaces, and some move the pointer/type into overlays. Run sequentially while you are not interacting with the desktop. Published tests used two outputs; one-output runs cannot establish two-output parity.

The manifest uses the release binaries and `quickshell-shell/result/bin/quickshell`; build all three before testing. `wlrctl` and `grim` come from PATH in the comparison shell, not a hardcoded machine store path.

```sh
mkdir -p local-results
nix develop path:./comparison -c python3 comparison/bench.py --functional --output local-results/functional.json
nix develop path:./comparison -c python3 comparison/bench.py --runs 10 --sample-seconds 30 --output local-results/performance.json
```

The latter takes over 30 minutes. The harness rotates implementation order, preserves errors, and writes logs beside the JSON. `--only iced` (or `web`, `quickshell`) narrows a diagnostic run. Results contain raw samples, environment/output information, and endpoint definitions. No Nix startup/build time is included.

The release wrapper runs the shared graphical tests plus the iced watcher/output/preview checks and Quickshell repeated-control check while preserving historical artifacts:

```sh
nix develop path:./comparison -c python3 comparison/release_check.py
```

Fresh evidence goes under `comparison/results/release/`; archive that directory before another campaign. The wrapper requires WebKit’s locked npm dependencies for HMR.

The original specialized scripts below use fixed output paths under `comparison/results/` and may temporarily edit example assets. **Run them in a disposable copy/checkout** to preserve published evidence and your edits:

```sh
nix develop path:./comparison -c python3 comparison/interactions.py
nix develop path:./comparison -c python3 comparison/frontend.py
nix develop path:./comparison -c python3 comparison/keyboard.py
```

The frontend test additionally needs `npm ci` in `webshell-prototype` and the web development shell's Node/npm on PATH. Consult individual scripts before running the CPU regression tools: some target recorded output names, change focus, or create a temporary headless output. They are research scripts rather than a portable CI graphical test suite.

## CPU diagnosis

For counters, run the iced binary directly from its project directory:

```sh
cd iced-shell
SHELL_DIAGNOSTICS=1 ./target/release/iced-shell
# Substitute a connector present on your desktop:
SHELL_DIAGNOSTICS=1 SHELL_STATIC=1 SHELL_STATIC_OUTPUT=eDP-1 ./target/release/iced-shell
```

Counters report every ten seconds. Static mode removes application timers, filesystem watching, workspace state, and extra surfaces. Stop with Ctrl-C; its control watcher is disabled. Clock experiments use `SHELL_CLOCK=0`, `60`, or the normal one-second default. A diagnostic reporter adds its own ten-second wakeup; disable it for syscall quiescence measurements.

For scripted experiments, use a disposable checkout as above:

```sh
nix develop path:./iced-shell -c bash -c 'cd iced-shell && cargo build --profile profiling --locked'
nix develop path:./comparison -c bash comparison/cpu_matrix.sh
nix develop path:./comparison -c bash comparison/cpu_perf.sh
```

`perf` availability depends on host kernel/security settings. The recorded profiles use user-space-only events; no system policy was changed. Read [counter definitions and limitations](comparison/CPU_ANALYSIS.md) before interpreting them. `cpu_summarize.py` and `cpu_profile_summary.py` regenerate the specific recorded investigation's tables, including its supplemental Quickshell run; they are not generic aggregators for arbitrary new datasets.

## Historical evidence and reproducibility limits

- [Original report](comparison/REPORT.md): five starts, five-second resource samples, pre-optimization iced. Dependency/source counts and advisory checks are dated to that snapshot and are not a fresh audit of today's tree.
- [CPU analysis](comparison/CPU_ANALYSIS.md): fresh unmodified baseline, isolated experiments, profiles, optimized runs, and reference implementations with 30-second windows.
- [Raw CPU evidence index](comparison/results/cpu/README.md): includes failed/interrupted attempts and their interpretation. Raw artifacts contain host-specific paths and desktop/output details intentionally as provenance.
- The pre-optimization executables existed locally under ignored `iced-shell/target/cpu-before/`. Their hashes/results are retained, but a fresh clone does **not** contain those executables or a complete buildable old source snapshot. Readers can reproduce the current implementation and inspect the baseline evidence; exact historical source reproduction requires an independently preserved pre-refactor revision. Adapter instrumentation patches alone are insufficient.
- Nix closures were estimated from mapped-library roots, not produced by minimal app packaging. They are not download-size or security scores. Advisory findings require checking current upstream fixes and downstream backports before making deployment decisions.
- Original repository work is under [CC0 1.0 Universal](LICENSE). The vendored adapter retains its upstream MIT license in `iced-shell/vendor/LICENSE`.

## What has actually been verified

Live Hyprland tests established two-output layer mapping/exclusive zones, launcher/menu toggles, real pointer and keyboard interactions, workspace actions/events, topology/content reload, and transparent input regions. The optimized iced run additionally verified a real headless-output add/remove and ordinary xdg preview. Physical monitor unplug/replug and a fresh-machine installation were not verified. WebKit has its own web-process recovery test; iced and Quickshell do not provide equivalent process isolation.

The subsequent release validation rebuilds all three implementations and reruns graphical checks and the rotating benchmark. All 30 benchmark runs and all eight graphical check groups passed. See the [release report](comparison/RELEASE_VALIDATION.md); results are recorded separately under `comparison/results/release/`, and earlier CPU investigation results remain unchanged.
