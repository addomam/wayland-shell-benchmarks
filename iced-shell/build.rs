fn main() {
    if let Ok(paths) = std::env::var("LD_LIBRARY_PATH") {
        for path in paths.split(':').filter(|p| !p.is_empty()) {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{path}");
        }
    }
    println!("cargo:rerun-if-env-changed=LD_LIBRARY_PATH");
}
