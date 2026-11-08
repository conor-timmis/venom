//! Logging bootstrap for the desktop process.

use tracing_subscriber::EnvFilter;

const DEFAULT_LOG_FILTER: &str = "info";

/// Installs the process-wide `tracing` subscriber.
///
/// `RUST_LOG` overrides the default filter so the prototype can be inspected
/// without rebuilding.
pub fn init() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER)),
        )
        .init();
}
