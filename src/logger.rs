use std::io::Write;

use log::{LevelFilter, Log, Metadata, Record};

const DEFAULT_LEVEL: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::Debug
} else {
    LevelFilter::Warn
};

/// Writes this workspace's records to stderr; dependency logs are dropped.
struct StderrLogger;

impl Log for StderrLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let target = metadata.target();
        metadata.level() <= log::max_level()
            && (target.starts_with(env!("CARGO_CRATE_NAME")) || target.starts_with("razer_"))
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Ignored: a closed or missing stderr must not take the app down.
            let _ = writeln!(
                std::io::stderr().lock(),
                "[{} {}] {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

/// `OPEN_SYNAPSE_LOG=debug` raises the level in release builds.
pub fn init() {
    static LOGGER: StderrLogger = StderrLogger;
    log::set_logger(&LOGGER).expect("logger is initialised once");
    let level = std::env::var("OPEN_SYNAPSE_LOG")
        .ok()
        .and_then(|l| l.parse().ok())
        .unwrap_or(DEFAULT_LEVEL);
    log::set_max_level(level);
}
