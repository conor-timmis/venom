use std::{error::Error, fmt};

use time::{Duration, OffsetDateTime};
use venom_domain::{Channel, Platform};

/// Default seed for reproducible prototype sessions and benchmarks.
pub const DEFAULT_SEED: u64 = 0x005E_ED0F_7A11;

/// 2026-01-01 is a deterministic, realistic default date for synthetic rows.
const DEFAULT_START_DAY: i64 = 20_454;
const DEFAULT_START_SECOND_OF_DAY: i64 = 18 * 60 * 60 + 30 * 60;

/// Configuration for [`SyntheticChatGenerator`](super::SyntheticChatGenerator).
#[derive(Debug, Clone)]
pub struct SyntheticConfig {
    seed: u64,
    channels: Vec<Channel>,
    start_time: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticConfigError {
    NoChannels,
}

impl fmt::Display for SyntheticConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoChannels => formatter.write_str("synthetic chat requires at least one channel"),
        }
    }
}

impl Error for SyntheticConfigError {}

impl SyntheticConfig {
    pub fn new(seed: u64, channels: Vec<Channel>) -> Result<Self, SyntheticConfigError> {
        if channels.is_empty() {
            return Err(SyntheticConfigError::NoChannels);
        }

        Ok(Self {
            seed,
            channels,
            start_time: default_start_time(),
        })
    }

    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }

    #[must_use]
    pub fn start_time(&self) -> OffsetDateTime {
        self.start_time
    }

    /// Overrides the synthetic clock origin, primarily for deterministic tests
    /// and for aligning a replay with a recorded session.
    #[must_use]
    pub fn with_start_time(mut self, start_time: OffsetDateTime) -> Self {
        self.start_time = start_time;
        self
    }

    pub(crate) fn into_parts(self) -> (u64, Vec<Channel>, OffsetDateTime) {
        (self.seed, self.channels, self.start_time)
    }
}

impl Default for SyntheticConfig {
    fn default() -> Self {
        Self {
            seed: DEFAULT_SEED,
            channels: default_channels(),
            start_time: default_start_time(),
        }
    }
}

/// Stable clock origin shared by default prototype runs.
#[must_use]
pub fn default_start_time() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
        + Duration::days(DEFAULT_START_DAY)
        + Duration::seconds(DEFAULT_START_SECOND_OF_DAY)
}

/// One channel per supported platform, used by the Phase 0 prototype.
#[must_use]
pub fn default_channels() -> Vec<Channel> {
    vec![
        Channel::new(Platform::Twitch, "venom_dev", "Venom Dev"),
        Channel::new(Platform::YouTube, "venom-live", "Venom Live"),
        Channel::new(Platform::Kick, "venom", "Venom"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{SyntheticConfig, SyntheticConfigError, default_channels, default_start_time};
    use venom_domain::Platform;

    #[test]
    fn default_config_covers_every_supported_platform() {
        let config = SyntheticConfig::default();
        let platforms: Vec<_> = config
            .channels()
            .iter()
            .map(|channel| channel.platform)
            .collect();

        assert_eq!(platforms, Platform::ALL);
    }

    #[test]
    fn config_rejects_an_empty_channel_set() {
        assert_eq!(
            SyntheticConfig::new(1, Vec::new()).unwrap_err(),
            SyntheticConfigError::NoChannels
        );
    }

    #[test]
    fn config_preserves_seed_and_channels() {
        let config = SyntheticConfig::new(42, default_channels()).expect("channels are present");

        assert_eq!(config.seed(), 42);
        assert_eq!(config.channels().len(), 3);
    }

    #[test]
    fn default_start_time_is_stable_and_realistic() {
        let config = SyntheticConfig::default();

        assert_eq!(config.start_time(), default_start_time());
        assert_eq!(config.start_time().year(), 2026);
        assert_eq!(config.start_time().hour(), 18);
        assert_eq!(config.start_time().minute(), 30);
    }

    #[test]
    fn start_time_can_be_overridden() {
        let start_time = time::OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1);
        let config = SyntheticConfig::default().with_start_time(start_time);

        assert_eq!(config.start_time(), start_time);
    }
}
