mod config;
mod events;
mod hyprland;
use config::{Edge, Kind, Page, SurfaceSpec};
use iced::{
    Color, Element, Length, Subscription, Task, Theme,
    widget::{button, column, container, row, space, text, text_input},
    window,
};
use iced_exwlshell::{
    actions::{ActionCallback, IcedXdgWindowSettings},
    daemon,
    reexport::{
        Anchor, Extent, KeyboardInteractivity, Layer, LayerSize, NewLayerShellSettings,
        OutputOption, PixelSize,
    },
    settings::{LayerShellSettings, Settings, StartMode},
    to_layer_message,
};
use std::{collections::BTreeMap, path::PathBuf};

#[to_layer_message(multi)]
#[derive(Clone, Debug)]
enum Message {
    FilesChanged,
    Control,
    Clock(String, Vec<window::Id>),
    Workspaces(hyprland::WorkspaceState),
    Output(iced_exwlshell::shell::ShellEvent),
    Toggle(String, String),
    Close(window::Id),
    Filter(window::Id, String),
    Activate(i64),
    Opened(window::Id),
    Escape(window::Id),
    Sized(window::Id, iced::Size),
}
struct Instance {
    id: window::Id,
    spec: SurfaceSpec,
    output: String,
    page: Page,
    query: String,
    size: Option<iced::Size>,
}
struct SurfaceManager {
    instances: BTreeMap<String, Instance>,
}
struct App {
    manager: SurfaceManager,
    desired: Vec<SurfaceSpec>,
    root: PathBuf,
    config: PathBuf,
    control: PathBuf,
    last_control: String,
    config_text: String,
    page_paths: Vec<PathBuf>,
    shell_events: iced_exwlshell::shell::ShellReceiver,
    output_names: BTreeMap<u32, String>,
    hypr: hyprland::Hyprland,
    state: Option<hyprland::WorkspaceState>,
    outputs: Vec<String>,
    preview: bool,
    static_mode: bool,
    clock_period: u64,
    clock: String,
}
fn main() -> iced_exwlshell::Result {
    shell_diagnostics::start();
    shell_diagnostics::log_backend();
    let (sender, receiver) = iced_exwlshell::shell::channel();
    daemon(
        move || App::new(receiver.clone()),
        "iced-shell",
        App::update,
        App::view,
    )
    .settings(Settings {
        shell_broadcast: sender,
        layer_settings: LayerShellSettings {
            start_mode: StartMode::Background,
            ..Default::default()
        },
        ..Default::default()
    })
    .theme(|_: &App, _| Some(Theme::Dark))
    .style(|_, _| iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: Color::WHITE,
    })
    .redraw_scope(|message| {
        use iced_exwlshell::redraw::Scope;
        match message {
            Message::Clock(_, ids) => Scope::Windows(ids.clone()),
            Message::Filter(id, _) => Scope::Window(*id),
            Message::Activate(_) | Message::Sized(_, _) | Message::Opened(_) => Scope::None,
            _ => Scope::All,
        }
    })
    .on_new_shell(|info| Some(Message::Opened(info.window)))
    .subscription(App::subscription)
    .run()
}
impl SurfaceManager {
    fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
        }
    }
    fn create(
        &mut self,
        key: String,
        spec: SurfaceSpec,
        output: String,
        page: Page,
        preview: bool,
    ) -> Task<Message> {
        let id = window::Id::unique();
        let task = if preview {
            Task::done(Message::NewBaseWindow {
                settings: IcedXdgWindowSettings {
                    size: Some(
                        PixelSize::try_px(
                            if spec.width == 0 { 800 } else { spec.width },
                            if spec.height == 0 { 600 } else { spec.height },
                        )
                        .unwrap(),
                    ),
                    ..Default::default()
                },
                id,
            })
        } else {
            Task::done(Message::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: LayerSize {
                        width: Extent::try_px(spec.width).unwrap_or(Extent::Fill),
                        height: Extent::try_px(spec.height).unwrap_or(Extent::Fill),
                    },
                    layer: match spec.layer {
                        config::Layer::Top => Layer::Top,
                        config::Layer::Bottom => Layer::Bottom,
                        config::Layer::Background => Layer::Background,
                        config::Layer::Overlay => Layer::Overlay,
                    },
                    anchor: spec.anchors.iter().fold(Anchor::empty(), |a, e| {
                        a | match e {
                            Edge::Top => Anchor::Top,
                            Edge::Bottom => Anchor::Bottom,
                            Edge::Left => Anchor::Left,
                            Edge::Right => Anchor::Right,
                        }
                    }),
                    exclusive_zone: Some(spec.exclusive_zone),
                    margin: Some((
                        spec.margins.top,
                        spec.margins.right,
                        spec.margins.bottom,
                        spec.margins.left,
                    )),
                    keyboard_interactivity: match spec.keyboard_interactivity {
                        config::Keyboard::None => KeyboardInteractivity::None,
                        config::Keyboard::Exclusive => KeyboardInteractivity::Exclusive,
                        config::Keyboard::OnDemand => KeyboardInteractivity::OnDemand,
                    },
                    output_option: OutputOption::OutputName(output.clone()),
                    namespace: Some(format!("iced-shell-{key}")),
                    ..Default::default()
                },
                id,
            })
        };
        eprintln!("CREATE {key}");
        self.instances.insert(
            key,
            Instance {
                id,
                spec,
                output,
                page,
                query: String::new(),
                size: None,
            },
        );
        task
    }
    fn destroy(&mut self, key: &str) -> Task<Message> {
        if let Some(i) = self.instances.remove(key) {
            eprintln!("DESTROY {key}");
            Task::done(Message::RemoveWindow(i.id))
        } else {
            Task::none()
        }
    }
    fn reconcile(
        &mut self,
        specs: &[SurfaceSpec],
        outputs: &[String],
        root: &std::path::Path,
        preview: bool,
    ) -> Task<Message> {
        let mut wanted = BTreeMap::new();
        for spec in specs {
            if !spec.visible {
                continue;
            }
            for output in outputs {
                if spec.output == "*" || spec.output == *output {
                    wanted.insert(
                        format!("{}@{}", spec.id, output),
                        (spec.clone(), output.clone()),
                    );
                }
            }
        }
        let mut tasks = vec![];
        let keys: Vec<_> = self.instances.keys().cloned().collect();
        for key in keys {
            if !wanted.contains_key(&key) {
                tasks.push(self.destroy(&key));
            }
        }
        for (key, (spec, output)) in wanted {
            let page = match config::page(root, &spec.page) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("PAGE_ERROR {e}");
                    continue;
                }
            };
            if let Some(i) = self.instances.get_mut(&key) {
                let mut old = i.spec.clone();
                old.page = spec.page.clone();
                if old == spec {
                    if i.page != page {
                        eprintln!("CONTENT {key}");
                        i.page = page;
                        tasks.push(Task::done(Message::Opened(i.id)));
                    }
                    i.spec = spec;
                    continue;
                }
                tasks.push(self.destroy(&key));
            }
            tasks.push(self.create(key, spec, output, page, preview));
        }
        Task::batch(tasks)
    }
}
impl App {
    fn new(shell_events: iced_exwlshell::shell::ShellReceiver) -> (Self, Task<Message>) {
        let mut config = PathBuf::from("surfaces.json");
        let mut control = PathBuf::from("/tmp/iced-shell-control");
        let mut preview = false;
        let mut args = std::env::args().skip(1);
        while let Some(a) = args.next() {
            match a.as_str() {
                "--config" => config = args.next().expect("--config needs path").into(),
                "--control" => control = args.next().expect("--control needs path").into(),
                "--preview" => preview = true,
                "--layer" => {}
                _ => {
                    eprintln!(
                        "Usage: iced-shell [--layer|--preview] [--config PATH] [--control PATH]"
                    );
                    std::process::exit(2)
                }
            }
        }
        let root = std::env::current_dir().unwrap();
        let config = root.join(config);
        let control = root.join(control);
        let config_text = std::fs::read_to_string(&config).expect("read configuration");
        let mut desired = config::parse(&config_text, &root).unwrap_or_else(|e| {
            eprintln!("CONFIG_ERROR {e}");
            std::process::exit(2)
        });
        let hypr = hyprland::Hyprland::new();
        let static_mode = std::env::var_os("SHELL_STATIC").is_some();
        if static_mode {
            desired.truncate(1);
            desired[0].output =
                std::env::var("SHELL_STATIC_OUTPUT").unwrap_or_else(|_| "eDP-1".into());
        }
        let outputs = Vec::new();
        let state = None;
        let clock_period = std::env::var("SHELL_CLOCK")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let last_control = std::fs::read_to_string(&control).unwrap_or_default();
        let page_paths = page_watches(&root, &desired);
        let mut app = Self {
            manager: SurfaceManager::new(),
            desired,
            root,
            config,
            control,
            last_control,
            config_text,
            page_paths,
            shell_events,
            output_names: BTreeMap::new(),
            hypr,
            state,
            outputs,
            preview,
            static_mode,
            clock_period,
            clock: events::clock_text(if static_mode { 0 } else { clock_period }),
        };
        let task = app.reconcile();
        (app, task)
    }
    fn reconcile(&mut self) -> Task<Message> {
        self.manager
            .reconcile(&self.desired, &self.outputs, &self.root, self.preview)
    }
    fn toggle(&mut self, id: &str, output: Option<&str>) -> Task<Message> {
        let specs: Vec<_> = self
            .desired
            .iter()
            .filter(|s| s.id == id)
            .cloned()
            .collect();
        let mut tasks = vec![];
        for spec in specs {
            let out = self
                .outputs
                .iter()
                .filter(|out| {
                    (spec.output == "*" || spec.output == ***out)
                        && output.is_none_or(|o| o == **out)
                })
                .min()
                .cloned();
            if let Some(out) = out {
                let key = format!("{id}@{out}");
                if self.manager.instances.contains_key(&key) {
                    tasks.push(self.manager.destroy(&key));
                    eprintln!("TOGGLE {key} hidden");
                } else if let Ok(page) = config::page(&self.root, &spec.page) {
                    tasks.push(self.manager.create(
                        key.clone(),
                        spec.clone(),
                        out,
                        page,
                        self.preview,
                    ));
                    eprintln!("TOGGLE {key} shown");
                }
            }
        }
        Task::batch(tasks)
    }
    fn update(&mut self, m: Message) -> Task<Message> {
        shell_diagnostics::inc(1);
        match m {
            Message::Toggle(id, out) => self.toggle(&id, Some(&out)),
            Message::Activate(id) => {
                if let Err(e) = self.hypr.activate(id) {
                    eprintln!("IPC_ERROR {e}")
                }
                Task::none()
            }
            Message::Close(id) | Message::Escape(id) => {
                let key = self
                    .manager
                    .instances
                    .iter()
                    .find(|(_, i)| i.id == id && i.page.kind != Kind::Bar)
                    .map(|(k, _)| k.clone());
                key.map(|k| self.manager.destroy(&k))
                    .unwrap_or_else(Task::none)
            }
            Message::Filter(id, q) => {
                if let Some(i) = self.manager.instances.values_mut().find(|i| i.id == id) {
                    i.query = q
                }
                Task::none()
            }
            Message::Opened(id) => {
                if let Some((key, i)) = self.manager.instances.iter().find(|(_, i)| i.id == id) {
                    eprintln!("READY {key}");
                    if i.page.kind == Kind::InputTest {
                        return window::size(id).map(move |size| Message::Sized(id, size));
                    }
                    if i.page.kind == Kind::Launcher {
                        return Task::done(Message::SetInputRegion {
                            id,
                            callback: ActionCallback::new(|r| r.add(0, 0, 16384, 16384)),
                        })
                        .chain(iced::widget::operation::focus(format!("search-{id}")));
                    }
                    return Task::done(Message::SetInputRegion {
                        id,
                        callback: ActionCallback::new(|r| r.add(0, 0, 16384, 16384)),
                    });
                }
                Task::none()
            }
            Message::Sized(id, size) => {
                if let Some(i) = self.manager.instances.values_mut().find(|i| i.id == id) {
                    i.size = Some(size);
                    if i.page.kind == Kind::InputTest {
                        let (x, y) = island_origin(size);
                        eprintln!(
                            "INPUT {}@{} {x},{y},250,160 size={}x{}",
                            i.spec.id, i.output, size.width, size.height
                        );
                        return Task::done(Message::SetInputRegion {
                            id,
                            callback: ActionCallback::new(move |r| r.add(x, y, 250, 160)),
                        });
                    }
                }
                Task::none()
            }
            Message::Clock(clock, _) => {
                self.clock = clock;
                Task::none()
            }
            Message::Workspaces(state) => {
                if self.state.as_ref() != Some(&state) {
                    eprintln!("WORKSPACES current={:?}", state.current);
                    self.state = Some(state);
                }
                Task::none()
            }
            Message::Output(event) => {
                use iced_exwlshell::shell::ShellEvent;
                shell_diagnostics::inc(9);
                match event {
                    ShellEvent::OutputAdded(info) | ShellEvent::OutputUpdated(info) => {
                        if let Some(name) = info.name {
                            self.output_names.insert(info.id, name);
                        }
                    }
                    ShellEvent::OutputRemoved(info) => {
                        self.output_names.remove(&info.id);
                    }
                    _ => return Task::none(),
                }
                let mut outputs: Vec<_> = self.output_names.values().cloned().collect();
                outputs.sort();
                if outputs != self.outputs {
                    self.outputs = outputs;
                    self.reconcile()
                } else {
                    Task::none()
                }
            }
            Message::FilesChanged => {
                shell_diagnostics::inc(10);
                if let Ok(current) = std::fs::read_to_string(&self.config) {
                    if current != self.config_text {
                        self.config_text = current.clone();
                        match config::parse(&current, &self.root) {
                            Ok(specs) => {
                                shell_diagnostics::inc(11);
                                self.page_paths = page_watches(&self.root, &specs);
                                self.desired = specs;
                                eprintln!("CONFIG applied");
                                return self.reconcile();
                            }
                            Err(e) => eprintln!("CONFIG_ERROR {e}"),
                        }
                    }
                }
                self.reload_pages()
            }
            Message::Control => {
                let mut tasks = Vec::new();
                if let Ok(c) = std::fs::read_to_string(&self.control) {
                    if c != self.last_control {
                        self.last_control = c.clone();
                        for line in c.lines() {
                            let mut p = line.split_whitespace();
                            match p.next() {
                                Some("quit") => return iced::exit(),
                                Some("toggle") => {
                                    if let Some(id) = p.next() {
                                        tasks.push(self.toggle(id, None));
                                    }
                                }
                                Some("workspace") => {
                                    if let Some(id) = p.next().and_then(|x| x.parse().ok()) {
                                        if let Err(e) = self.hypr.activate(id) {
                                            eprintln!("IPC_ERROR {e}");
                                        }
                                    }
                                }
                                Some("reload") => tasks.push(self.reload_pages()),
                                _ => {}
                            }
                        }
                    }
                }
                Task::batch(tasks)
            }
            _ => Task::none(),
        }
    }
    fn reload_pages(&mut self) -> Task<Message> {
        // Retain known targets while a descriptor is temporarily missing during an edit.
        self.page_paths = self
            .page_paths
            .iter()
            .cloned()
            .chain(page_watches(&self.root, &self.desired))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut tasks = Vec::new();
        for (key, i) in &mut self.manager.instances {
            match config::page(&self.root, &i.spec.page) {
                Ok(page) if page != i.page => {
                    i.page = page;
                    tasks.push(Task::done(Message::Opened(i.id)));
                    eprintln!("CONTENT {key}");
                }
                Err(e) => eprintln!("PAGE_ERROR {e}"),
                _ => {}
            }
        }
        eprintln!("CONTENT reloaded");
        Task::batch(tasks)
    }
    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            self.shell_events.listen().filter_map(|event| match event {
                iced_exwlshell::shell::ShellEvent::OutputAdded(_)
                | iced_exwlshell::shell::ShellEvent::OutputUpdated(_)
                | iced_exwlshell::shell::ShellEvent::OutputRemoved(_) => {
                    Some(Message::Output(event))
                }
                _ => None,
            }),
            iced::event::listen_with(|e, _, id| {
                match &e {
                    iced::Event::Mouse(_) => shell_diagnostics::inc(13),
                    iced::Event::Keyboard(_) => shell_diagnostics::inc(14),
                    _ => {}
                }
                match e {
                    iced::Event::Window(window::Event::Resized(size)) => {
                        Some(Message::Sized(id, size))
                    }
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                        ..
                    }) => Some(Message::Escape(id)),
                    _ => None,
                }
            }),
        ];
        if !self.static_mode {
            if std::env::var_os("SHELL_NO_WATCH").is_none() {
                subscriptions.push(events::files(
                    self.config.clone(),
                    self.control.clone(),
                    self.page_paths.clone(),
                ));
            }
            if std::env::var_os("SHELL_NO_WORKSPACES").is_none() {
                subscriptions.push(events::workspaces());
            }
            if self.clock_period > 0
                && self
                    .manager
                    .instances
                    .values()
                    .any(|i| i.page.kind == Kind::Bar)
            {
                subscriptions.push(events::clock(
                    self.clock_period,
                    self.manager
                        .instances
                        .values()
                        .filter(|i| i.page.kind == Kind::Bar)
                        .map(|i| i.id)
                        .collect(),
                ));
            }
        }
        Subscription::batch(subscriptions)
    }
    fn view(&self, id: window::Id) -> Element<'_, Message> {
        shell_diagnostics::inc(2);
        let Some(i) = self.manager.instances.values().find(|i| i.id == id) else {
            return text("").into();
        };
        let toggle = |target: &str| Message::Toggle(target.into(), i.output.clone());
        let bg = Color::from_rgb8(
            u8::from_str_radix(&i.page.background[1..3], 16).unwrap(),
            u8::from_str_radix(&i.page.background[3..5], 16).unwrap(),
            u8::from_str_radix(&i.page.background[5..7], 16).unwrap(),
        );
        if self.static_mode {
            return container(text("Static shell"))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
        let content: Element<_> = match i.page.kind {
            Kind::Bar => {
                let mut bar = row![
                    button("Apps").on_press(toggle("launcher")),
                    button("Menu").on_press(toggle("menu"))
                ]
                .spacing(8)
                .align_y(iced::Center);
                if let Some(state) = &self.state {
                    for w in &state.list {
                        bar = bar.push(
                            button(text(if state.current == Some(w.id) {
                                format!("[{}]", w.id)
                            } else {
                                w.id.to_string()
                            }))
                            .on_press(Message::Activate(w.id)),
                        );
                    }
                }
                bar.push(text(&i.page.title))
                    .push(space::horizontal())
                    .push(text(&self.clock))
                    .padding([5, 12])
                    .into()
            }
            Kind::Launcher => {
                let mut c = column![
                    row![
                        text(&i.page.title).size(26),
                        space::horizontal(),
                        button("Close").on_press(Message::Close(id))
                    ],
                    text_input("Search applications", &i.query)
                        .id(format!("search-{id}"))
                        .on_input(move |s| Message::Filter(id, s))
                ]
                .spacing(14);
                for name in ["Terminal", "Files", "Browser", "Settings"] {
                    if name.to_lowercase().contains(&i.query.to_lowercase()) {
                        c = c.push(
                            button(name)
                                .on_press(Message::Close(id))
                                .width(Length::Fill),
                        );
                    }
                }
                c.padding(24).into()
            }
            Kind::Menu => column![
                text(&i.page.title).size(22),
                button("Input experiment").on_press(toggle("input-test")),
                button("Close").on_press(Message::Close(id))
            ]
            .padding(20)
            .spacing(16)
            .into(),
            Kind::InputTest => {
                return container(
                    container(
                        column![
                            text(&i.page.title),
                            button("Close").on_press(Message::Close(id))
                        ]
                        .spacing(16)
                        .padding(20),
                    )
                    .width(250)
                    .height(160)
                    .style(move |_| container::Style {
                        background: Some(bg.into()),
                        ..Default::default()
                    }),
                )
                .center(Length::Fill)
                .into();
            }
            Kind::Generic => column![
                text(&i.page.title).size(24),
                button("Close").on_press(Message::Close(id))
            ]
            .padding(20)
            .spacing(16)
            .into(),
        };
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                ..Default::default()
            })
            .into()
    }
}

