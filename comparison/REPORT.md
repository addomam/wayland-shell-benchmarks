# Three shell runtimes on enclave

**Historical comparison:** these five-run results predate the iced event-driven refactor. See [CPU analysis](CPU_ANALYSIS.md) for the subsequent investigation and longer before/after measurements. Original measurements below are preserved. Source/dependency counts and advisory findings also describe that historical snapshot; they are not a current inventory of the refactored tree.

The three projects implement the common shell workload and pass its live Hyprland surface, output, pointer, workspace, and frontend-reload checks. Native iced used the least measured process memory and reached its bar layers first. Quickshell used the least sampled CPU and required the least application code. WebKit retained browser development/HMR and isolated web-process recovery, with substantially more process memory and background work in this implementation.

These are results for the supplied prototypes and pins, not universal toolkit rankings. The [contract](PARITY.md), [commands](../README.md), [generated numerical summary](SUMMARY.md), and [raw measurements](results/performance.json) make the comparison repeatable.

## Workload and feature parity

Each implementation has two 44-pixel top bars with exclusive zones, a 560×390 launcher, a 320×240 separate menu, and a 620×360 transparent input experiment. All four declarations expand over both outputs; the three overlays start hidden. Launchers filter the same four sample entries and never launch an application. The bar supports workspace state and activation and updates a clock once per second. Fonts, grid/list layout, clock formatting, and visual styling are similar in purpose but not pixel-identical.

| Capability | WebKit/GTK | iced/wgpu | Quickshell |
| --- | --- | --- | --- |
| Layer surfaces and exclusive zones on both outputs | Passed | Passed | Passed |
| Height/zone changes, moving bar to bottom | Passed | Passed | Passed |
| Add/remove arbitrary declarations; switch page | Passed | Passed | Passed |
| Malformed JSON retains working layout | Passed | Passed | Passed |
| Launcher/menu/input overlay show and hide | Passed | Passed | Passed |
| Real Apps and Close clicks on both outputs | Passed | Passed | Passed |
| Workspace IPC events and actual workspace-button clicks | Passed | Passed | Passed |
| Launcher filtering and Escape | Passed; screenshot inspected | Passed; screenshot inspected | Passed; screenshot inspected |
| Transparent pass through and interactive island, scales 1 and 2 | Passed | Passed | Passed |
| Independent frontend edit, host retained | HTML/CSS reload | Native descriptor reload | QML generation reload |
| Same native surfaces after frontend edit | Yes | Yes | No; dynamic objects rebuilt |
| Ordinary-window preview | GTK windows | xdg windows | FloatingWindow |
| Isolated renderer recovery | Web process terminated and recovered | No separate renderer process | No separate renderer process |
| Physical output hotplug | Implemented, not physically tested | Polled through Hyprland, not physically tested | Implemented, not physically tested |

JSON declarations have the same geometry/state fields. Their `page` selects an HTML document, an iced JSON content descriptor, or a QML component. A native descriptor can select the provided native templates and change their title/color; arbitrary new widget code requires compiling Rust. QML can define new native UI components. HTML/JS execution and Vite HMR are browser-specific capabilities. This is common-workload parity with these explicit architectural differences, not a claim that native templates implement a web engine.

Evidence: [functional tests](results/functional.json), [real pointer tests](results/interactions.json), [frontend tests](results/frontend.json), and [keyboard/workspace/recovery tests](results/keyboard.json). Launcher and input screenshots are retained beside the logs. The frontend test changed/restored actual files. It verified native addresses before/after and, for Vite, observed the DOM's computed color change from `rgb(32,35,50)` to `rgb(48,64,80)` without surface replacement.

Ordinary-window previews were exercised in the current Hyprland session; the new implementations were not tested in a separate GNOME session. The final web-process termination check retained the Rust host and restored the bar layer in about306ms in one trial; this is recovery evidence rather than a statistically sampled recovery benchmark.

## Measurements

Tests ran on a 4-core/8-thread Intel i7-1068NG7 with NixOS 26.11 and Hyprland 0.56.2. The internal 2560×1600 display used scale 2; the external 2560×1440 display used scale 1; both were 60 Hz. See [environment.txt](results/environment.txt) and the captured monitor/compositor metadata in each result file.

