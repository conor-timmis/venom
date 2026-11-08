use std::fmt;

/// A supported livestream platform.
///
/// The set is intentionally small for Phase 0. Adding a platform should be a
/// deliberate contract change rather than an implicit string value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Platform {
    Twitch,
    YouTube,
    Kick,
}

impl Platform {
    pub const ALL: [Self; 3] = [Self::Twitch, Self::YouTube, Self::Kick];

    /// Stable lowercase identifier used in internal keys and synthetic IDs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Twitch => "twitch",
            Self::YouTube => "youtube",
            Self::Kick => "kick",
        }
    }

    /// Human-readable label suitable for UI chrome.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Twitch => "Twitch",
            Self::YouTube => "YouTube",
            Self::Kick => "Kick",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::Platform;

    #[test]
    fn stable_identifiers_are_unique() {
        let identifiers: HashSet<_> = Platform::ALL
            .iter()
            .map(|platform| platform.as_str())
            .collect();
        assert_eq!(identifiers.len(), Platform::ALL.len());
    }

    #[test]
    fn display_names_are_human_readable() {
        assert_eq!(Platform::Twitch.to_string(), "Twitch");
        assert_eq!(Platform::YouTube.to_string(), "YouTube");
        assert_eq!(Platform::Kick.to_string(), "Kick");
    }
}
