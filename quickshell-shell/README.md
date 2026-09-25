# Quickshell surface runtime

A QML implementation of the shared [comparison contract](../comparison/PARITY.md), running as a Wayland shell client. Quickshell 0.3.0 and Qt 6.11.0 are pinned through the same Nixpkgs revision as the web prototype. Hyprland remains the compositor.

## Run

```sh
cd quickshell-shell
nix build path:.
./result/bin/quickshell -p shell.qml
```

`nix build` fetches the pinned upstream executable; this project has no native compilation step. For measurements, run the executable directly and exclude Nix startup. The shared harness uses this project’s `result/bin/quickshell` symlink, created by the build command above.

Environment variables:

- `SHELL_CONFIG`: absolute JSON configuration path; defaults to this project's `surfaces.json`.
- `SHELL_CONTROL`: command file; defaults to `/tmp/quickshell-shell.control`.
- `SHELL_PREVIEW=1`: ordinary floating windows instead of layer surfaces. Intended for a Wayland desktop; offscreen Qt cannot load the statically referenced PanelWindow backend.

```sh
printf 'toggle launcher\n' > /tmp/quickshell-shell.control
printf 'toggle menu@DP-3\n' > /tmp/quickshell-shell.control
printf 'workspace 2\n' > /tmp/quickshell-shell.control
printf 'reload\n' > /tmp/quickshell-shell.control
printf 'quit\n' > /tmp/quickshell-shell.control
```

Commands are polled every 150 ms and consumed by clearing the file. An unqualified toggle selects the first matching instance in lexicographic order. Buttons inside a surface target their originating output. The four launcher entries are interaction samples; selecting them only closes the launcher.

## Runtime and configuration

`shell.qml` owns the desired specs and a map of native objects keyed by `id@connector`. `config.js` validates the complete JSON document before reconciliation. Unknown fields, duplicate IDs, invalid property values, and invalid/missing QML components reject the update while retaining the working layout. Configuration file events debounce for 300 ms before parsing.

`Surface.qml` binds each spec to a PanelWindow and its WlrLayershell attached object. Native role/namespace are established when the object is created. Geometry, anchors, layer, keyboard focus, zone, and margins use Quickshell's supported property setters. Existing IDs retain their window object on configuration updates. Removed IDs destroy their window. `Quickshell.screens` supplies connector names and triggers reconciliation when the output list changes. Wildcard declarations expand over actual outputs.

`page` is a local `pages/Name.qml` component. The runtime gives it `manager` and `surface` properties. Add reusable types to `pages/qmldir` so dynamically loaded components resolve siblings through Quickshell's virtual reload filesystem. `pages/Generic.qml` demonstrates an arbitrary configured surface. QML is trusted native shell code: it can import Quickshell capabilities. There is no browser sandbox or remote-page loading.

The native Quickshell Hyprland module receives workspace state over compositor IPC. Bar buttons call the narrowly exposed `manager.activate(id)`, constrained to workspace IDs 1–100, using Hyprland's Lua dispatcher syntax. No subprocess polling is used. This dispatcher syntax targets the current Hyprland 0.56 session.

## Frontend development

Edit the QML files while running. Explicit FileView watchers cover dynamically loaded pages and trigger Quickshell configuration reload in the same host process. Dynamic surface objects are reconstructed and transient state resets. A root QML parse failure uses Quickshell's generation fallback, but a malformed dynamically loaded page can leave an empty generation until corrected. This differs from WebKit/Vite HMR. JSON topology reload is handled separately by FileView and reconciliation and retains working surfaces on validation failure.

A separate menu PanelWindow extends below the bar. The input test uses a 620×360 transparent window whose `Region` mask references only the centered 250×160 card. The mask controls native input delivery, allowing pointer events through the surrounding area. This avoids a DOM-to-native rectangle bridge.

## Validation and limitations

```sh
nix develop path:. -c node test-config.js
```

The test checks configuration rejection and per-output expansion. Initial real Hyprland smoke testing mapped both bars on eDP-1 and DP-3; the centralized comparison harness records full topology, input, workspace, resource, and timing results under `../comparison/`. Consult those results for actual observations rather than treating tests as claims of first-frame presentation.

The completed [comparison report](../comparison/REPORT.md) records passing topology reconciliation, real launcher/workspace clicks, filtering/Escape, and transparent-input tests on both output scales. QML frontend edits were verified to retain the host while replacing dynamic native surfaces. Those five-run measurements are historical; the [CPU investigation](../comparison/CPU_ANALYSIS.md) includes ten completed measurements per implementation and records a Quickshell control timeout.

QML/Qt code shares the shell process. There is no isolated web worker process to terminate and recover; a native renderer crash can terminate the whole shell. Quickshell has upstream crash handling, but this prototype does not implement an extra supervisor. Physical monitor hotplug must be tested separately. No audio, tray, notifications, network, Bluetooth, lock-screen, or application execution services were added.

The host emitted a desktop portal activation warning and system Fontconfig warnings while successfully mapping windows. Neither prevented the initial smoke test. The first attempt at dynamic page loading failed to find sibling QML types; the explicit `pages/qmldir` resolved it.

## Upstream references

- [PanelWindow](https://quickshell.org/docs/v0.3.0/types/Quickshell/PanelWindow/)
- [WlrLayershell](https://quickshell.org/docs/v0.3.0/types/Quickshell.Wayland/WlrLayershell/)
- [FileView](https://quickshell.org/docs/v0.3.0/types/Quickshell.Io/FileView/)
- [Hyprland IPC integration](https://quickshell.org/docs/v0.3.0/types/Quickshell.Hyprland/Hyprland/)

The clean next extraction is the surface manager plus page context (`manager`, `surface`), retaining QML and its native reload model as this implementation's frontend.