Five process starts per implementation ran in rotating order, with warm OS caches and no concurrent builds or other test shells. Rust binaries were release builds; Quickshell was the pinned distribution executable. Nix evaluation/download/build time was excluded. No cold-boot cache purge or power/frequency tuning was performed.

| Implementation | Startup median / p95 ms | Launcher control median / p95 ms | Pointer launcher median / p95 ms |
| --- | ---: | ---: | ---: |
| WebKit/GTK | 388.9 / 408.6 | 75.4 / 76.8 | 16.7 / 36.1 |
| iced/wgpu | 115.9 / 126.3 | 76.1 / 97.2 | 26.4 / 36.9 |
| Quickshell | 166.2 / 167.7 | 85.4 / 202.1 | 36.3 / 37.0 |

Startup ends when both bar entries have the configured height and nonzero compositor alpha. Launcher latency ends at its configured size and nonzero alpha. Those observations do not establish that the frontend has finished painting, that all workspace data is populated, or that photons have reached the panel. Sampling polls Hyprland at 10 ms intervals and includes IPC overhead and normal compositor animations. Pointer timings include starting `wlrctl`, with six clicks per implementation (three per output). Control timings include the shared 150 ms file-poll interval. With five startup samples, nearest-rank p95 is just the maximum. The small pointer differences do not support a strong responsiveness ranking.

| Implementation | Idle PSS MiB | All surfaces visible PSS MiB | Idle CPU % of one core | Active CPU % of one core | Processes idle / active |
| --- | ---: | ---: | ---: | ---: | ---: |
| WebKit/GTK | 439.6 | 511.0 | 4.57 | 17.93 | 50 / 50 |
| iced/wgpu | 40.6 | 44.5 | 2.11 | 5.19 | 1 / 1 |
| Quickshell | 93.1 | 122.7 | 0.97 | 1.53 | 1 / 1 |

Each idle/active sampling window lasted five seconds, after a settling interval. PSS is summed over the complete observed application process tree, including WebKit's web/network processes, bubblewrap helpers, and D-Bus proxies. There were no memory-read errors. Summed RSS was approximately 1696/1817 MiB for web, 69/73 MiB for iced, and 131/162 MiB for Quickshell (idle/active); RSS double-counts shared pages, so PSS is the more useful comparison here. Raw samples retain both, plus process names, mapped libraries, and host CPU activity.

The web implementation eagerly loads eight WebViews, including hidden overlays. Iced creates native windows only when visible and destroys hidden overlays; Quickshell holds QML objects while hidden. These allocation strategies deliver the common visible workload but explain some startup/memory differences. Retained GPU buffers, VRAM, compositor CPU, and driver allocations outside process PSS are not measured. The normal renderer defaults were used rather than forcing an identical graphics backend. GTK emitted a Vulkan device initialization warning; all three loaded the host Mesa stack and rendered successfully. This is not a controlled GPU efficiency comparison.

## Dependency and supply-chain evidence

All projects pin the same Nixpkgs revision. Both Rust projects have Cargo.lock checksums; Vite has package-lock integrity data and is development-only. Quickshell adds no third-party QML packages. The [scan artifact](results/supply-chain.json) records exact packages, sources/checksums, OSV responses, source counts, and measured store roots. The [inventory script](supply_chain.py) can repeat it.

| Indicator | WebKit/GTK | iced/wgpu | Quickshell |
| --- | ---: | ---: | ---: |
| External Cargo packages in lockfile | 11 | 379 | 0 |
| npm packages in lockfile, including optional platforms | 64, development only | 0 | 0 |
| Approximate mapped-runtime Nix closure paths | 731 | 76 | 348 |
| Approximate closure NAR size MiB | 3496 | 1112 | 2148 |
| Application source/asset lines counted | 1591 | 962 | 318 |
| Application source lines containing `unsafe` | 30 | 0 | 0 |

These columns count different things. Quickshell's zero Cargo/npm dependencies says nothing about Qt's C++ dependency graph. Cargo lockfiles include optional/platform dependencies beyond the executing Linux path. Nix closure totals include transitive references of measured mapped package roots, excluding system-wide path and font-cache aggregators that would otherwise count much of the desktop. They are approximate dependency-footprint indicators, not exact deployment sizes, audited lines of code, or vulnerability counts. Fonts, plugins, multimedia libraries, drivers, and locale packages remain part of those closures.

