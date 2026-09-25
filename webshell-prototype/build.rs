use std::process::Command;

fn main() {
    let output = Command::new("pkg-config")
        .args([
            "--libs-only-L",
            "gtk4",
            "webkitgtk-6.0",
            "gtk4-layer-shell-0",
        ])
        .output()
        .expect("pkg-config is required");
    assert!(output.status.success(), "pkg-config failed");
    for flag in String::from_utf8(output.stdout).unwrap().split_whitespace() {
        if let Some(dir) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={dir}");
            println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        }
    }
}
