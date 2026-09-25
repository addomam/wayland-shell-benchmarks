//! Event producers filter unchanged values before waking the iced UI.
use crate::{Message, hyprland::Hyprland};
use iced::{
    Subscription,
    futures::{SinkExt, StreamExt},
};
use notify::{EventKind, RecursiveMode, Watcher};
use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncBufReadExt, BufReader};

pub fn files(config: PathBuf, control: PathBuf, pages: Vec<PathBuf>) -> Subscription<Message> {
    Subscription::run_with((config, control, pages), |paths| {
        let (config, control, pages) = paths.clone();
        iced::stream::channel(32, async move |mut out| {
            let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
            let watched_config = config.clone();
            let watched_control = control.clone();
            let watched_pages = pages.clone();
            let mut watcher =
                notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                    if let Ok(event) = event {
                        if matches!(
                            event.kind,
                            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                        ) {
                            let paths: Vec<_> = event
                                .paths
                                .into_iter()
                                .filter(|p| {
                                    p == &watched_config
                                        || p == &watched_control
                                        || watched_pages.contains(p)
                                })
                                .collect();
                            if !paths.is_empty() {
                                let _ = tx.unbounded_send(paths);
                            }
                        }
                    }
                })
                .expect("inotify watcher");
            let mut dirs = std::collections::BTreeSet::new();
            dirs.insert(config.parent().unwrap().to_path_buf());
            dirs.insert(control.parent().unwrap().to_path_buf());
            dirs.extend(pages.iter().filter_map(|p| p.parent().map(PathBuf::from)));
            for dir in dirs {
                if let Err(error) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
                    eprintln!("WATCH_ERROR {}: {error}", dir.display());
                }
            }
            let mut pending = false;
            loop {
                let paths = if pending {
                    match tokio::time::timeout(Duration::from_millis(250), rx.next()).await {
                        Ok(paths) => paths,
                        Err(_) => {
                            pending = false;
                            if out.send(Message::FilesChanged).await.is_err() {
                                break;
                            }
                            continue;
                        }
                    }
                } else {
                    rx.next().await
                };
                let Some(paths) = paths else {
                    break;
                };
                if paths.iter().any(|p| p == &control) {
                    if out.send(Message::Control).await.is_err() {
                        break;
                    }
                }
                if paths.iter().any(|p| p == &config || pages.contains(p)) {
                    pending = true;
                }
            }
        })
    })
}

pub fn clock(period: u64, ids: Vec<iced::window::Id>) -> Subscription<Message> {
    Subscription::run_with((period, ids), |data| {
        let (period, ids) = data.clone();
        iced::stream::channel(1, async move |mut out| {
            loop {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                let next = Duration::from_secs((now.as_secs() / period + 1) * period);
                tokio::time::sleep(next - now).await;
                shell_diagnostics::inc(12);
                let text = clock_text(period);
                if out.send(Message::Clock(text, ids.clone())).await.is_err() {
                    break;
                }
            }
        })
    })
}

pub fn workspaces() -> Subscription<Message> {
    Subscription::run(|| {
        iced::stream::channel(4, async |mut out| {
            let hypr = Hyprland::new();
            let mut previous = None;
            loop {
                let Some(path) = hypr.event_path() else {
                    std::future::pending::<()>().await;
                    return;
                };
                match tokio::net::UnixStream::connect(path).await {
                    Ok(stream) => {
                        let mut lines = BufReader::new(stream).lines();
                        // Refresh after connecting, including after any interrupted connection.
                        if let Ok(state) = hypr.state() {
                            if previous.as_ref() != Some(&state) {
                                previous = Some(state.clone());
                                if out.send(Message::Workspaces(state)).await.is_err() {
                                    return;
                                }
                            }
                        }
                        while let Ok(Some(line)) = lines.next_line().await {
                            shell_diagnostics::inc(8);
                            let kind = line.split_once(">>").map(|(k, _)| k).unwrap_or("");
                            if matches!(
                                kind,
                                "workspacev2"
                                    | "createworkspacev2"
                                    | "destroyworkspacev2"
                                    | "focusedmonv2"
                                    | "moveworkspacev2"
                                    | "renameworkspace"
                            ) {
                                if let Ok(state) = hypr.state() {
                                    if previous.as_ref() != Some(&state) {
                                        previous = Some(state.clone());
                                        if out.send(Message::Workspaces(state)).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("Hyprland event socket: {e}"),
                }
                // Retry only on disconnection; no heartbeat while connected.
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
    })
}

pub fn clock_text(period: u64) -> String {
    if period == 0 {
        return String::new();
    }
    let sec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if period == 60 {
        format!("{:02}:{:02} UTC", sec / 3600 % 24, sec / 60 % 60)
    } else {
        format!(
            "{:02}:{:02}:{:02} UTC",
            sec / 3600 % 24,
            sec / 60 % 60,
            sec % 60
        )
    }
}