The 2026-09-24 [OSV batch scan](https://google.github.io/osv.dev/post-v1-querybatch/) returned no advisories for the web project's Cargo/npm versions. It returned three findings for iced:

- `lru 0.16.4`: [RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html), a panic-safety memory-safety issue, fixed in 0.18.2. The actual renderer chain is `cryoglyph → iced_wgpu → iced_renderer`. The advisory requires specific panic/unwind/key-drop conditions; exploitability in this UI was not established.
- `paste 1.0.15`: [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html), maintenance status.
- `ttf-parser 0.25.1`: [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html), maintenance status. The latter two are not demonstrated vulnerabilities in this application.

Cargo/npm scanning omits native toolkit security. The pinned WebKitGTK 2.52.4 predates fixes listed in [WSA-2026-0005](https://webkitgtk.org/security/WSA-2026-0005.html), including fixes in 2.52.5/2.52.6. Qt 6.11.0 is within the version range in [CVE-2026-76151](https://www.qt.io/blog/security-advisory-cve-2026-76151), an HTTP client denial-of-service issue fixed in 6.11.2 or by patch. These are upstream version matches, not a complete audit of Nix backports or proof that the local-only demo exposes the affected paths. Update and revalidate the toolkit pins before turning this baseline into a daily shell.

The web implementation has the largest custom unsafe boundary: raw GTK/WebKit pointers and C callbacks. Its trusted-page bridge is narrow, but trusted local pages can import scripts. Web processes are sandboxed separately and recoverable. Iced has no custom unsafe application code and its descriptors cannot run arbitrary script, but the renderer, fonts, Wayland adapter, and drivers remain part of its trust base. QML is powerful trusted shell code with access to Quickshell capabilities; the entire QML/Qt UI shares one process. None of these dependency observations justifies calling a stack universally safe.

## Developer ergonomics and discovered failures

Web development has the broadest live-editing surface: HTML/CSS/JS and ordinary Vite HMR. Its runtime/bridge code is larger, and raw FFI lifecycle correctness needs work before reuse. It pays for eight eager browser views in this workload.

The iced version keeps native application code in typed Rust and fits the explicit update/reconcile model. Its descriptor reload is useful for content/color iteration, but layout/widget changes require rebuilding. The custom shell adapter uses action macros and needed iced's Wayland feature even though it owns a separate event loop. The first input-region implementation incorrectly used declared width 0 for stretched windows; the real two-scale pointer test found it. The fix queries allocated logical size and updates on resize.

Quickshell needed much less application source and no application native compile step. Its scene/window primitives fit this task directly. Dynamic QML pages needed an explicit `qmldir` for sibling types and explicit file watchers; full engine reload rebuilt dynamic surfaces. Root QML parse fallback and malformed dynamically loaded pages have different behavior: the latter can produce an empty generation until corrected. JSON validation still preserves the working layout. Testing also found a Close button below the fixed launcher height; moving it to the header fixed the issue.

The shared harness initially used cached global coordinates. This compositor changes usable-area placement when output focus changes, so tests now re-read bar/window placement before clicking and restore every output's original workspace. Exclusive launchers did not accept another click on the underlying bar to dismiss; Close/Escape worked. A compositor-generated configuration warning overlay, Fontconfig warnings, and portal warnings were present. They are documented conditions of this desktop, not evidence of a clean isolated lab session.

Source counts indicate application maintenance surface, not total ecosystem complexity or agent effort. Agent token consumption, keystrokes, and implementation wall times were not instrumented, so no numerical agentic-productivity ranking is claimed.

## What to do next

For continued shell UI exploration, Quickshell is the most convenient native iteration baseline here; iced is attractive when typed native code and lower measured host memory matter more than full UI hot reload. Keep WebKit when web authoring and browser HMR are requirements, and first reduce eager views and replace unsafe FFI with a disciplined wrapper. The pointer timing differences are too small and the endpoints too indirect to override those engineering tradeoffs.

Before a stronger performance claim, collect presentation-feedback or high-speed-camera click-to-display latency, GPU memory/time, longer idle traces, cold-cache starts, and more samples in a session without the configuration warning overlay. Test physical hotplug and renderer failure handling separately. Keep the same scenario definitions while adding services so functionality growth does not silently change the comparison.
