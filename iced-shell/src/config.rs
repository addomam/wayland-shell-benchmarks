use serde::Deserialize;
use std::{collections::BTreeSet, path::Path};
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Layer {
    Background,
    Bottom,
    #[default]
    Top,
    Overlay,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Keyboard {
    #[default]
    None,
    Exclusive,
    OnDemand,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Margins {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceSpec {
    pub id: String,
    pub page: String,
    #[serde(default)]
    pub layer: Layer,
    #[serde(default)]
    pub anchors: Vec<Edge>,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub exclusive_zone: i32,
    #[serde(default)]
    pub margins: Margins,
    #[serde(default)]
    pub keyboard_interactivity: Keyboard,
    #[serde(default = "all")]
    pub output: String,
    #[serde(default = "yes")]
    pub visible: bool,
}
fn all() -> String {
    "*".into()
}
fn yes() -> bool {
    true
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    surfaces: Vec<SurfaceSpec>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Bar,
    Launcher,
    Menu,
    InputTest,
    Generic,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub kind: Kind,
    pub title: String,
    #[serde(default = "background")]
    pub background: String,
}
fn background() -> String {
    "#202332".into()
}
pub fn page(root: &Path, name: &str) -> Result<Page, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let file = root.join(name).canonicalize().map_err(|e| e.to_string())?;
    if !file.starts_with(&root) {
        return Err("page must stay inside configuration directory".into());
    }
    let p: Page = serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if p.background.len() != 7
        || !p.background.starts_with('#')
        || u32::from_str_radix(&p.background[1..], 16).is_err()
    {
        return Err("background requires #RRGGBB".into());
    }
    Ok(p)
}
pub fn parse(text: &str, root: &Path) -> Result<Vec<SurfaceSpec>, String> {
    let c: Config = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut ids = BTreeSet::new();
    for s in &c.surfaces {
        if s.id.is_empty()
            || !s
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || !ids.insert(&s.id)
        {
            return Err("invalid or duplicate surface ID".into());
        }
        if s.width > 16384 || s.height > 16384 || s.exclusive_zone < -1 || s.output.is_empty() {
            return Err("invalid dimensions, zone or output".into());
        }
        if s.width == 0 && !(s.anchors.contains(&Edge::Left) && s.anchors.contains(&Edge::Right))
            || s.height == 0
                && !(s.anchors.contains(&Edge::Top) && s.anchors.contains(&Edge::Bottom))
        {
            return Err("zero dimension requires opposite anchors".into());
        }
        if s.anchors
            .iter()
            .enumerate()
            .any(|(i, e)| s.anchors[..i].contains(e))
        {
            return Err("duplicate anchor".into());
        }
        page(root, &s.page)?;
    }
    Ok(c.surfaces)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_valid_and_duplicate_invalid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(root.join("surfaces.json")).unwrap();
        assert_eq!(parse(&text, root).unwrap().len(), 4);
        assert!(parse(&text.replace("\"launcher\"", "\"bar\""), root).is_err());
        assert!(parse(&text.replace("\"height\":44", "\"height\":0"), root).is_err());
    }
    #[test]
    fn rejects_escape() {
        assert!(
            page(
                Path::new(env!("CARGO_MANIFEST_DIR")),
                "../webshell-prototype/Cargo.toml"
            )
            .is_err()
        );
    }
}
