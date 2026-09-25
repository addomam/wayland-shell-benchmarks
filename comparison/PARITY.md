# Shared implementation and comparison contract

The three projects implement a shell client under the same Hyprland session. This checklist defines the common workload before collecting results; differences and failed checks must remain visible in the final report.

## Required behavior

- One 44 logical-pixel top bar per selected output, anchored top/left/right, exclusive zone 44.
- Bar: Apps button, Menu button, workspace buttons/current workspace, brand, 1 Hz clock.
- Launcher: 560×390 overlay, exclusive keyboard focus, four sample entries (Terminal, Files, Browser, Settings), case-insensitive filtering, close button/Escape. Sample entries do not execute applications.
- Menu: separate 320×240 overlay, top/left anchors, left margin 12, close button. No extra exclusive zone.
- Input experiment: 620×360 transparent overlay with a centered 250×160 interactive region. Transparent surroundings pass input through. Button closes it.
- JSON desired surfaces with stable IDs; create/update/destroy, invalid configuration retains working state; output `*` expands over actual connector names. Honor layer, anchors, dimensions, exclusive zone, margins, keyboard mode, visibility, and output selector.
- Change height/zone/anchors, add/remove a surface, and switch its content without restarting the host. Each renderer supports its own frontend format; arbitrary HTML/JS execution is specific to WebKit. Native content/theme reload and QML reload are compared explicitly to browser HMR rather than called identical.
- Workspace state/events from Hyprland IPC, plus click-to-activate. No system service scope expansion.
- Launcher/menu actions target the originating output. A local control file accepts `toggle <id>`, `reload`, `workspace <id>`, and `quit`. The original implementations polled at 150 ms; the subsequent iced CPU optimization uses inotify instead. Report that intentional transport/latency difference. Config changes debounce at least 250 ms. Log CREATE/DESTROY/READY/TOGGLE/CONFIG/WORKSPACES for diagnostics.
- Real layer mode required; ordinary window preview where supported. Renderer crash recovery and failure isolation are recorded as architecture-specific capabilities, not simulated equivalents.

## Configuration boundary

Match the existing `webshell-prototype/surfaces.json` field names and values. A `page` selects renderer-specific content: HTML path for WebKit, a native page descriptor for iced, QML page/component for Quickshell. Provide the four examples and a generic content example so arbitrary surface declarations can be tested. Keep all defaults and dimensions above equal. Provide a locked Nix dev environment, release build/run commands, and a control-file option/environment setting.

## Measurement rules

Use release Rust binaries and the pinned Quickshell binary directly, excluding Nix evaluation and dependency downloads. Run sequentially on the same outputs and compositor. Record exact versions, output geometry/scales/refresh, renderer backend, and process tree.

Measure repeated warm startup to observable bar layer mapping (compositor observation, not a claim of first displayed photon); control-to-visible launcher mapping separately from real pointer interaction where possible; workspace action/event response with its observation boundary stated. Report sample count, median and p95, raw samples, polling resolution, and failures. Never mix log-ready timestamps with first-frame presentation claims.

Sample total application process-tree RSS/PSS and CPU over a fixed idle interval and with overlays visible. Include WebKit children. Report graphics/driver and GPU memory limitations, cache state, and unrelated desktop activity. Avoid absolute rankings from incomparable measurement endpoints.

Inventory lockfiles, direct/transitive dependencies, Nix runtime closures, native unsafe/FFI exposure, renderer process isolation, upstream provenance, and available vulnerability evidence. Dependency counts and closure sizes are exposure indicators, not vulnerability counts or proof of security.

Record developer ergonomics from actual implementation work: source/config size, rebuild/reload workflow, errors/workarounds, unsafe code, API fit, and limits. Do not invent agent token/effort timings that were not measured.
