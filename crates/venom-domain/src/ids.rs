//! Opaque, platform-scoped identifiers.
//!
//! Callers must only compare identifiers that originate from the same
//! platform. The types intentionally do not expose the raw string as mutable
//! state; build a new value instead.

macro_rules! define_id_type {
    ($name:ident, $description:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// Creates an identifier, trimming surrounding whitespace.
            ///
            /// Identifiers are expected to be non-empty. Empty values indicate
            /// a programming error at the adapter boundary and panic in debug
            /// builds so faults surface during development.
            pub fn new(value: impl Into<String>) -> Self {
                let value = value.into();
                let trimmed = value.trim();
                debug_assert!(
                    !trimmed.is_empty(),
                    concat!($description, " must not be empty")
                );
                Self(trimmed.to_owned())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

define_id_type!(MessageId, "message id");
define_id_type!(ChannelId, "channel id");
define_id_type!(UserId, "user id");

#[cfg(test)]
mod tests {
    use super::{MessageId, UserId};

    #[test]
    fn identifier_trims_surrounding_whitespace() {
        assert_eq!(MessageId::new("  message-1 \n").as_str(), "message-1");
    }

    #[test]
    fn identifiers_render_without_decoration() {
        assert_eq!(UserId::new("twitch:user-1").to_string(), "twitch:user-1");
    }
}
