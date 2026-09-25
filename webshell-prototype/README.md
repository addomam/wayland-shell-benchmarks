# WebShell runtime prototype

A Wayland shell **client**, built from Rust, GTK4, gtk4-layer-shell, and WebKitGTK 6.0. Hyprland remains the compositor. The runtime turns a JSON desired layout into one native GTK window and WebView per surface instance. HTML/CSS/JS owns presentation; Rust owns native surface lifecycle and a small trusted bridge.

## Run

From this directory, on Hyprland or another compositor offering `zwlr_layer_shell_v1`:

```sh
nix develop path:.
./build.sh
./target/debug/webshell --layer
```

`--preview` uses ordinary GTK windows, including under GNOME. `--layer` exits clearly if layer-shell is unavailable. The default [surfaces.json](surfaces.json) makes one bar, launcher, menu, and input-region test overlay per connected output. The launcher, menu, and input test start hidden. Click **Apps** or **Menu ▾** in a bar to open its counterpart on that bar's output.

The config can also be supplied with `--config /path/to/layout.json`. For repeatable development actions, `--control /tmp/webshell-command` watches that file for `toggle launcher`, `toggle menu`, `toggle input-test`, `reload`, `crash bar`, `workspace 3`, or `quit`. Write a different command between identical repeated commands so the file content changes. The control file is a local test aid, not a web API.

For a quick manual check, start with `--layer --control /tmp/webshell-command`, then run `printf 'toggle launcher\n' > /tmp/webshell-command`. Run `printf 'quit\n' > /tmp/webshell-command` to stop. The same commands work with `--preview`. Run from the project directory: relative page paths resolve there.

## Configuration

Each entry in `surfaces.json` has a stable `id` and a `page` (project-local path or HTTP(S) URL). `layer` is `background`, `bottom`, `top`, or `overlay`. `anchors` may contain `top`, `bottom`, `left`, and `right`. `width` and `height` are GTK logical pixels; omit `width` only when anchoring both left and right to stretch across an output. `height` is required. Optional `exclusiveZone` and `margins` also use logical pixels. `keyboardInteractivity` is `none` (default), `on-demand`, or `exclusive`. `output` defaults to `"*"` for every output, or takes a connector such as `"eDP-1"`. `visible` defaults to `true`.

Reload accepts a complete valid file at once. A bad JSON document, unknown field, duplicate ID, invalid geometry, or missing local page leaves the previous layout running. Page, zone, margin, and visibility changes update existing native instances. Width, height, layer, anchors, keyboard mode, output, and trusted-origin changes replace the affected native instance while retaining its logical ID. The file poll interval is 150 ms with a 250 ms stability debounce.

## Frontend development

Local pages such as `assets/bar.html` are loaded as `file://` URLs. Editing a file under `assets/` reloads **local** WebViews within one polling interval without recreating the native window. A loaded HTTP page is left to its own development server.

For actual Vite HMR, start Vite in one terminal:

```sh
nix develop path:.
npm ci
npm run dev
```

In a second terminal, keep the base config intact and point only the bar at the loopback page:

```sh
nix develop path:.
sed 's#assets/bar.html#http://127.0.0.1:5173/dev-bar.html#' surfaces.json > /tmp/webshell-dev.json
./target/debug/webshell --layer --config /tmp/webshell-dev.json
```

Edit `assets/style.css`. Vite updates CSS in the loaded WebView; the bar's native layer surface stays alive. `assets/dev-bar.html` exposes its computed background color through the document title so this can be checked in runtime logs. The Vite server binds only to `127.0.0.1`.

## Architecture and lifecycle

The original proof of concept used one `main.rs`, direct C FFI, a small INI parser, and an asset/config poll. It created a bar and hidden launcher from file paths. The current version retains direct FFI and its known lifecycle order, but uses Cargo solely for `serde` and `serde_json`. Vite is an optional development dependency. There is no GTK, WebKit, or compositor Rust crate.

`src/config.rs` parses and validates the complete JSON file into typed `SurfaceSpec` values. The boundary is **configuration → `Vec<SurfaceSpec>` → `SurfaceManager::reconcile`**, so a future TypeScript configuration frontend can replace JSON without changing native reconciliation. Stable logical IDs are config IDs; instances are keyed `id@connector` (for example, `bar@eDP-1`). `output: "*"` expands one declaration over GTK's `GdkMonitor` list; an explicit connector selects one output. The monitor list's `items-changed` signal triggers reconciliation when GTK reports hotplug.

`SurfaceManager` owns each `GtkWindow`, WebKit WebView, content manager, page URI, visibility, input rectangles, and chosen monitor. It initializes gtk4-layer-shell, namespace, monitor, layer, anchors, margins, exclusive zone, and keyboard mode **before** the window is realized or presented. The bar anchors top/left/right with zero layer width, so it stretches across its output; the launcher is a separate overlay window. Stable instance IDs survive a native window replacement.

Reconciliation is keyed by instance ID. Page URL, exclusive zone, margins, and visibility update in place. Size, layer, anchors, keyboard mode, monitor object, or transition between trusted and untrusted page origins recreate that instance's native window. This is intentionally a small desired-state diff rather than a general framework.

`src/hyprland.rs` reads workspace state and sends workspace activation over Hyprland's command socket. A dedicated thread listens to the event socket and signals the GTK main loop through a channel. The bar's `assets/shell.js` hides the WebKit script-message transport behind `shell.workspaces.get()`, `subscribe(callback)`, and `activate(id)`, plus the minimal `shell.surfaces.toggle(id)` and `shell.inputRegions.set(rects)` calls. Rust parses only typed bridge commands; there is **no arbitrary shell-command API**. The runtime sends structured workspace state and `{id, instance, output}` context events into each trusted WebView. The frontend can read `shell.context` after load.

