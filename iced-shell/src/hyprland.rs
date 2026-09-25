use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub monitor: String,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct WorkspaceState {
    pub current: Option<i64>,
    pub list: Vec<Workspace>,
}
#[derive(Deserialize)]
struct WorkspaceWire {
    id: i64,
    name: String,
    monitor: String,
}
#[derive(Deserialize)]
struct ActiveWire {
    id: i64,
}

pub struct Hyprland {
    dir: Option<PathBuf>,
}
impl Hyprland {
    pub fn new() -> Self {
        let dir = match (
            std::env::var("XDG_RUNTIME_DIR"),
            std::env::var("HYPRLAND_INSTANCE_SIGNATURE"),
        ) {
            (Ok(runtime), Ok(signature)) => {
                Some(PathBuf::from(runtime).join("hypr").join(signature))
            }
            _ => None,
        };
        Self { dir }
    }
    pub fn event_path(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(".socket2.sock"))
    }
    pub fn request(&self, command: &str) -> Result<String, String> {
        shell_diagnostics::inc(7);
        let dir = self.dir.as_ref().ok_or("Hyprland IPC unavailable")?;
        let mut stream =
            UnixStream::connect(dir.join(".socket.sock")).map_err(|e| e.to_string())?;
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
        stream
            .write_all(command.as_bytes())
            .map_err(|e| e.to_string())?;
        let mut out = String::new();
        stream.read_to_string(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }
    pub fn state(&self) -> Result<WorkspaceState, String> {
        let mut list: Vec<Workspace> =
            serde_json::from_str::<Vec<WorkspaceWire>>(&self.request("j/workspaces")?)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|x| Workspace {
                    id: x.id,
                    name: x.name,
                    monitor: x.monitor,
                })
                .collect();
        list.sort_by_key(|w| w.id);
        let active: ActiveWire =
            serde_json::from_str(&self.request("j/activeworkspace")?).map_err(|e| e.to_string())?;
        Ok(WorkspaceState {
            current: Some(active.id),
            list,
        })
    }
    pub fn activate(&self, id: i64) -> Result<(), String> {
        if !(1..=100).contains(&id) {
            return Err("workspace ID must be 1..100".into());
        }
        let reply = self.request(&format!(
            "dispatch hl.dsp.focus({{ workspace = \"{id}\" }})"
        ))?;
        if reply.trim() == "ok" {
            Ok(())
        } else {
            Err(format!("Hyprland dispatch: {reply}"))
        }
    }
}
