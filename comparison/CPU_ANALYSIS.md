# iced CPU investigation

A subsequent [release validation](RELEASE_VALIDATION.md) rebuilt all three versions and repeated ten rotating measurements each. This document preserves the original optimization investigation.

See [architecture and before/after flow diagrams](../ARCHITECTURE.md) and the [reproduction guide](../REPRODUCING.md). Historical baseline binaries/source are not included in a fresh clone; raw evidence is retained.

The excess CPU was primarily caused by the prototype's 150 ms tick combined with the adapter's redraw-all default. The static runtime blocks; no inherent ~2% idle floor was found. Replacing polling and targeting redraws reduced iced CPU by approximately **84% idle and 94% active**, with essentially unchanged median startup.

## Final measurements

Medians; CPU is percent of one core, PSS is MiB. The fresh before run has three 30-second idle/active samples; the final comparison has ten complete runs per implementation, with the same 30-second windows and rotating launch order. One incomplete Quickshell attempt and its replacement are explicitly retained below.

| Implementation | Startup ms | Idle PSS | Active PSS | Idle CPU | Active CPU |
| --- | ---: | ---: | ---: | ---: | ---: |
| iced before | 127.5 | 41.2 | 46.3 | 2.06% | 5.31% |
| iced after | 127.1 | 42.4 | 43.1 | **0.33%** | **0.33%** |
| Quickshell, unchanged | 172.7 | 93.4 | 121.0 | 0.96% | 1.50% |
| WebKit, unchanged | 407.2 | 441.9 | 512.1 | 3.39% | 5.60% |

The [complete tables](CPU_TABLES.md) include RSS, process counts, latency tails, dispatch/redraw/present rates, context switches, cycles and instructions. Raw evidence is in [results/cpu](results/cpu/README.md); the [original comparison](REPORT.md) and its five-second samples remain intact.

A static single surface and a static two-bar workload had **zero post-settlement UI dispatches, redraws and presents**. The original idle workload submitted 13.3 frames/s; the original active workload submitted 53.4 frames/s. The updated normal workload submits two frames/s for the two clocks. A separately focused launcher still animates its caret correctly.

All native Rust and live graphical regressions passed, including config topology, exclusive zones, launcher/menu/input behavior, workspace actions/events, descriptor reload, ordinary preview and a temporary headless output. Test processes/layers were stopped and the original two workspaces/focus restored.

## Scope and baseline

This investigation preserves the comparison's visible workload: two output bars, second-resolution clocks and workspace buttons; the active case additionally shows launcher, menu and input-region surfaces on both outputs. No animations, widgets or interactions were removed to obtain the normal benchmark result. Static/minute-clock configurations are diagnostic experiments only.

Historical results remain in `results/performance.json`. Before changing behavior, the existing release application was rebuilt with the pinned Nix environment and measured three times with 30-second idle and active windows. `results/cpu/before.json` contains startup, RSS, PSS, process counts and CPU samples. Original executables are retained locally in `iced-shell/target/cpu-before/`; hashes are recorded alongside results.

## Instrumentation

Counters are enabled by `SHELL_DIAGNOSTICS=1` and reported every ten seconds from a separate thread. Normal comparison runs disable counters. The reporter adds one thread wake every ten seconds but sends no UI event. Static tests therefore distinguish UI dispatch counts from whole-process scheduling.

| Counter | Boundary |
| --- | --- |
| wakeups | Successful return from the adapter's calloop dispatch, including immediately runnable dispatches; not hardware interrupts or total thread wakeups |
| messages | Application `update` calls, excluding internal actions consumed by the adapter |
| views | Application `view` calls |
| layouts | Adapter UI build/relayout entries; a cached layout may avoid some widget work |
| redraw_requests | Per-surface `request_refresh` calls, before request coalescing |
| refresh_callbacks | Adapter refresh handlers, including callbacks that could return early |
| draws | iced UI primitive generation (`ui.draw`) |
| renders | Calls to iced's compositor `present`, which acquires a texture and invokes the renderer |
| presents | Successful compositor presents; inspected iced_wgpu 0.14 source submits rendering and calls `SurfaceTexture::present` on this path. These are submitted frames, not compositor display/photon counts |
| hypr_queries / hypr_events | Command-socket requests / event-socket lines received |
| outputs | Output queries in the baseline, output notifications in the updated runtime |
| config_checks / config_reloads | Config reads triggered by the watcher / valid declarations applied |
| clock_fires | Delivered clock deadlines (baseline: clock string changes within the shared tick) |
| pointer / keyboard | iced input events observed by the event subscription |

