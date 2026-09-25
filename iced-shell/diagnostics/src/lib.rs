use std::sync::{OnceLock, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};
pub static COUNTERS: [AtomicU64; 17] = [const { AtomicU64::new(0) }; 17];
pub const NAMES: [&str; 17] = ["wakeups", "messages", "views", "layouts", "redraw_requests", "renders", "presents", "hypr_queries", "hypr_events", "outputs", "config_checks", "config_reloads", "clock_fires", "pointer", "keyboard", "refresh_callbacks", "draws"];
pub fn enabled() -> bool { *ENABLED.get_or_init(|| std::env::var_os("SHELL_DIAGNOSTICS").is_some()) }
static ENABLED: OnceLock<bool> = OnceLock::new();
#[inline] pub fn inc(i: usize) { if enabled() { COUNTERS[i].fetch_add(1, Ordering::Relaxed); } }
pub fn start() {
 if !enabled() { return; }
 std::thread::spawn(|| {
  let mut last = Instant::now();
  loop {
   std::thread::sleep(Duration::from_secs(10));
   let elapsed = last.elapsed().as_secs_f64(); last = Instant::now();
   let fields: Vec<_> = NAMES.iter().enumerate().map(|(i,n)| format!("\"{n}\":{}", COUNTERS[i].swap(0, Ordering::Relaxed))).collect();
   eprintln!("DIAGNOSTICS {{\"seconds\":{elapsed:.6},{}}}", fields.join(","));
  }
 });
}

struct BackendLogger;
impl log::Log for BackendLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool { metadata.target().starts_with("iced_wgpu") && metadata.level() <= log::Level::Info }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) { eprintln!("RENDERER {}", record.args()); }
    }
    fn flush(&self) {}
}
static LOGGER: BackendLogger = BackendLogger;
pub fn log_backend() {
    if enabled() {
        let _ = log::set_logger(&LOGGER);
        log::set_max_level(log::LevelFilter::Info);
    }
}
