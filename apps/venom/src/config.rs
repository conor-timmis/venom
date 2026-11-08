//! Prototype harness configuration.
//!
//! Values come from environment variables so a run can be reproduced without a
//! rebuild. Arguments are limited to the headless switch; the checklist harness
//! does not need a general command-line parser yet.

use std::{error::Error, ffi::OsString, fmt, time::Duration};

use venom_core::DEFAULT_SEED;

pub const DEFAULT_INITIAL_MESSAGES: usize = 10_000;
pub const DEFAULT_RETAINED_MESSAGES: usize = 10_000;
pub const DEFAULT_LIVE_RATE: u32 = 60;
pub const MAX_INITIAL_MESSAGES: usize = 50_000;
pub const MAX_RETAINED_MESSAGES: usize = 50_000;
pub const MAX_BURST_MESSAGES: usize = 5_000;

/// The UI timer and the feed share this tick rate (50 ms).
pub const LIVE_TICK_HZ: u32 = 20;
pub const ALLOWED_LIVE_RATES: [u32; 4] = [0, 30, 60, 120];

const ENV_SEED: &str = "VENOM_PROTOTYPE_SEED";
const ENV_PREFILL: &str = "VENOM_PROTOTYPE_PREFILL";
const ENV_RETAINED: &str = "VENOM_PROTOTYPE_RETAINED";
const ENV_RATE: &str = "VENOM_PROTOTYPE_RATE";
const ENV_EXIT_AFTER_MS: &str = "VENOM_PROTOTYPE_EXIT_AFTER_MS";
const HEADLESS_FLAG: &str = "--headless";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarnessConfig {
    /// Runs the feed without creating a window; useful in CI.
    pub headless: bool,
    pub initial_messages: usize,
    pub retained_messages: usize,
    pub seed: u64,
    pub live_rate: u32,
    /// Optional smoke-test escape hatch so CI or a terminal run can close the
    /// window after the prototype has painted. In headless mode it bounds the
    /// simulated live phase.
    pub exit_after: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    UnknownArgument(String),
    InvalidValue {
        variable: &'static str,
        value: String,
        expected: &'static str,
    },
    UnsupportedRate(u32),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownArgument(argument) => {
                write!(formatter, "unknown argument '{argument}' (try --headless)")
            }
            Self::InvalidValue {
                variable,
                value,
                expected,
            } => write!(
                formatter,
                "invalid value '{value}' for {variable}: {expected}"
            ),
            Self::UnsupportedRate(rate) => {
                write!(
                    formatter,
                    "unsupported live rate {rate}; choose 0, 30, 60, or 120"
                )
            }
        }
    }
}

impl Error for ConfigError {}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            headless: false,
            initial_messages: DEFAULT_INITIAL_MESSAGES,
            retained_messages: DEFAULT_RETAINED_MESSAGES,
            seed: DEFAULT_SEED,
            live_rate: DEFAULT_LIVE_RATE,
            exit_after: None,
        }
    }
}

impl HarnessConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::resolve(std::env::args_os().skip(1), |name| std::env::var(name).ok())
    }

    fn resolve(
        args: impl IntoIterator<Item = OsString>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let headless = parse_args(args)?;
        let retained_messages = match env_usize(env(ENV_RETAINED), ENV_RETAINED)? {
            Some(0) => {
                return Err(invalid_value(
                    ENV_RETAINED,
                    "0",
                    "expected a positive integer",
                ));
            }
            Some(value) => value.min(MAX_RETAINED_MESSAGES),
            None => DEFAULT_RETAINED_MESSAGES,
        };
        let initial_messages = env_usize(env(ENV_PREFILL), ENV_PREFILL)?
            .unwrap_or(DEFAULT_INITIAL_MESSAGES)
            .min(MAX_INITIAL_MESSAGES)
            .min(retained_messages);
        let seed = env_u64(env(ENV_SEED), ENV_SEED)?.unwrap_or(DEFAULT_SEED);
        let live_rate = match env_u64(env(ENV_RATE), ENV_RATE)? {
            Some(rate) => {
                let rate = u32::try_from(rate).unwrap_or(u32::MAX);
                if !ALLOWED_LIVE_RATES.contains(&rate) {
                    return Err(ConfigError::UnsupportedRate(rate));
                }
                rate
            }
            None => DEFAULT_LIVE_RATE,
        };
        let exit_after = env_u64(env(ENV_EXIT_AFTER_MS), ENV_EXIT_AFTER_MS)?
            .map(Duration::from_millis)
            .filter(|duration| !duration.is_zero());

        Ok(Self {
            headless,
            initial_messages,
            retained_messages,
            seed,
            live_rate,
            exit_after,
        })
    }
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<bool, ConfigError> {
    let mut headless = false;

    for argument in args {
        let argument = argument
            .into_string()
            .map_err(|_| ConfigError::UnknownArgument("<non-unicode>".to_owned()))?;

        if argument == HEADLESS_FLAG {
            headless = true;
        } else {
            return Err(ConfigError::UnknownArgument(argument));
        }
    }

    Ok(headless)
}

