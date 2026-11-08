//! Application composition and lifecycle.
//!
//! The windowed path wires the feed and Slint bridge together. The headless
//! path runs the same feed without creating a window so CI can exercise the
//! full 10k-row workload on machines without a display server.

use std::{
    error::Error,
    fmt,
    rc::Rc,
    time::{Duration, Instant},
};

use slint::{ComponentHandle, Timer};
use time::UtcOffset;
use venom_core::SyntheticConfigError;

use crate::{
    MainWindow,
    config::{ConfigError, HarnessConfig, LIVE_TICK_HZ},
    feed::{ChatFeed, FeedSnapshot},
    format::Clock,
    ui_bridge::ChatView,
};

/// The Slint timer in `main.slint` and the feed share this interval.
const LIVE_TICK_INTERVAL: Duration = Duration::from_millis(1000 / LIVE_TICK_HZ as u64);

/// Errors that prevent the prototype from starting or completing a run.
#[derive(Debug)]
pub enum AppError {
    Config(ConfigError),
    Synthetic(SyntheticConfigError),
    Platform(slint::PlatformError),
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "configuration error: {error}"),
            Self::Synthetic(error) => write!(formatter, "synthetic feed error: {error}"),
            Self::Platform(error) => write!(formatter, "Slint error: {error}"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Synthetic(error) => Some(error),
            Self::Platform(error) => Some(error),
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<SyntheticConfigError> for AppError {
    fn from(error: SyntheticConfigError) -> Self {
        Self::Synthetic(error)
    }
}

impl From<slint::PlatformError> for AppError {
    fn from(error: slint::PlatformError) -> Self {
        Self::Platform(error)
    }
}

pub fn run() -> Result<(), AppError> {
    crate::logging::init();
    let config = HarnessConfig::from_env()?;
    tracing::info!(?config, "starting Venom");

    if config.headless {
        run_headless(&config)
    } else {
        run_window(&config)
    }
}

fn run_window(config: &HarnessConfig) -> Result<(), AppError> {
    let window = MainWindow::new()?;
    let feed = Rc::new(ChatFeed::new(config, Clock::local())?);
    let view = ChatView::new(&window, feed);
    view.bind(&window);
    view.start();

    if let Some(delay) = config.exit_after {
        tracing::info!(?delay, "smoke-test exit scheduled");
        Timer::single_shot(delay, || {
            let _ = slint::quit_event_loop();
        });
    }

    window.run()?;
    tracing::info!("Venom desktop stopped");
    Ok(())
}

fn run_headless(config: &HarnessConfig) -> Result<(), AppError> {
    // Headless output is deterministic, so pin the display offset to UTC.
    let feed = ChatFeed::new(config, Clock::fixed(UtcOffset::UTC))?;

    let prefill_started = Instant::now();
    feed.reload();
    report(
        "headless prefill",
        prefill_started.elapsed(),
        &feed.snapshot(),
    );

    if let Some(duration) = config.exit_after {
        let live_started = Instant::now();
        simulate_live(&feed, duration);
        report("headless live", live_started.elapsed(), &feed.snapshot());
    }

    Ok(())
}

/// Drives the feed at the same cadence as the windowed Slint timer.
fn simulate_live(feed: &ChatFeed, duration: Duration) {
    let deadline = Instant::now() + duration;

    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if remaining.is_zero() {
            break;
        }

        feed.tick();
        std::thread::sleep(LIVE_TICK_INTERVAL.min(remaining));
    }
}

fn report(stage: &str, elapsed: Duration, snapshot: &FeedSnapshot) {
    tracing::info!(
        stage,
        elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        retained = snapshot.retained,
        generated = snapshot.generated,
        evicted = snapshot.evicted,
        last_insert_micros = snapshot.last_insert_micros,
        worst_insert_micros = snapshot.worst_insert_micros,
        rate = snapshot.rate,
        "synthetic feed report"
    );
}

#[cfg(test)]
mod tests {
    use super::{LIVE_TICK_HZ, LIVE_TICK_INTERVAL};
    use std::time::Duration;

    #[test]
    fn live_tick_interval_matches_the_shared_rate() {
        assert_eq!(LIVE_TICK_HZ, 20);
        assert_eq!(LIVE_TICK_INTERVAL, Duration::from_millis(50));
    }
}
