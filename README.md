# Three ways to build a Wayland panel

An experiment in the space occupied by Quickshell: how much desktop-shell functionality can a small Rust runtime provide, and what do we gain or give up by using a browser, iced/wgpu, or Qt Quick?

These are **three shell clients running under Hyprland**, not compositors or complete desktop environments. They implement the same small workload: per-output bars, workspace buttons, a launcher, a separate popup menu, transparent input regions, and runtime configuration reload. Launcher entries are samples and do not start applications. There is no audio, tray, notification, or lock-screen service.

| Implementation | Frontend | Native stack | Development model |
| --- | --- | --- | --- |
| [WebKit](webshell-prototype/README.md) | HTML/CSS/JavaScript | Rust → GTK4 → gtk4-layer-shell; WebKitGTK | Asset reload or an independent HTTP/HMR server |
| [iced](iced-shell/README.md) | Rust widgets + JSON page descriptors | iced → iced_wgpu; iced_exwlshell → Wayland | Descriptor reload; widget changes require compilation |
| [Quickshell](quickshell-shell/README.md) | QML | Quickshell → Qt Quick → Wayland | QML reload in the same host; dynamic windows are reconstructed |

Start with the **[architecture diagrams](ARCHITECTURE.md)**, including the iced event flow before and after optimization. The [shared workload contract](comparison/PARITY.md) explains what “equivalent” means and where the implementations differ.

## What we found so far

Fresh release-validation measurements on one Intel MacBook, with two outputs under Hyprland:

| Release implementation | Startup ms | Idle / active PSS MiB | Idle / active CPU % |
| --- | ---: | ---: | ---: |
| Optimized iced | 126.8 | 42.4 / 43.1 | 0.33 / 0.33 |
| Quickshell | 168.2 | 94.4 / 122.2 | 0.96 / 1.49 |
| WebKit | 407.5 | 441.2 / 511.2 | 3.39 / 5.51 |

Medians from **ten complete rotating runs per implementation, with zero failures** in this release pass. CPU is percent of **one core**, summed across the process tree over 30-second windows. Active means all eight surfaces visible, without synthetic typing or pointer motion. Startup ends at compositor-observed layer mapping, not first painted frame. See the [release-validation report](comparison/RELEASE_VALIDATION.md), [full tables](comparison/RELEASE_TABLES.md), and [raw evidence](comparison/results/release/README.md).

The earlier unoptimized iced baseline measured 2.06% idle / 5.31% active CPU. It is retained as historical evidence in the CPU investigation; the three current implementations above are the buildable release workload.

The iced prototype originally sent a message every 150 ms, causing all visible surfaces to rebuild and redraw. Event-driven sources and targeted redraws removed most of that cost. A static iced surface now blocks with no observed recurring UI work. This establishes an avoidable prototype cost, not a universal toolkit ranking. Quickshell and WebKit have not received the same polling cleanup; clocks, backends, process isolation, and reload semantics also differ.

All builds, unit/configuration checks, and eight graphical regression groups passed again in the release validation. Read the [CPU investigation](comparison/CPU_ANALYSIS.md) for profiles, failures, methodology, and remaining uncertainty. The [original comparison](comparison/REPORT.md) retains the earlier five-second measurements and dated dependency/security inventory. Those original figures are historical, not current benchmark claims.

## Build and try it

Requirements: Linux with Nix and flakes enabled, network access for dependencies, a working graphics driver, and a Wayland session. The recorded environment is NixOS/x86_64-linux. Application flakes also declare aarch64-linux, but it has not been tested. Rust builds use locked Cargo dependencies inside pinned development shells; they are not packaged hermetic `nix build` derivations.

Run from the repository root:

```sh
nix develop path:./webshell-prototype -c bash -c 'cd webshell-prototype && cargo build --release --locked && cargo test --locked'
nix develop path:./iced-shell -c bash -c 'cd iced-shell && ./build.sh'
nix build path:./quickshell-shell --out-link quickshell-shell/result
```

Run **one at a time** in your graphical session:

```sh
(cd webshell-prototype && ./target/release/webshell --layer --control /tmp/webshell.control)
(cd iced-shell && ./target/release/iced-shell --layer --control /tmp/iced-shell.control)
SHELL_CONTROL=/tmp/quickshell.control quickshell-shell/result/bin/quickshell -p quickshell-shell/shell.qml
```

Click Apps or Menu on the bar. Edit that project's `surfaces.json` to change topology. Stop with Ctrl-C, or write `quit` to its control file. Keep the Nix store dependencies used by the Rust build available; copying the executable alone is insufficient.

Real panels require `zwlr_layer_shell_v1` (Hyprland, Sway, etc.). GNOME normally does not offer it. Use `--preview` for the Rust programs or `SHELL_PREVIEW=1` for Quickshell to try ordinary windows. Workspace integration and the automated graphical tests specifically target the recorded Hyprland API; another compositor does not provide that integration automatically.

## Reproduce and explore

- [Reproduction guide](REPRODUCING.md): tests, benchmark commands, prerequisites, and evidence boundaries.
- [Architecture](ARCHITECTURE.md): components, ownership, memory costs, and iced scheduling diagrams.
- [Release validation](comparison/RELEASE_VALIDATION.md) / [fresh tables](comparison/RELEASE_TABLES.md): rebuilt versions and repeated graphical/performance checks.
- [CPU analysis](comparison/CPU_ANALYSIS.md) / [investigation tables](comparison/CPU_TABLES.md): optimization experiments and before/after evidence.
- [Historical report](comparison/REPORT.md): initial parity tests, developer ergonomics, and dated supply-chain findings.
- [Raw evidence guide](comparison/results/cpu/README.md): profiles, counters, regression results, and unsuccessful attempts.

This is a research prototype, not a supported replacement desktop shell. The useful next comparison is to apply the same event-source discipline to Quickshell and WebKit, then compare equivalent clock/render workloads. There is no evidence here that warrants replacing iced with a custom renderer yet.

## License

The original work in this repository is dedicated to the public domain under [CC0 1.0 Universal](LICENSE). The vendored adapter in `iced-shell/vendor/` retains its upstream MIT license; see [its license](iced-shell/vendor/LICENSE).