fn env_usize(value: Option<String>, variable: &'static str) -> Result<Option<usize>, ConfigError> {
    match value {
        Some(value) => {
            let trimmed = value.trim();
            let parsed = trimmed
                .parse()
                .map_err(|_| invalid_value(variable, trimmed, "expected a non-negative integer"))?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

fn env_u64(value: Option<String>, variable: &'static str) -> Result<Option<u64>, ConfigError> {
    match value {
        Some(value) => {
            let trimmed = value.trim();
            let parsed = trimmed
                .parse()
                .map_err(|_| invalid_value(variable, trimmed, "expected a non-negative integer"))?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

fn invalid_value(variable: &'static str, value: &str, expected: &'static str) -> ConfigError {
    ConfigError::InvalidValue {
        variable,
        value: value.to_owned(),
        expected,
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, ffi::OsString, time::Duration};

    use super::{
        ALLOWED_LIVE_RATES, ConfigError, DEFAULT_INITIAL_MESSAGES, DEFAULT_LIVE_RATE,
        DEFAULT_RETAINED_MESSAGES, ENV_EXIT_AFTER_MS, ENV_PREFILL, ENV_RATE, ENV_RETAINED,
        ENV_SEED, HarnessConfig,
    };
    use venom_core::DEFAULT_SEED;

    fn resolve(args: &[&str], variables: &[(&str, &str)]) -> Result<HarnessConfig, ConfigError> {
        let args = args.iter().map(OsString::from).collect::<Vec<_>>();
        let variables = variables
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect::<HashMap<_, _>>();

        HarnessConfig::resolve(args, |name| variables.get(name).cloned())
    }

    #[test]
    fn defaults_match_the_sprint_two_workload() {
        let config = resolve(&[], &[]).expect("defaults are valid");

        assert!(!config.headless);
        assert_eq!(config.initial_messages, DEFAULT_INITIAL_MESSAGES);
        assert_eq!(config.retained_messages, DEFAULT_RETAINED_MESSAGES);
        assert_eq!(config.seed, DEFAULT_SEED);
        assert_eq!(config.live_rate, DEFAULT_LIVE_RATE);
        assert_eq!(config.exit_after, None);
    }

    #[test]
    fn headless_flag_is_recognised() {
        let config = resolve(&["--headless"], &[]).expect("flag is valid");

        assert!(config.headless);
    }

    #[test]
    fn unknown_arguments_are_rejected() {
        assert_eq!(
            resolve(&["--fullscreen"], &[]).unwrap_err(),
            ConfigError::UnknownArgument("--fullscreen".to_owned())
        );
    }

    #[test]
    fn environment_overrides_are_parsed() {
        let config = resolve(
            &[],
            &[
                (ENV_SEED, "42"),
                (ENV_PREFILL, " 250 "),
                (ENV_RETAINED, "1000"),
                (ENV_RATE, "120"),
                (ENV_EXIT_AFTER_MS, "2500"),
            ],
        )
        .expect("values are valid");

        assert_eq!(config.seed, 42);
        assert_eq!(config.initial_messages, 250);
        assert_eq!(config.retained_messages, 1_000);
        assert_eq!(config.live_rate, 120);
        assert_eq!(config.exit_after, Some(Duration::from_millis(2_500)));
    }

    #[test]
    fn prefill_is_clamped_to_retention() {
        let config = resolve(&[], &[(ENV_PREFILL, "10000"), (ENV_RETAINED, "500")])
            .expect("values are valid");

        assert_eq!(config.retained_messages, 500);
        assert_eq!(config.initial_messages, 500);
    }

    #[test]
    fn zero_retention_is_rejected() {
        assert!(matches!(
            resolve(&[], &[(ENV_RETAINED, "0")]),
            Err(ConfigError::InvalidValue { .. })
        ));
    }

    #[test]
    fn unsupported_rates_are_rejected() {
        assert_eq!(
            resolve(&[], &[(ENV_RATE, "45")]).unwrap_err(),
            ConfigError::UnsupportedRate(45)
        );
        assert!(ALLOWED_LIVE_RATES.contains(&30));
    }

    #[test]
    fn malformed_numbers_are_rejected() {
        assert!(matches!(
            resolve(&[], &[(ENV_SEED, "one")]),
            Err(ConfigError::InvalidValue { .. })
        ));
        assert!(matches!(
            resolve(&[], &[(ENV_EXIT_AFTER_MS, "1.5")]),
            Err(ConfigError::InvalidValue { .. })
        ));
    }

    #[test]
    fn zero_exit_after_means_run_until_closed() {
        let config = resolve(&[], &[(ENV_EXIT_AFTER_MS, "0")]).expect("zero is valid");

        assert_eq!(config.exit_after, None);
    }
}