fn page_watches(root: &std::path::Path, specs: &[SurfaceSpec]) -> Vec<PathBuf> {
    specs
        .iter()
        .flat_map(|spec| {
            let path = root.join(&spec.page);
            let resolved = path.canonicalize().ok();
            std::iter::once(path).chain(resolved)
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn island_origin(size: iced::Size) -> (i32, i32) {
    (
        ((size.width - 250.0) / 2.0).floor() as i32,
        ((size.height - 160.0) / 2.0).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_region_uses_allocated_logical_size() {
        assert_eq!(island_origin(iced::Size::new(1280.0, 360.0)), (515, 100));
        assert_eq!(island_origin(iced::Size::new(2560.0, 360.0)), (1155, 100));
        assert_eq!(island_origin(iced::Size::new(620.0, 360.0)), (185, 100));
    }
    #[test]
    fn reconciliation_tracks_logical_ids_and_preserves_content_updates() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut specs = config::parse(
            &std::fs::read_to_string(root.join("surfaces.json")).unwrap(),
            &root,
        )
        .unwrap();
        let mut manager = SurfaceManager::new();
        let outputs = vec!["TEST-1".into(), "TEST-2".into()];
        let _ = manager.reconcile(&specs, &outputs, &root, false);
        assert_eq!(manager.instances.len(), 2);
        let id = manager.instances["bar@TEST-1"].id;
        specs[0].page = "pages/generic.json".into();
        let _ = manager.reconcile(&specs, &outputs, &root, false);
        assert_eq!(manager.instances["bar@TEST-1"].id, id);
        specs[0].height = 56;
        let _ = manager.reconcile(&specs, &outputs, &root, false);
        assert_ne!(manager.instances["bar@TEST-1"].id, id);
        let _ = manager.reconcile(&specs, &outputs[..1], &root, false);
        assert_eq!(manager.instances.len(), 1);
        let _ = manager.reconcile(&[], &outputs, &root, false);
        assert!(manager.instances.is_empty());
    }
}