Vendored adapter sources make these boundaries reviewable. No generic tracing subsystem or renderer replacement was introduced. `vendor/README.md` describes the small patches. Counter totals are aggregated across actual visible native surfaces; hidden overlays have no window to render.

## Root cause established before optimization

The original 150 ms tick always delivered an application message. In `iced_exwlshell::handle_normal_dispatch`, the default policy invalidates every window for every application message. The adapter also releases/rebuilds all borrowed views before/after application mutation. Equality checks *inside* `App::update` did not undo that invalidation.

The instrumented original version presented approximately 13.3 frames/s across two bars and completed approximately 49 UI-loop dispatches/s. It rebuilt roughly 13.3 views/layout entries per second despite only one visible clock change per second. It also read configuration/control files about 6.67 times/s and queried outputs about 0.95 times/s. These counters are recorded in `instrumented-before.log`.

The underlying Wayland loop computes the earliest requested refresh deadline and passes it to calloop. Without pending deadlines it blocks indefinitely; it does not use a fixed `Poll` loop. Wayland frame callbacks permit future rendering; they do not continuously invalidate unchanged content. The static test establishes this experimentally.

## Changes

- Replaced the shared tick and periodic descriptor/config/control reads with an inotify-backed `notify` watcher. Parent-directory watches survive atomic replacement; relevant config/page events use a 250 ms quiet-period debounce. Control commands are delivered immediately. Unrelated paths are filtered before waking the async consumer.
- Replaced the polled Hyprland channel with a directly awaited event socket. Workspace cache updates are filtered for equality before sending UI messages. Startup/reconnect and relevant events may issue command queries; there is no connected heartbeat. A disconnected socket retries after one second.
- Replaced monitor polling with the adapter's Wayland output event subscription and connector-name cache.
- Scheduled one clock deadline at the actual next second (or experimental minute) boundary. No per-output timers exist. The subscription stops if no bars are visible.
- Added a small `Scope::Windows` adapter extension so one global clock update redraws only bars. Launcher filtering targets its own window.
- Replaced the periodic retry of initial input-region sizing with the native-window-created callback. Real pointer testing found this lifecycle dependency when the tick was removed.

The adapter still rebuilds all borrowed views on a global state update, even when redraw targets are narrower. This remaining behavior is measured rather than described as eliminated.

## Reproduction

Run from the repository root in the active Hyprland session. Graphical tests temporarily create surfaces and switch workspaces; the interaction helpers restore the original cursor/workspaces.

```sh
nix develop path:./iced-shell -c bash -c 'cd iced-shell && ./build.sh'
nix develop path:./iced-shell -c bash -c 'cd iced-shell && cargo build --profile profiling --locked'
nix develop path:./comparison -c python3 comparison/cpu_probe.py --label static --env SHELL_STATIC=1 --seconds 40
nix develop path:./comparison -c bash comparison/cpu_matrix.sh
nix develop path:./comparison -c bash comparison/cpu_perf.sh
nix develop path:./comparison -c python3 comparison/cpu_trace.py --label static-trace --env SHELL_STATIC=1
nix develop path:./comparison -c python3 comparison/bench.py --only iced --functional --output comparison/results/cpu/functional.json
nix develop path:./comparison -c python3 comparison/cpu_regressions.py
nix develop path:./comparison -c python3 comparison/bench.py --runs 10 --sample-seconds 30 --output comparison/results/cpu/after.json
```

`cpu_probe.py --perf /path/to/perf` additionally captures separate 30-second stat and sampled-profile windows. Pass `--command /absolute/path/to/iced-shell/target/profiling/iced-shell` for symbols. Profiling and syscall tracing are separate from benchmark measurements. `cpu_trace.py` launches the application as strace's child to respect the host's Yama ptrace restrictions without changing system policy.

The existing startup endpoint is compositor-observed mapped bar geometry and nonzero alpha, with 10 ms polling. It is not first-content-frame or photon latency. CPU is summed process-tree CPU time as a percentage of one core. RSS double counts shared mappings; PSS is the preferred memory comparison. The original 2-second idle and 1-second active settling periods are preserved in the shared benchmark; diagnostic experiments settle for 12 seconds.

## Workload interpretation

The static single-surface run produced no post-settlement UI dispatches, view builds, redraws or presents. Its 40-second `/proc` CPU sample rounded to zero; a separate 30-second perf window recorded 0.41 ms task-clock, 129,702 user cycles and 66,222 user instructions. Most remaining observable activity is the diagnostic reporter (one wake/ten seconds). The static two-bar, watcher-only, and workspace-only runs likewise had zero ongoing UI work. Zero CPU here means below the process accounting resolution (one tick/40 seconds is about 0.025% of one core), not literally zero executed instructions.

