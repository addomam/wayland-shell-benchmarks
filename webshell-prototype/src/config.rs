use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}
impl Layer {
    pub fn native(&self) -> i32 {
        match self {
            Self::Background => 0,
            Self::Bottom => 1,
            Self::Top => 2,
            Self::Overlay => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}
impl Edge {
    pub fn native(&self) -> i32 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Top => 2,
            Self::Bottom => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Keyboard {
    #[default]
    None,
    Exclusive,
    OnDemand,
}
impl Keyboard {
    pub fn native(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Exclusive => 1,
            Self::OnDemand => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Margins {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}
impl Margins {
    pub fn edge(&self, edge: &Edge) -> i32 {
        match edge {
            Edge::Top => self.top,
            Edge::Right => self.right,
            Edge::Bottom => self.bottom,
            Edge::Left => self.left,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum OutputSelector {
    All,
    Named(String),
}
impl TryFrom<String> for OutputSelector {
    type Error = String;
    fn try_from(value: String) -> Result<Self, String> {
        if value == "*" {
            Ok(Self::All)
        } else if !value.is_empty() && !value.contains('@') {
            Ok(Self::Named(value))
        } else {
            Err("output must be '*' or a connector name".into())
        }
    }
}
fn all_outputs() -> OutputSelector {
    OutputSelector::All
}
fn visible() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Page {
    Local(PathBuf),
    Url(String),
}
impl TryFrom<String> for Page {
    type Error = String;
    fn try_from(value: String) -> Result<Self, String> {
        if value.starts_with("http://") || value.starts_with("https://") {
            if http_origin(&value).is_some() {
                Ok(Self::Url(value))
            } else {
                Err("page URL needs a host".into())
            }
        } else if value.contains("://") || value.is_empty() {
            Err("page must be a local path or HTTP(S) URL".into())
        } else {
            Ok(Self::Local(PathBuf::from(value)))
        }
    }
}

pub fn http_origin(url: &str) -> Option<&str> {
    let scheme_end = url.find("://")?;
    if !matches!(&url[..scheme_end], "http" | "https") {
        return None;
    }
    let start = scheme_end + 3;
    let end = url[start..]
        .find(['/', '?', '#'])
        .map_or(url.len(), |offset| start + offset);
    let authority = &url[start..end];
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    Some(&url[..end])
}

impl Page {
    pub fn uri(&self, root: &Path) -> Result<String, String> {
        match self {
            Self::Url(url) => Ok(url.clone()),
            Self::Local(page) => {
                let path = fs::canonicalize(root.join(page))
                    .map_err(|e| format!("{}: {e}", page.display()))?;
                if !path.starts_with(root) {
                    return Err(format!("page outside project: {}", page.display()));
                }
                let mut uri = String::from("file://");
                for b in path.to_string_lossy().as_bytes() {
                    if b.is_ascii_alphanumeric() || b"/-._~".contains(b) {
                        uri.push(*b as char);
                    } else {
                        uri.push_str(&format!("%{:02X}", b));
                    }
                }
                Ok(uri)
            }
        }
    }
    pub fn trusted(&self) -> bool {
        match self {
            Self::Local(_) => true,
            Self::Url(url) => {
                let Some(authority) = http_origin(url).and_then(|s| s.strip_prefix("http://"))
                else {
                    return false;
                };
                ["localhost", "127.0.0.1", "[::1]"].iter().any(|host| {
                    authority == *host
                        || authority
                            .strip_prefix(&format!("{host}:"))
                            .is_some_and(|p| p.parse::<u16>().is_ok_and(|n| n > 0))
                })
            }
        }
    }

    pub fn allows_navigation(&self, root: &Path, uri: &str) -> bool {
        if !self.trusted() {
            return true;
        }
        match self {
            Self::Local(_) => Self::Local(PathBuf::from("."))
                .uri(root)
                .is_ok_and(|root_uri| uri.starts_with(&format!("{root_uri}/"))),
            Self::Url(url) => http_origin(url) == http_origin(uri),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceSpec {
    pub id: String,
    pub page: Page,
    pub layer: Layer,
    #[serde(default)]
    pub anchors: Vec<Edge>,
    #[serde(default)]
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub exclusive_zone: i32,
    #[serde(default)]
    pub margins: Margins,
    #[serde(default)]
    pub keyboard_interactivity: Keyboard,
    #[serde(default = "all_outputs")]
    pub output: OutputSelector,
    #[serde(default = "visible")]
    pub visible: bool,
}
impl SurfaceSpec {
    pub fn requires_recreate(&self, other: &Self) -> bool {
        self.layer != other.layer
            || self
                .anchors
                .iter()
                .map(Edge::native)
                .collect::<BTreeSet<_>>()
                != other
                    .anchors
                    .iter()
                    .map(Edge::native)
                    .collect::<BTreeSet<_>>()
            || self.width != other.width
            || self.height != other.height
            || self.keyboard_interactivity != other.keyboard_interactivity
            || self.output != other.output
            || self.page.trusted() != other.page.trusted()
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    surfaces: Vec<SurfaceSpec>,
}

pub fn parse(src: &str, root: &Path) -> Result<Vec<SurfaceSpec>, String> {
    let config: Config = serde_json::from_str(src).map_err(|e| format!("config JSON: {e}"))?;
    let mut ids = BTreeSet::new();
    for s in &config.surfaces {
        if s.id.is_empty()
            || !s
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || !ids.insert(&s.id)
        {
            return Err(format!("invalid or duplicate surface ID: {}", s.id));
        }
        if s.width < 0
            || (s.width == 0
                && !(s.anchors.contains(&Edge::Left) && s.anchors.contains(&Edge::Right)))
            || s.height <= 0
            || s.exclusive_zone < 0
            || s.anchors.len() > 4
        {
            return Err(format!("invalid size, zone, or anchors for {}", s.id));
        }
        let mut edges = BTreeSet::new();
        for e in &s.anchors {
            if !edges.insert(e.native()) {
                return Err(format!("duplicate anchor for {}", s.id));
            }
        }
        s.page.uri(root)?;
    }
    Ok(config.surfaces)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_complete_layout_before_reconcile() {
        let dir = std::env::current_dir().unwrap();
        let good = r#"{"surfaces":[{"id":"bar","page":"assets/bar.html","layer":"top","anchors":["top","left","right"],"height":44,"output":"*"}]}"#;
        assert_eq!(parse(good, &dir).unwrap()[0].height, 44);
        assert!(parse(
            &good.replace("assets/bar.html", "assets/missing.html"),
            &dir
        )
        .is_err());
        assert!(parse(&good.replace("\"top\",\"left\"", "\"top\",\"top\""), &dir).is_err());
        assert!(parse("{", &dir).is_err());
        assert!(parse(&good.replace("\"left\",\"right\"", "\"left\""), &dir).is_err());
    }
    #[test]
    fn distinguishes_mutable_changes_from_new_native_surfaces() {
        let root = std::env::current_dir().unwrap();
        let src = r#"{"surfaces":[{"id":"bar","page":"assets/bar.html","layer":"top","anchors":["top","left","right"],"height":44,"exclusiveZone":44}]}"#;
        let original = parse(src, &root).unwrap().remove(0);
        let mut zone = original.clone();
        zone.exclusive_zone = 50;
        assert!(!original.requires_recreate(&zone));
        let mut page = original.clone();
        page.page = Page::Local("assets/menu.html".into());
        assert!(!original.requires_recreate(&page));
        let mut height = original.clone();
        height.height = 50;
        assert!(original.requires_recreate(&height));
        let mut layer = original.clone();
        layer.layer = Layer::Bottom;
        assert!(original.requires_recreate(&layer));
        let mut remote = original.clone();
        remote.page = Page::Url("https://example.com/".into());
        assert!(original.requires_recreate(&remote));
        let mut reordered = original.clone();
        reordered.anchors.reverse();
        assert!(!original.requires_recreate(&reordered));
    }

    #[test]
    fn remote_pages_do_not_receive_the_native_bridge() {
        assert!(Page::Local("assets/bar.html".into()).trusted());
        assert!(Page::Url("http://127.0.0.1:5173/dev-bar.html".into()).trusted());
        assert!(Page::Url("http://[::1]:5173/dev-bar.html".into()).trusted());
        assert!(!Page::Url("https://example.org/bar.html".into()).trusted());
        assert!(!Page::Url("http://localhost.evil/bar.html".into()).trusted());
        assert!(!Page::Url("http://localhost:5173.evil/bar.html".into()).trusted());
        assert!(!Page::Url("http://user@localhost/bar.html".into()).trusted());
        assert!(http_origin("http://localhost:5173/path").is_some());
        assert!(http_origin("http://").is_none());

        let root = std::env::current_dir().unwrap();
        let local = Page::Local("assets/bar.html".into());
        assert!(local.allows_navigation(&root, &local.uri(&root).unwrap()));
        assert!(!local.allows_navigation(&root, &format!("file://{}-other/page", root.display())));
        let loopback = Page::Url("http://localhost:5173/bar".into());
        assert!(loopback.allows_navigation(&root, "http://localhost:5173/launcher"));
        assert!(!loopback.allows_navigation(&root, "http://localhost:5173.evil/bar"));
    }
}