Only local project pages and loopback HTTP pages receive a script-message handler. A config change from a trusted page to an arbitrary remote page recreates the WebView without that handler. Main-frame navigation away from a trusted page's local root or loopback origin is blocked. A trusted local page can still include remote scripts, so only load trusted frontend code. The bridge is a narrow privilege boundary, not a sandbox for hostile local content.

## Experiments and results on enclave

The initial web-runtime checks below ran on NixOS 26.11, Hyprland 0.56.2, and two real outputs (`eDP-1` and `DP-2`). Hyprland advertised `zwlr_layer_shell_v1` version 5. The locked Nix shell built the runtime; `cargo test --locked` passed its config, reconciliation, output-expansion, and bridge-trust tests.

| Experiment | Observed result |
| --- | --- |
| Layer startup and outputs | `bar@eDP-1` was 1280×44; `bar@DP-2` was 2560×44. Both appeared at layer level 2. Their output reservations each increased by 44 logical pixels. |
| Launcher | It appeared at overlay level 3 and hid again. A real virtual-pointer click on the DP-2 bar opened `launcher@DP-2`. |
| Config zone/height/layer | Changing the zone 44→50 updated reservations in place with no bar recreation. Height 44→56 recreated the two bar windows. Top→bottom moved both to layer level 1. |
| Add/remove/page | Adding `notes` created a new overlay, removing it destroyed that instance, and changing a page URI loaded it in the existing window. Malformed JSON logged one error and kept the previous layout. |
| Frontend reload/HMR | A local CSS edit reloaded local views without `CREATE`/`DESTROY`. With Vite, a CSS edit changed the bar DOM's computed color from `rgb(32, 35, 50)` to `rgb(39, 59, 97)` with one initial WebView load and no native recreation. |
| Workspace bridge | A DOM workspace-button click switched Hyprland 1→2→1. Event socket updates reached Rust and changed the bar's DOM title from `active:1` to `active:2` and back. Hyprland 0.56's Lua dispatcher syntax was required; legacy `dispatch workspace 2` failed. |
| Popup overflow | A separate `menu` overlay appeared at x=12, y=44, size 320×240 below a 44-pixel bar. This worked without asking the bar's DOM to paint outside its native surface. |
| Transparent input | A 1280×360 transparent overlay installed one DOM-reported `GdkSurface` input rectangle. A real pointer click at x=45,y=20 passed through to the bar and opened its launcher. A click at x=640,y=180 reached the overlay's interactive card and changed its DOM title. |
| Web process failure | `webkit_web_view_terminate_web_process` produced `WEB_PROCESS_TERMINATED`, followed by destroy/create/reload of only the affected bar instance. The Rust host stayed alive. |
| Ordinary preview | The refactored runtime loaded its pages in GTK preview windows; launcher show/hide and local asset reload also worked. |

After the 2026-09-24 cleanup, `cargo fmt --check`, the locked build, and all four Rust tests passed again. A fresh layer-mode run showed bars on `eDP-1` and `DP-3` and opened the launcher; the external connector had a different name from the earlier `DP-2` run. A preview-mode run showed and hid the launcher and reloaded local bar pages. `npm run dev` served `dev-bar.html` from `127.0.0.1:5173`. The more detailed HMR, workspace, popup, input-region, and crash experiments in the table were performed before this cleanup and were not repeated afterward.

The popup experiment favors **separate native surfaces**: input and lifecycle are clear, and the menu can extend below the bar. Exact anchoring to an arbitrary DOM element is not solved; this demo places it relative to the output's usable top edge. The input-region experiment uses explicit rectangles reported by JS, because WebKit does not expose its DOM hit-test region as a Wayland input region. Dynamic DOM movement would need rectangle updates after layout changes and scrolling.

## Limits and next boundary

A physical monitor hotplug was not performed; `GdkMonitor` changes are wired and wildcard expansion was verified on two connected monitors. Connector strings are stable enough for this machine, while a missing connector falls back to `monitor-N` and may change after reordering. Hyprland command queries currently run synchronously on GTK's thread; a slow or failed socket can briefly stall it (2-second timeout). Process recovery has no restart backoff. Input rectangles are reported in CSS pixels. Subsequent shared pointer tests passed on scale-1 and scale-2 outputs, including stretched windows; browser zoom and arbitrary CSS transforms remain outside that verification. Direct FFI keeps GTK/WebKit ownership and callback lifetimes in unsafe Rust; this is the largest implementation risk remaining.

GTK/WebKit logged an Intel Vulkan driver warning, and Hyprland's portal Settings activation logged a warning in some runs. They did not prevent the tested pages or surfaces from working.

The smallest reusable boundary is now `Vec<SurfaceSpec>` into `SurfaceManager`, plus typed `BridgeCommand` and `WorkspaceState` over the WebKit transport. The next engineering step is to isolate the unsafe GTK/WebKit bindings behind a small safe wrapper, then test real monitor hotplug and improve popup positioning. System services such as audio, notifications, tray, and MPRIS remain outside this slice.

The [iced](../iced-shell/README.md) and [Quickshell](../quickshell-shell/README.md) companions now implement the common workload. The [comparison report](../comparison/REPORT.md) records fresh two-output topology, pointer, workspace-button, filtering/Escape, frontend reload, Vite HMR, and WebKit recovery checks, plus five repeated release measurements and dependency evidence. That report is the historical five-run snapshot. See the [CPU investigation](../comparison/CPU_ANALYSIS.md) for current 30-second measurements and the [architecture diagrams](../ARCHITECTURE.md) for the three stacks.