A second-resolution clock produces one application message and two submitted frames per second across the two bars. The complete active configuration rebuilds eight borrowed views on each global clock update, but the selective redraw policy still submits only the two changed bars. In the tested all-overlays arrangement the launcher field is not keyboard focused; disabling the clock makes that arrangement static. A separate launcher-only configuration has a focused caret and produces two frames/s with **zero application messages and view rebuilds**. Inspection of iced's text-input implementation confirms a 500 ms redraw deadline for caret blinking. No application-defined launcher animation exists in this workload; compositor opening/closing animations are retained and are outside the client CPU process tree.

A focused-launcher typing check was run against both original and updated binaries. The original search field did not receive typing until clicked. The native-created callback fixes that lifecycle issue: the updated launcher receives typing immediately. Screenshots are retained in `autofocus-*-typed.png`. The optimization did not obtain savings by disabling search focus or caret behavior.

The first input-region regression failure is retained as `regressions-before-lifecycle-fix.json`. After the callback fix, all shared pointer, keyboard, filtering/Escape, workspace-action, topology and descriptor reload tests pass. `output-events.json` additionally records adding/removing `HEADLESS-1`, with a corresponding bar created/destroyed from Wayland notifications. The headless output was removed. This tests real compositor output events, not a physical cable hotplug.

## Renderer sanity check

Diagnostics identify the default adapter as Intel Iris Plus Graphics (ICL GT2), **Vulkan**, Intel Mesa 26.2.3. The selected format is `Bgra8UnormSrgb`, premultiplied alpha, with `AutoVsync`; iced_wgpu configures maximum frame latency 1. Both attached outputs are approximately 60 Hz, at scales 2 and 1. The `AutoVsync` policy and submitted frames are observed; this does not measure display scanout timing.

An alternate `WGPU_BACKEND=gl` run initially failed because the standalone binary's library path exposed Vulkan but not the EGL loader. Providing the already installed libglvnd library directory enabled the hardware Intel OpenGL adapter (`Rgba8UnormSrgb`, Mesa 26.2.3). Its single 40-second sample was 0.273% CPU versus about 0.299% for the default Vulkan diagnostic. This small, single-run difference is not evidence for a meaningful backend ranking. Vulkan remains the default; no present-mode, latency, animation, or quality setting was reduced.

Exact alternate-backend experiment on this pinned host:

```sh
nix develop path:./comparison -c python3 comparison/cpu_probe.py --label backend-gl \
  --env WGPU_BACKEND=gl \
  --env LD_LIBRARY_PATH=/nix/store/dpi7sgi4dmnddwjwdi57d8x06sx49vkn-libglvnd-1.7.0/lib:/run/opengl-driver/lib \
  --seconds 40
```

The preserved original active binary was also measured with counters: approximately **53.4 presents/s**, 53.4 view/layout builds/s and 63 dispatch returns/s for eight visible surfaces, at 5.27% CPU in that diagnostic sample. The updated all-overlays case produces two presents/s and eight view/layout builds/s at approximately 0.35% CPU. The dramatic frame-count reduction establishes the cause more directly than comparing framework labels or inferring costs from arbitrary stack names.

## Profile interpretation

The optimized idle 30-second perf window recorded 102.96 ms task-clock, 63.9 million user cycles and 24.3 million user instructions, compared with 718.75 ms, 431.8 million cycles and 151.2 million instructions in the original instrumented idle window. These are separate windows, not perfectly paired hardware experiments. The symbolized profiling binary uses the same release optimization settings with debug information retained; final resource comparisons use the ordinary stripped release binary.

The optimized idle cycle profile contains 101 samples, with no lost samples. Summing **self** overhead by mapped object gives approximately 43% in the application binary (including statically linked iced/wgpu/executor code), 28% in Intel's Vulkan driver, 15% in libc, 7% in libwayland-client and 2% in libdrm; approximately 2.5% is unresolved. These are coarse sample shares, not stage timings. The small sample count does not support a fine-grained percentage decomposition.

Inspected call chains reach iced text drawing/font fallback, `wgpu_core::command::render::encode_render_pass`, command-buffer finish/submission, Vulkan image acquisition/presentation, Mesa command emission, allocation/memory copies and Wayland queue dispatch. The refresh-handler path has approximately 20% **inclusive** sampled overhead and present approximately 18%; those overlap and must not be added. `polling::epoll::Poller::wait_deadline` appears in samples around wake/return processing. Its name alone is not evidence of busy polling: the static counters and syscall trace establish whether it actually blocks.

