use tracing::Level;
use tracing_subscriber::EnvFilter;

use crate::args::LogLevel;

/// Initializes a console tracing subscriber at the requested level. Safe to call once at
/// startup; a second call is a silent no-op (matches `hematite-cli`'s `logging::init`).
pub fn init(log_level: LogLevel, json: bool) {
    #[cfg(windows)]
    let _ = colored::control::set_virtual_terminal(true);

    let level = match log_level {
        LogLevel::Quiet => Level::ERROR,
        LogLevel::Normal => Level::WARN,
        LogLevel::Verbose => Level::DEBUG,
        LogLevel::Trace => Level::TRACE,
    };

    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    let result = if json {
        tracing_subscriber::fmt().json().with_env_filter(filter).try_init()
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .try_init()
    };
    let _ = result;
}
