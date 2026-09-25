# Release validation evidence

All eight graphical check groups and all 30 benchmark runs passed. See the [release report](../../RELEASE_VALIDATION.md) and [tables](../../RELEASE_TABLES.md).

This directory records a fresh build, graphical regression pass, and independent rerun of the current WebKit, optimized iced, and Quickshell implementations. Previous evidence under `../cpu/` and the original results directory is preserved.

- `build-checks.json`: release builds, unit/configuration tests, formatting checks.
- `source-hashes.json`, `binaries.sha256`, `quickshell-store-path.txt`: source and executable provenance. Build artifacts themselves are not published.
- `checks.json`, `*-runner.log`: graphical suite exit statuses and logs.
- `functional.json`: all three implementations' topology checks.
- `regressions/`: fresh frontend, pointer, keyboard, workspace, WebKit recovery and iced-specific artifacts from existing fixed-path scripts. Screenshot references inside legacy JSON are relative to their original locations; use the matching filenames here. Byte-identical captures may match historical images.
- `performance.json`, `performance-*.log`: ten rotating runs per implementation, 30-second idle and active windows. Errors remain in the JSON rather than being discarded.

Run `nix develop path:./comparison -c python3 comparison/release_check.py` to repeat the graphical suite. It temporarily uses legacy result paths, copies evidence here, and restores the historical files. It overwrites this release-validation directory; archive it first if you want another distinct campaign. Do not run other tests concurrently. The existing scripts restore edited assets and desktop state in their cleanup paths; inspect logs if interrupted.

The suite moves the pointer, types into overlays and switches workspaces. It requires the active Hyprland session and built binaries. It is not a headless unit-test runner.