The original binary is stripped, so its sampled Rust addresses cannot be reliably split into iced layout versus wgpu preparation. Driver/Wayland frames are still visible. The before/after message and frame counters provide the causal evidence: each unnecessary application tick rebuilt views and submitted complete frames. No fabricated stage percentages are assigned to unresolved addresses.

Quickshell's `QSG_INFO=1` startup diagnostics identify Qt Quick's threaded render loop, Intel hardware OpenGL 4.6 through Mesa 26.2.3, swap interval 1, RGBA8 and no multisample buffers. Thus the implementations use different default graphics APIs on the same GPU. The iced OpenGL sanity check did not reveal an order-of-magnitude change. These measurements compare the existing defaults, not matched-backend renderer efficiency.

## Syscalls and blocking

Fresh runs under strace avoid changing Yama ptrace policy. Measurements use only syscall entries begun in a settled 30-second interval; startup, tracer shutdown and operations begun before the interval are excluded. Tracing is intrusive and its timings are not used for the CPU ranking.

| Observable entries in 30 s | Original iced idle | Updated iced idle | Quickshell idle |
| --- | ---: | ---: | ---: |
| GPU/driver `ioctl` | 16,800 | 2,520 | 1,247 |
| `openat` | 458 | 0 | 200 |
| Hyprland socket `connect` | 29 | 0 | 0 |
| `futex` | 5,272 | 615 | 4,015 |
| `epoll_pwait` + `epoll_wait` + `poll` + `ppoll` | 2,264 | 478 | 1,738 |

These are **all-thread syscall-entry counts**, not equivalent UI wakeup counters or GPU command counts. OpenGL/Vulkan have different syscall patterns, so `ioctl` counts cannot rank renderer efficiency across backends. Within the same iced/Vulkan implementation the 6.67× reduction in `ioctl` calls agrees exactly with reducing bar presents from 13.33/s to 2/s.

The static trace has **zero syscall entries inside the settled interval**. Its main thread is blocked in `epoll_pwait(..., timeout=-1)` for 45.55 seconds, until shutdown interrupts it. The trace run disables diagnostic reporting, unlike the periodic-counter tests. No busy loop, repeating flush, zero-duration timer or clipboard heartbeat appears while static.

The original trace opens files and creates approximately one Hyprland command connection per second; the updated steady-state trace does neither. Quickshell's 200 file opens match its unchanged 150 ms control-file polling timer. It also performs 4,019 `newfstatat` calls on `/etc/localtime` in this interval. Its sampled stacks include `SystemClock::onTimeout`, Qt date/time conversion through `QLocalTime`/`tzset`, QObject signal activation, GL rendering and GLib event dispatch. The complete cost of those mechanisms was not isolated by modifying Quickshell, so this is evidence of remaining work, not a claim that one function explains all of its CPU.

### Watcher coverage regression

Review found that watching only the sample `pages/` directory would narrow the existing descriptor contract. The final watcher derives its subscriptions from configured descriptor paths, including resolved symlink targets, and watches their parent directories. The path list is cached and refreshed only by config/content events, never by clock updates. `watch-paths.json` tests a descriptor outside `pages/`, atomic replacement of a symlink target, malformed descriptor retention and recovery, with native surface addresses retained. Parent registration failures are logged rather than taking down existing surfaces. Atomic file replacement is supported; replacing entire watched directory trees may require restarting the runtime.

An initial final-benchmark pass was stopped during review and retained as `after-prewatch-fix.json`; it is excluded from the final table. The event/render profiles were gathered before this watch-coverage correction. The final settled trace (`final-idle-trace.json`) again contains zero file opens, metadata checks or Hyprland connects, and exactly 2,520 driver `ioctl` calls in 30 seconds. The correction does not change the default workload's timers, redraw policy or rendering; a fresh settled syscall trace checks the delivered implementation for accidental metadata polling.

## Interpretation and next step

**Primarily outcome A: prototype scheduling/invalidation.** The 150 ms application tick repeatedly invoked the adapter's redraw-all default. Eliminating the tick reduces bar frames by 6.67×; targeting clock redraws additionally avoids repainting six unchanged overlays. Event-driven filesystem, compositor and output subscriptions are quiescent without input. There is no demonstrated ~2% iced/wgpu idle floor.

