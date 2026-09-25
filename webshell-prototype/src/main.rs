mod config;
mod hyprland;

use config::{Edge, OutputSelector, Page, SurfaceSpec};
use hyprland::{Hyprland, WorkspaceState};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::{Duration, Instant, UNIX_EPOCH};

type P = *mut c_void;
fn c(s: &str) -> CString {
    CString::new(s).expect("NUL byte")
}

#[link(name = "gtk-4")]
unsafe extern "C" {
    fn gtk_init();
    fn gtk_window_new() -> P;
    fn gtk_window_set_title(w: P, s: *const c_char);
    fn gtk_window_set_default_size(w: P, width: c_int, height: c_int);
    fn gtk_window_set_child(w: P, child: P);
    fn gtk_window_present(w: P);
    fn gtk_window_destroy(w: P);
    fn gtk_window_set_decorated(w: P, value: c_int);
    fn gtk_widget_set_visible(w: P, value: c_int);
    fn gtk_widget_set_size_request(w: P, width: c_int, height: c_int);
    fn gtk_native_get_surface(w: P) -> P;
}
#[link(name = "webkitgtk-6.0")]
unsafe extern "C" {
    fn webkit_web_view_new() -> P;
    fn webkit_web_view_get_user_content_manager(view: P) -> P;
    fn webkit_user_content_manager_register_script_message_handler(
        manager: P,
        name: *const c_char,
        world: *const c_char,
    ) -> c_int;
    fn webkit_web_view_load_uri(view: P, uri: *const c_char);
    fn webkit_web_view_reload(view: P);
    fn webkit_web_view_get_title(view: P) -> *const c_char;
    fn webkit_web_view_evaluate_javascript(
        view: P,
        script: *const c_char,
        length: isize,
        world: P,
        source: P,
        cancellable: P,
        callback: P,
        data: P,
    );
    fn webkit_web_view_terminate_web_process(view: P);
    fn webkit_web_view_set_background_color(view: P, color: *const GdkRgba);
    fn webkit_navigation_policy_decision_get_navigation_action(decision: P) -> P;
    fn webkit_navigation_action_get_request(action: P) -> P;
    fn webkit_uri_request_get_uri(request: P) -> *const c_char;
    fn webkit_policy_decision_ignore(decision: P);
}
#[link(name = "javascriptcoregtk-6.0")]
unsafe extern "C" {
    fn jsc_value_to_string(value: P) -> *mut c_char;
}
#[link(name = "gtk4-layer-shell")]
unsafe extern "C" {
    fn gtk_layer_is_supported() -> c_int;
    fn gtk_layer_init_for_window(w: P);
    fn gtk_layer_set_namespace(w: P, namespace: *const c_char);
    fn gtk_layer_set_layer(w: P, layer: c_int);
    fn gtk_layer_set_anchor(w: P, edge: c_int, anchor: c_int);
    fn gtk_layer_set_exclusive_zone(w: P, zone: c_int);
    fn gtk_layer_set_keyboard_mode(w: P, mode: c_int);
    fn gtk_layer_set_margin(w: P, edge: c_int, margin: c_int);
    fn gtk_layer_set_monitor(w: P, monitor: P);
}
unsafe extern "C" {
    fn gdk_display_get_default() -> P;
    fn gdk_display_get_monitors(display: P) -> P;
    fn gdk_monitor_get_connector(monitor: P) -> *const c_char;
    fn gdk_surface_set_input_region(surface: P, region: P);
}
#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn g_signal_connect_data(
        instance: P,
        signal: *const c_char,
        handler: *const c_void,
        data: P,
        destroy: P,
        flags: c_uint,
    ) -> u64;
    fn g_object_unref(object: P);
}
#[link(name = "gio-2.0")]
unsafe extern "C" {
    fn g_list_model_get_n_items(model: P) -> c_uint;
    fn g_list_model_get_item(model: P, index: c_uint) -> P;
}
#[link(name = "glib-2.0")]
unsafe extern "C" {
    fn g_main_loop_new(context: P, running: c_int) -> P;
    fn g_main_loop_run(loop_: P);
    fn g_main_loop_quit(loop_: P);
    fn g_main_loop_unref(loop_: P);
    fn g_timeout_add(
        interval: c_uint,
        callback: unsafe extern "C" fn(P) -> c_int,
        data: P,
    ) -> c_uint;
    fn g_free(ptr: P);
}
#[link(name = "cairo")]
unsafe extern "C" {
    fn cairo_region_create() -> P;
    fn cairo_region_union_rectangle(region: P, rect: *const Rect) -> c_int;
    fn cairo_region_destroy(region: P);
}
#[repr(C)]
struct GdkRgba {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}
#[repr(C)]
#[derive(Clone, Debug, Deserialize)]
struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Clone)]
struct Output {
    id: String,
    monitor: P,
}
struct Instance {
    key: String,
    output: String,
    monitor: P,
    spec: SurfaceSpec,
    widget: P,
    view: P,
    content_manager: P,
    uri: String,
    visible: bool,
    regions: Option<Vec<Rect>>,
}
struct SurfaceManager {
    instances: BTreeMap<String, Instance>,
    layer_mode: bool,
}
impl SurfaceManager {
    fn new(layer_mode: bool) -> Self {
        Self {
            instances: BTreeMap::new(),
            layer_mode,
        }
    }
    fn count(&self) -> usize {
        self.instances.len()
    }
    fn expand(
        specs: &[SurfaceSpec],
        outputs: &[Output],
    ) -> BTreeMap<String, (SurfaceSpec, P, String)> {
        let mut desired = BTreeMap::new();
        for spec in specs {
            for output in outputs {
                let selected = match &spec.output {
                    OutputSelector::All => true,
                    OutputSelector::Named(name) => name == &output.id,
                };
                if selected {
                    let key = format!("{}@{}", spec.id, output.id);
                    desired.insert(key, (spec.clone(), output.monitor, output.id.clone()));
                }
            }
        }
        desired
    }
    unsafe fn create(
        &mut self,
        key: String,
        spec: SurfaceSpec,
        monitor: P,
        output: String,
        root: &Path,
        app_ptr: P,
    ) -> Result<(), String> {
        let uri = spec.page.uri(root)?;
        let window = gtk_window_new();
        gtk_window_set_title(window, c(&format!("WebShell: {key}")).as_ptr());
        let width = if self.layer_mode && spec.width == 0 {
            0
        } else if spec.width == 0 {
            700
        } else {
            spec.width
        };
        gtk_window_set_default_size(window, width, spec.height);
        // All layer-shell setup must precede the first realize/present call.
        if self.layer_mode {
            gtk_window_set_decorated(window, 0);
            gtk_layer_init_for_window(window);
            gtk_layer_set_namespace(window, c(&format!("webshell-{key}")).as_ptr());
            gtk_layer_set_monitor(window, monitor);
            gtk_layer_set_layer(window, spec.layer.native());
            for edge in [&Edge::Left, &Edge::Right, &Edge::Top, &Edge::Bottom] {
                gtk_layer_set_anchor(window, edge.native(), spec.anchors.contains(edge) as c_int);
                gtk_layer_set_margin(window, edge.native(), spec.margins.edge(edge));
            }
            gtk_layer_set_exclusive_zone(window, spec.exclusive_zone);
            gtk_layer_set_keyboard_mode(window, spec.keyboard_interactivity.native());
        }
        let view = webkit_web_view_new();
        gtk_widget_set_size_request(view, 1, spec.height);
        webkit_web_view_set_background_color(
            view,
            &GdkRgba {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.0,
            },
        );
        let content_manager = webkit_web_view_get_user_content_manager(view);
        if spec.page.trusted() {
            g_signal_connect_data(
                content_manager,
                c("script-message-received::shell").as_ptr(),
                on_message as *const c_void,
                app_ptr,
                ptr::null_mut(),
                0,
            );
            if webkit_user_content_manager_register_script_message_handler(
                content_manager,
                c("shell").as_ptr(),
                ptr::null(),
            ) == 0
            {
                eprintln!("failed to register script bridge for {key}");
            }
        }
        g_signal_connect_data(
            view,
            c("decide-policy").as_ptr(),
            on_policy as *const c_void,
            app_ptr,
            ptr::null_mut(),
            0,
        );
        g_signal_connect_data(
            view,
            c("load-changed").as_ptr(),
            on_load as *const c_void,
            app_ptr,
            ptr::null_mut(),
            0,
        );
        g_signal_connect_data(
            view,
            c("notify::title").as_ptr(),
            on_title as *const c_void,
            app_ptr,
            ptr::null_mut(),
            0,
        );
        g_signal_connect_data(
            view,
            c("web-process-terminated").as_ptr(),
            on_terminated as *const c_void,
            app_ptr,
            ptr::null_mut(),
            0,
        );
        gtk_window_set_child(window, view);
        let visible = spec.visible;
        self.instances.insert(
            key.clone(),
            Instance {
                key: key.clone(),
                output,
                monitor,
                spec,
                widget: window,
                view,
                content_manager,
                uri: uri.clone(),
                visible,
                regions: None,
            },
        );
        webkit_web_view_load_uri(view, c(&uri).as_ptr());
        if visible {
            gtk_window_present(window);
        } else {
            gtk_widget_set_visible(window, 0);
        }
        println!(
            "CREATE {key} {}",
            if self.layer_mode { "layer" } else { "preview" }
        );
        Ok(())
    }
    unsafe fn destroy(&mut self, key: &str) {
        if let Some(instance) = self.instances.remove(key) {
            gtk_window_destroy(instance.widget);
            println!("DESTROY {key}");
        }
    }
    unsafe fn update(&mut self, key: &str, spec: SurfaceSpec, root: &Path) {
        let Some(instance) = self.instances.get_mut(key) else {
            return;
        };
        if instance.spec.page != spec.page {
            if let Ok(uri) = spec.page.uri(root) {
                instance.uri = uri.clone();
                webkit_web_view_load_uri(instance.view, c(&uri).as_ptr());
                println!("PAGE {key} {uri}");
            }
        }
        if self.layer_mode {
            if instance.spec.exclusive_zone != spec.exclusive_zone {
                gtk_layer_set_exclusive_zone(instance.widget, spec.exclusive_zone);
                println!("ZONE {key} {}", spec.exclusive_zone);
            }
            if instance.spec.margins != spec.margins {
                for edge in [&Edge::Left, &Edge::Right, &Edge::Top, &Edge::Bottom] {
                    gtk_layer_set_margin(instance.widget, edge.native(), spec.margins.edge(edge));
                }
                println!("MARGINS {key}");
            }
        }
        if instance.visible != spec.visible {
            if spec.visible {
                gtk_window_present(instance.widget);
                Self::apply_regions(instance);
            } else {
                gtk_widget_set_visible(instance.widget, 0);
            }
            instance.visible = spec.visible;
            println!("VISIBLE {key} {}", spec.visible);
        }
        instance.spec = spec;
    }
    unsafe fn reconcile(
        &mut self,
        specs: &[SurfaceSpec],
        outputs: &[Output],
        root: &Path,
        app_ptr: P,
    ) {
        let desired = Self::expand(specs, outputs);
        let old: Vec<String> = self.instances.keys().cloned().collect();
        for key in old {
            let replace = match desired.get(&key) {
                None => true,
                Some((spec, monitor, _)) => {
                    self.instances[&key].spec.requires_recreate(spec)
                        || self.instances[&key].monitor != *monitor
                }
            };
            if replace {
                self.destroy(&key);
            }
        }
        for (key, (spec, monitor, output)) in desired {
            if self.instances.contains_key(&key) {
                self.update(&key, spec, root);
            } else if let Err(e) = self.create(key, spec, monitor, output, root, app_ptr) {
                eprintln!("create failed: {e}");
            }
        }
    }
    unsafe fn toggle(&mut self, id: &str, origin_output: Option<&str>) {
        let preferred = origin_output.map(|output| format!("{id}@{output}"));
        let key = preferred
            .filter(|k| self.instances.contains_key(k))
            .or_else(|| {
                self.instances
                    .keys()
                    .find(|k| k.starts_with(&format!("{id}@")))
                    .cloned()
            });
        let Some(key) = key else {
            eprintln!("unknown surface: {id}");
            return;
        };
        let instance = self.instances.get_mut(&key).unwrap();
        if instance.visible {
            gtk_widget_set_visible(instance.widget, 0);
        } else {
            gtk_window_present(instance.widget);
            Self::apply_regions(instance);
        }
        instance.visible = !instance.visible;
        println!(
            "TOGGLE {key} {}",
            if instance.visible { "shown" } else { "hidden" }
        );
    }
    unsafe fn apply_regions(instance: &Instance) {
        let Some(rects) = &instance.regions else {
            return;
        };
        let surface = gtk_native_get_surface(instance.widget);
        if surface.is_null() {
            return;
        }
        let region = cairo_region_create();
        for rect in rects {
            cairo_region_union_rectangle(region, rect);
        }
        gdk_surface_set_input_region(surface, region);
        cairo_region_destroy(region);
        println!("INPUT {} {} rects", instance.key, rects.len());
    }
    unsafe fn set_regions(&mut self, key: &str, rects: Vec<Rect>) {
        if rects.len() > 32
            || rects.iter().any(|r| {
                r.width <= 0
                    || r.height <= 0
                    || r.x < 0
                    || r.y < 0
                    || r.width > 4096
                    || r.height > 4096
            })
        {
            eprintln!("invalid input rectangles for {key}");
            return;
        }
        if let Some(instance) = self.instances.get_mut(key) {
            instance.regions = Some(rects);
            Self::apply_regions(instance);
        }
    }
    unsafe fn reload_local(&self) {
        for instance in self.instances.values() {
            if matches!(instance.spec.page, Page::Local(_)) {
                webkit_web_view_reload(instance.view);
                println!("ASSET_RELOAD {}", instance.key);
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BridgeCommand {
    SurfaceToggle { id: String },
    WorkspaceActivate { id: i64 },
    InputRegions { rects: Vec<Rect> },
}
struct App {
    root: PathBuf,
    config_path: PathBuf,
    control_path: Option<PathBuf>,
    config_text: String,
    pending_config: Option<(String, Instant)>,
    last_bad: Option<String>,
    control_text: String,
    asset_stamp: u128,
    specs: Vec<SurfaceSpec>,
    outputs: Vec<Output>,
    monitors: P,
    outputs_dirty: bool,
    manager: SurfaceManager,
    hypr: Hyprland,
    workspace_state: Option<WorkspaceState>,
    failed: Vec<String>,
    main_loop: P,
}
impl App {
    unsafe fn snapshot_outputs(&mut self) {
        let mut fresh = Vec::new();
        for i in 0..g_list_model_get_n_items(self.monitors) {
            let monitor = g_list_model_get_item(self.monitors, i);
            if monitor.is_null() {
                continue;
            }
            let connector = gdk_monitor_get_connector(monitor);
            let id = if connector.is_null() {
                format!("monitor-{i}")
            } else {
                CStr::from_ptr(connector).to_string_lossy().into_owned()
            };
            fresh.push(Output { id, monitor });
        }
        let app_ptr = self as *mut _ as P;
        self.manager
            .reconcile(&self.specs, &fresh, &self.root, app_ptr);
        for output in std::mem::replace(&mut self.outputs, fresh) {
            g_object_unref(output.monitor);
        }
        println!(
            "OUTPUTS {}",
            self.outputs
                .iter()
                .map(|x| x.id.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    unsafe fn send_state(&self, view: P) {
        let Some(instance) = self.manager.instances.values().find(|i| i.view == view) else {
            return;
        };
        let context = serde_json::json!({"id":instance.spec.id,"instance":instance.key,"output":instance.output});
        let mut js =
            format!("window.dispatchEvent(new CustomEvent('shell:context',{{detail:{context}}}));");
        if let Some(state) = &self.workspace_state {
            let payload = serde_json::to_string(state).unwrap();
            js.push_str(&format!(
                "window.dispatchEvent(new CustomEvent('shell:workspaces',{{detail:{payload}}}));"
            ));
        }
        let script = c(&js);
        webkit_web_view_evaluate_javascript(
            view,
            script.as_ptr(),
            -1,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
    }
    unsafe fn refresh_workspaces(&mut self) {
        match self.hypr.state() {
            Ok(state) if self.workspace_state.as_ref() != Some(&state) => {
                println!(
                    "WORKSPACES current={:?} list={:?}",
                    state.current,
                    state.list.iter().map(|w| w.id).collect::<Vec<_>>()
                );
                self.workspace_state = Some(state);
                for instance in self.manager.instances.values() {
                    if instance.spec.page.trusted() {
                        self.send_state(instance.view);
                    }
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("workspace state: {e}"),
        }
    }
    unsafe fn poll(&mut self, app_ptr: P) {
        if self.outputs_dirty {
            self.outputs_dirty = false;
            self.snapshot_outputs();
        }
        if let Ok(text) = fs::read_to_string(&self.config_path) {
            if text != self.config_text && self.last_bad.as_deref() != Some(text.as_str()) {
                match &self.pending_config {
                    Some((pending, since))
                        if pending == &text && since.elapsed() >= Duration::from_millis(250) =>
                    {
                        match config::parse(&text, &self.root) {
                            Ok(specs) => {
                                self.specs = specs;
                                self.manager.reconcile(
                                    &self.specs,
                                    &self.outputs,
                                    &self.root,
                                    app_ptr,
                                );
                                self.config_text = text;
                                self.last_bad = None;
                                println!("CONFIG reload");
                            }
                            Err(e) => {
                                eprintln!("CONFIG_ERROR (previous layout kept): {e}");
                                self.last_bad = Some(text);
                            }
                        }
                        self.pending_config = None;
                    }
                    Some((pending, _)) if pending == &text => {}
                    _ => self.pending_config = Some((text, Instant::now())),
                }
            } else {
                self.pending_config = None;
            }
        }
        let fresh = stamp(&self.root.join("assets"));
        if fresh != self.asset_stamp {
            self.asset_stamp = fresh;
            self.manager.reload_local();
        }
        if self.hypr.changed() {
            self.refresh_workspaces();
        }
        for key in std::mem::take(&mut self.failed) {
            if let Some(old) = self.manager.instances.get(&key) {
                let spec = old.spec.clone();
                let output = old.output.clone();
                if let Some(monitor) = self
                    .outputs
                    .iter()
                    .find(|o| o.id == output)
                    .map(|o| o.monitor)
                {
                    self.manager.destroy(&key);
                    if let Err(e) =
                        self.manager
                            .create(key.clone(), spec, monitor, output, &self.root, app_ptr)
                    {
                        eprintln!("recovery failed {key}: {e}");
                    } else {
                        println!("RECOVERED {key}");
                    }
                }
            }
        }
        if let Some(path) = &self.control_path {
            if let Ok(text) = fs::read_to_string(path) {
                if text != self.control_text {
                    self.control_text = text.clone();
                    let command = text.trim();
                    match command {
                        "quit" => g_main_loop_quit(self.main_loop),
                        "reload" => self.manager.reload_local(),
                        x if x.starts_with("toggle ") => self
                            .manager
                            .toggle(x.trim_start_matches("toggle ").trim(), None),
                        x if x.starts_with("crash ") => {
                            let id = x.trim_start_matches("crash ").trim();
                            if let Some(w) = self
                                .manager
                                .instances
                                .values()
                                .find(|w| w.key == id || w.spec.id == id)
                            {
                                webkit_web_view_terminate_web_process(w.view);
                            }
                        }
                        x if x.starts_with("workspace ") => {
                            if let Ok(id) = x.trim_start_matches("workspace ").trim().parse() {
                                if let Err(e) = self.hypr.activate(id) {
                                    eprintln!("workspace activation: {e}");
                                }
                            }
                        }
                        "" => {}
                        x => eprintln!("unknown control command: {x}"),
                    }
                }
            }
        }
    }
}
fn stamp(dir: &Path) -> u128 {
    fn visit(path: &Path, total: &mut u128) {
        if let Ok(entries) = fs::read_dir(path) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    visit(&p, total)
                } else if let Ok(m) = e.metadata() {
                    let t = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map_or(0, |t| t.as_nanos());
                    *total = total.wrapping_add(t ^ m.len() as u128);
                }
            }
        }
    }
    let mut n = 0;
    visit(dir, &mut n);
    n
}
unsafe extern "C" fn on_tick(data: P) -> c_int {
    let app = &mut *(data as *mut App);
    app.poll(data);
    1
}
unsafe extern "C" fn on_monitors(
    _model: P,
    _pos: c_uint,
    _removed: c_uint,
    _added: c_uint,
    data: P,
) {
    (*(data as *mut App)).outputs_dirty = true;
}
unsafe extern "C" fn on_load(view: P, event: c_int, data: P) {
    if event != 3 {
        return;
    }
    let app = &*(data as *const App);
    if let Some((key, instance)) = app.manager.instances.iter().find(|(_, i)| i.view == view) {
        println!("LOADED {key} {}", instance.uri);
        if instance.spec.page.trusted() {
            app.send_state(view);
        }
    }
}
unsafe extern "C" fn on_terminated(view: P, reason: c_int, data: P) {
    let app = &mut *(data as *mut App);
    if let Some(key) = app
        .manager
        .instances
        .iter()
        .find(|(_, i)| i.view == view)
        .map(|(key, _)| key.clone())
    {
        eprintln!("WEB_PROCESS_TERMINATED {key} reason={reason}");
        app.failed.push(key);
    }
}
unsafe extern "C" fn on_title(view: P, _pspec: P, data: P) {
    let app = &*(data as *const App);
    if let Some((key, _)) = app.manager.instances.iter().find(|(_, i)| i.view == view) {
        let title = webkit_web_view_get_title(view);
        if !title.is_null() {
            println!(
                "DOM_TITLE {key} {}",
                CStr::from_ptr(title).to_string_lossy()
            );
        }
    }
}
unsafe extern "C" fn on_policy(view: P, decision: P, kind: c_int, data: P) -> c_int {
    if kind != 0 {
        return 0;
    }
    let action = webkit_navigation_policy_decision_get_navigation_action(decision);
    if action.is_null() {
        return 0;
    }
    let request = webkit_navigation_action_get_request(action);
    if request.is_null() {
        return 0;
    }
    let ptr = webkit_uri_request_get_uri(request);
    if ptr.is_null() {
        return 0;
    }
    let uri = CStr::from_ptr(ptr).to_string_lossy();
    let app = &mut *(data as *mut App);
    let Some(instance) = app.manager.instances.values().find(|i| i.view == view) else {
        return 0;
    };
    if instance.spec.page.trusted() {
        if !instance.spec.page.allows_navigation(&app.root, &uri) {
            eprintln!("blocked navigation from {} to {}", instance.key, uri);
            webkit_policy_decision_ignore(decision);
            return 1;
        }
    }
    0
}
unsafe extern "C" fn on_message(manager: P, value: P, data: P) {
    let text = jsc_value_to_string(value);
    if text.is_null() {
        return;
    }
    let src = CStr::from_ptr(text).to_string_lossy().into_owned();
    g_free(text.cast());
    let app = &mut *(data as *mut App);
    let Some((key, output)) = app
        .manager
        .instances
        .iter()
        .find(|(_, i)| i.content_manager == manager)
        .map(|(k, i)| (k.clone(), i.output.clone()))
    else {
        return;
    };
    match serde_json::from_str::<BridgeCommand>(&src) {
        Ok(BridgeCommand::SurfaceToggle { id }) => app.manager.toggle(&id, Some(&output)),
        Ok(BridgeCommand::WorkspaceActivate { id }) => {
            if let Err(e) = app.hypr.activate(id) {
                eprintln!("workspace activation: {e}");
            } else {
                app.refresh_workspaces();
            }
        }
        Ok(BridgeCommand::InputRegions { rects }) => app.manager.set_regions(&key, rects),
        Err(e) => eprintln!("bridge message rejected from {key}: {e}"),
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || !["--preview", "--layer"].contains(&args[1].as_str()) {
        eprintln!("usage: webshell (--preview|--layer) [--config FILE] [--control FILE]");
        std::process::exit(2);
    }
    let root = std::env::current_dir()
        .expect("current directory")
        .canonicalize()
        .unwrap();
    let mut config_path = root.join("surfaces.json");
    let mut control_path = None;
    let mut i = 2;
    while i < args.len() {
        if i + 1 >= args.len() {
            eprintln!("missing value for {}", args[i]);
            std::process::exit(2)
        }
        match args[i].as_str() {
            "--config" => config_path = PathBuf::from(&args[i + 1]),
            "--control" => control_path = Some(PathBuf::from(&args[i + 1])),
            x => {
                eprintln!("unknown option: {x}");
                std::process::exit(2)
            }
        }
        i += 2;
    }
    let text = fs::read_to_string(&config_path).expect("read config");
    let specs = config::parse(&text, &root).unwrap_or_else(|e| {
        eprintln!("config error: {e}");
        std::process::exit(2)
    });
    unsafe {
        gtk_init();
        let layer_mode = args[1] == "--layer";
        if layer_mode && gtk_layer_is_supported() == 0 {
            eprintln!("layer-shell unavailable; use --preview");
            std::process::exit(3);
        }
        let display = gdk_display_get_default();
        assert!(!display.is_null(), "GTK display unavailable");
        let monitors = gdk_display_get_monitors(display);
        let loop_ = g_main_loop_new(ptr::null_mut(), 0);
        let app = Box::new(App {
            root: root.clone(),
            config_path,
            control_path,
            config_text: text,
            pending_config: None,
            last_bad: None,
            control_text: String::new(),
            asset_stamp: stamp(&root.join("assets")),
            specs,
            outputs: Vec::new(),
            monitors,
            outputs_dirty: false,
            manager: SurfaceManager::new(layer_mode),
            hypr: Hyprland::new(),
            workspace_state: None,
            failed: Vec::new(),
            main_loop: loop_,
        });
        let ptr = Box::into_raw(app);
        g_signal_connect_data(
            monitors,
            c("items-changed").as_ptr(),
            on_monitors as *const c_void,
            ptr.cast(),
            ptr::null_mut(),
            0,
        );
        (*ptr).snapshot_outputs();
        (*ptr).refresh_workspaces();
        g_timeout_add(150, on_tick, ptr.cast());
        println!("READY mode={} surfaces={}", args[1], (*ptr).manager.count());
        g_main_loop_run(loop_);
        let app = &mut *ptr;
        for key in app.manager.instances.keys().cloned().collect::<Vec<_>>() {
            app.manager.destroy(&key);
        }
        for output in std::mem::take(&mut app.outputs) {
            g_object_unref(output.monitor);
        }
        g_main_loop_unref(loop_);
        drop(Box::from_raw(ptr));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wildcard_expands_to_stable_per_output_ids() {
        let root = std::env::current_dir().unwrap();
        let specs=config::parse(r#"{"surfaces":[{"id":"bar","page":"assets/bar.html","layer":"top","anchors":["left","right"],"height":44,"output":"*"}]}"#,&root).unwrap();
        let outputs = vec![
            Output {
                id: "eDP-1".into(),
                monitor: 1usize as P,
            },
            Output {
                id: "DP-2".into(),
                monitor: 2usize as P,
            },
        ];
        let expanded = SurfaceManager::expand(&specs, &outputs);
        assert_eq!(
            expanded.keys().cloned().collect::<Vec<_>>(),
            vec!["bar@DP-2", "bar@eDP-1"]
        );
    }
}
