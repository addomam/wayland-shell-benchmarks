# Pinned adapter sources

`iced_exwlshell` and `exwlshellev` are vendored from the crates.io 0.20.1 archives (MIT licenses retained). The preexisting Cargo source cache was checked byte-for-byte against the archives before copying.

Local changes:

- Optional `shell-diagnostics` counters at dispatch return, redraw request, UI layout/draw, compositor present attempt and successful present boundaries.
- `redraw::Scope::Windows(Vec<Id>)` permits one global clock message to invalidate just the bar surfaces across outputs.

The adapter still rebuilds all borrowed views on application messages. No lifetime/ownership or renderer scheduling behavior was replaced. Counters are disabled unless `SHELL_DIAGNOSTICS` is set. Archive-to-vendor patches are recorded with the CPU experiment results.