The adapter contributes a specific, documented behavior: every application message rebuilds borrowed views, and its default redraw scope is global. That default amplified this prototype's polling. Its blocking Wayland loop is not the source of continuous work. Eight view/layout entries per clock message remain in the active case, but only two frames are submitted. The measured residual work is normal text/UI preparation, GPU submission and protocol handling at actual invalidations. The current evidence does not establish a fundamental retained-scene-graph advantage or justify replacing iced.

Quickshell remains unmodified, including its control-file polling and per-bar `SystemClock` objects. As in the original comparison, iced formats UTC with integer arithmetic while Quickshell formats local `QDateTime`; the timezone/calendar work is another application-level difference, not an intrinsic framework requirement. Its new profiles show timer/date-time/signal/event-loop work and hardware GL rendering. It is not a fully event-driven control implementation yet. A fair next architecture comparison should apply the same event-source discipline to each prototype before interpreting residual differences as framework costs. This investigation does not assign an exact fraction of Quickshell CPU to its polling, clocks or scene graph without those separate interventions.

The smallest useful native API improvement is a surface-targeted invalidation boundary: shared system state is produced once, equality-filtered, then delivered only to affected surfaces. The multi-window redraw-scope extension is small enough to propose upstream. If many more surfaces make view/layout rebuilding significant, first test immutable per-surface view models and iced's existing caching facilities. A custom retained renderer would need persistent nodes, dirty tracking, incremental layout, shared text/GPU caches and redraw-on-invalidation, but this workload provides no reason to build one now.

## Limits

- This is one live desktop/GPU, with warm OS and shader caches, normal compositor animations and no fixed CPU frequency. Host busy percentages are retained. It is not a cross-hardware framework ranking.
- CPU includes application child processes, but excludes compositor CPU and GPU execution/memory. Vulkan and OpenGL defaults differ.
- Five-second historical samples and thirty-second fresh samples have different sensitivity to startup/activation transients. Unchanged reference implementations can consequently produce different results; both datasets are retained.
- perf is limited to user-space events on this host. Kernel/DRM execution is included in process CPU accounting but cannot be fully attributed by these samples. Zero `context-switches:u` must not be interpreted as zero scheduling; `/proc` provides the real thread-switch deltas.
- UI layout counters mark build/relayout entry points, not cache misses or individual widget layouts. Presents are client submissions, not displayed photons. Individual GPU render-pass command counts are not instrumented.
- The counter reporter itself wakes every ten seconds. It is disabled for final performance and syscall tracing runs.
- Physical monitor unplug/replug was not attempted; a real compositor headless-output add/remove was verified.

### Reference-run failure and harness correction

The final rotating pass completed all ten iced and all ten WebKit runs, but Quickshell attempt 9 timed out while hiding its launcher via the control file. The host was still alive and the launcher layer remained mapped. The failed attempt and its log are preserved in `after.json`; no partial metrics were invented or pooled. A supplemental Quickshell run is recorded separately in `after-quickshell-retry.json`.

Review found that the harness's purportedly distinct whitespace nonce (`monotonic_ns() % 997`) could repeat on adjacent identical commands. It now uses an incrementing modulo counter, guaranteeing adjacent commands differ. The previous command contents were not retained, so a nonce collision cannot be proved to have caused this timeout; a consumer race also cannot be excluded retrospectively. Quickshell passed 100 alternating launcher show/hide operations with the corrected harness (`control-quickshell.json`). This correction changes only transport nonce uniqueness, not surfaces, timing endpoints, resource accounting or sample durations. The original rotating run used the original nonce rule; the supplemental run uses the correction.

The historical 75 ms iced control-launcher median used five-second resource windows. The fresh unmodified 30-second baseline's median was approximately 161 ms, reflecting a different phase relative to the 150 ms polling timer. The event-driven version is approximately 32 ms. These are control-file-to-layer timings, not pointer input latency; the pointer regression remains around the former 26 ms median and within the test's 10 ms observation granularity.


### WebKit sampling-window diagnostic

The unchanged WebKit reference's fresh 30-second active average is much lower than the historical five-second average. A separate diagnostic (`web-active-transient.json`) measured six consecutive five-second windows after the usual one-second active settle: **28.08%, 2.83%, 2.64%, 2.64%, 2.26%, 2.84%**. This establishes a substantial activation transient and demonstrates why the original short samples and fresh longer averages are not interchangeable. It does not prove every difference between the historical runs came from that transient. The diagnostic is not pooled into the ten-run comparison, and the shared benchmark's settling policy was preserved.
