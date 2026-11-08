use crate::{ChannelId, Platform};

/// A chat channel on a specific platform.
///
/// `ChannelId` values are platform-scoped, so the same channel slug on two
/// platforms represents two distinct channels.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Channel {
    pub id: ChannelId,
    pub platform: Platform,
    pub display_name: String,
}

impl Channel {
    pub fn new(platform: Platform, id: impl Into<String>, display_name: impl Into<String>) -> Self {
        let display_name = display_name.into();
        let display_name = display_name.trim();
        debug_assert!(
            !display_name.is_empty(),
            "channel display name must not be empty"
        );

        Self {
            id: ChannelId::new(id),
            platform,
            display_name: display_name.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Channel;
    use crate::Platform;

    #[test]
    fn channel_keeps_platform_scope() {
        let channel = Channel::new(Platform::Twitch, " venom_dev ", " Venom Dev ");
        assert_eq!(channel.id.as_str(), "venom_dev");
        assert_eq!(channel.display_name, "Venom Dev");
        assert_eq!(channel.platform, Platform::Twitch);
    }
}
