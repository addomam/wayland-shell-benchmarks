# Native iced shell

A native counterpart to `../webshell-prototype`, using iced 0.14, iced_wgpu (through iced's `wgpu` feature), and iced_exwlshell 0.20.1. Hyprland remains the compositor. Outputs come from Wayland events; workspace integration uses Hyprland IPC.

## Build and run

```sh
cd iced-shell
nix develop path:. -c ./build.sh
./target/release/iced-shell --layer --config surfaces.json --control /tmp/iced-shell-control
# Ordinary xdg windows on the same Wayland session:
./target/release/iced-shell --preview
```

The binary embeds library search paths from the pinned Nix development shell. Keep the Nix store dependencies available. Build artifacts are excluded by `.gitignore`; when running from an untracked directory, Nix may copy build artifacts while evaluating the flake. Avoid concurrent Nix evaluations during builds.

```sh
printf 'toggle launcher\n' > /tmp/iced-shell-control
printf 'workspace 2\n' > /tmp/iced-shell-control
printf 'reload\n' > /tmp/iced-shell-control
printf 'quit\n' > /tmp/iced-shell-control
```

Control commands are detected by inotify file events. Use trailing whitespace to repeat the same command. `toggle` targets the lexicographically first matching output; UI actions target the originating output.

## Configuration and content

`surfaces.json` uses the shared surface schema. `page` references a local JSON descriptor relative to the project working directory. The native renderer supports `bar`, `launcher`, `menu`, `input-test`, and `generic` page kinds, a title, and a `#RRGGBB` background. Edit `pages/*.json` to update existing widgets without replacing the Wayland surface. This is native content/theme reload; changing widget implementation requires a Rust rebuild. There is no HTML or JavaScript engine.

Configuration and descriptor changes trigger an inotify watcher with a 250 ms quiet-period debounce; there is no periodic filesystem scan. Parsing and validation complete before reconciliation. Invalid JSON, duplicate IDs, invalid page descriptors, and invalid layer dimensions retain the working shell. `output: "*"` expands to Wayland connector names. Output additions, updates, and removals arrive through the adapter’s shell-event subscription. Disconnected outputs remove instances; a temporary headless output was added and removed successfully in the [CPU regression tests](../comparison/results/cpu/output-events.json). Physical cable hotplug remains unverified.

`SurfaceManager` owns stable logical IDs and iced window IDs. A page-only change updates native widget content in place. Layer geometry/property changes recreate the native surface. Toggling destroys/recreates the underlying surface while retaining the configuration declaration. A subsequent config reconciliation applies declared visibility. The initializer creates layer roles before first rendering through iced_exwlshell. Ordinary preview uses xdg toplevels.

## UI and experiments

Bars reserve 44 logical pixels and contain Apps, Menu, workspace buttons, and a clock. Workspace events come from Hyprland's event socket; typed native actions issue a restricted workspace dispatch through its command socket. The launcher filters four sample entries; these close it without launching programs. Escape and Close dismiss overlays.

The menu is a separate native layer surface, avoiding clipping at the bar boundary. The input experiment creates a transparent 620×360 overlay with one 250×160 interactive island. `SetInputRegion` creates a real Wayland input region. The shared graphical tests confirmed transparent pass through and interactive clicks on both scale-1 and scale-2 outputs, including stretched windows. No DOM hit testing is involved.

All UI and rendering execute in one process. There is no isolated web process to recover; a renderer panic or process crash terminates this shell. This is a documented comparison difference, not a simulated recovery mechanism.

## Tests and comparison

`build.sh` builds the release binary, runs parser/trust validation tests, and checks formatting. See `../comparison/PARITY.md` for the common workload and the comparison report for measured graphical behavior, latency, process-tree resources, and dependency exposure. `READY` means native create action processed, not first displayed photon. Benchmarks use compositor layer observation instead.

Official upstream API reference: <https://docs.rs/iced_exwlshell/0.20.1/>; source: <https://github.com/waycrate/exwlshelleventloop/tree/v0.20.1>.

## Implementation findings

Initial release compilation required iced's `wayland` feature even though iced_exwlshell owns the active event loop: the iced facade still brings iced_winit, whose platform selection otherwise rejects the build. GPU rendering uses iced_wgpu; the lockfile also contains winit and multiple Smithay versions through upstream dependencies.

Native smoke checks under the available Hyprland session mapped one 44-pixel bar on each of two outputs (1280 and 2560 logical pixels wide), a 560×390 launcher, and the input-test overlay. Ordinary xdg preview windows also mapped. Both runs exited on the control command. These observations establish mapping, not input passthrough or first-frame latency; see the common test results for those checks.

The completed [comparison report](../comparison/REPORT.md) records subsequent passing topology, pointer, workspace-button, filtering/Escape, and descriptor-reload tests, plus five measured release launches. It includes the allocated-size input-region bug found and fixed during testing.

The shared advisory scan found `RUSTSEC-2026-0253` in locked `lru 0.16.4`, reached through `cryoglyph 0.1.0 → iced_wgpu 0.14.0`. Keep this dependency finding visible when comparing supply-chain risk; see the shared scan artifact for advisory details and dates. Updating to a fixed incompatible major release needs upstream compatibility validation. No application Rust code uses unsafe blocks or raw FFI, but its native dependencies do.

A stretched input overlay exposed a bug during the shared pointer test: a configured width of zero means the compositor chooses the width, so it cannot be used to calculate the interactive island's position. The runtime now queries the allocated logical window size and updates the region on resize. Unit tests cover 620, 1280 and 2560 logical-pixel allocations; real pointer results are recorded in the shared interaction report.

## CPU diagnostics

The runtime has no shared periodic tick. One global clock sleeps until the next second boundary; one Hyprland event stream refreshes its cache only on relevant events. Clock redraws target the bars. Hidden overlays have no native window or per-overlay subscription. The input region is initialized after native window creation and updated on resize.

```sh
SHELL_DIAGNOSTICS=1 ./target/release/iced-shell
# Static single text surface: no clock, filesystem watcher, workspace subscription, or hidden declarations.
SHELL_DIAGNOSTICS=1 SHELL_STATIC=1 SHELL_STATIC_OUTPUT=eDP-1 ./target/release/iced-shell
# Controlled experiments; normal defaults preserve the comparison workload:
SHELL_CLOCK=0 ./target/release/iced-shell    # no clock
SHELL_CLOCK=60 ./target/release/iced-shell   # minute clock
SHELL_NO_WATCH=1 SHELL_NO_WORKSPACES=1 ./target/release/iced-shell
# Optimized profiling binary with symbols:
nix develop path:. -c cargo build --profile profiling --locked
```

Diagnostic counters print every ten seconds from a separate reporting thread; they do not send UI messages. Normal runs disable them. `SHELL_NO_WATCH` also disables the control-file listener; stop those diagnostic runs with SIGTERM. See [CPU analysis](../comparison/CPU_ANALYSIS.md) for counter definitions, profiles, raw results and limitations. The two adapter crates are pinned in `vendor/` for boundary instrumentation and a small multi-window redraw-scope extension.
