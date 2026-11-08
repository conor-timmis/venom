//! Presentation formatting helpers.
//!
//! Synthetic timestamps are stored as UTC instants. The renderer shows them in
//! the user's local offset when the platform can provide one, falling back to
//! UTC so a headless or restricted environment still renders deterministically.

use time::{OffsetDateTime, UtcOffset};

/// A fixed display offset shared by every row in one prototype run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    offset: UtcOffset,
}

impl Clock {
    #[must_use]
    pub fn local() -> Self {
        Self {
            offset: UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
        }
    }

    #[must_use]
    pub const fn fixed(offset: UtcOffset) -> Self {
        Self { offset }
    }

    #[must_use]
    pub fn format(self, timestamp: OffsetDateTime) -> String {
        let local = timestamp.to_offset(self.offset);
        format!(
            "{:02}:{:02}:{:02}",
            local.hour(),
            local.minute(),
            local.second()
        )
    }
}

/// Groups digits with thousands separators for compact harness labels.
#[must_use]
pub fn count_label(count: u64) -> String {
    let digits = count.to_string();
    let mut label = String::with_capacity(digits.len() + digits.len() / 3);
    let length = digits.len();

    for (index, digit) in digits.chars().enumerate() {
        let remaining = length - index;
        if index > 0 && remaining.is_multiple_of(3) {
            label.push(',');
        }
        label.push(digit);
    }

    label
}

#[cfg(test)]
mod tests {
    use time::{Duration, OffsetDateTime, UtcOffset};

    use super::{Clock, count_label};

    #[test]
    fn local_offset_is_applied_to_the_displayed_time() {
        let utc = OffsetDateTime::UNIX_EPOCH + Duration::hours(1);
        let clock = Clock::fixed(UtcOffset::from_hms(2, 0, 0).expect("valid offset"));

        assert_eq!(clock.format(utc), "03:00:00");
    }

    #[test]
    fn utc_is_used_when_no_offset_is_available() {
        let utc = OffsetDateTime::UNIX_EPOCH + Duration::seconds(61);
        let clock = Clock::fixed(UtcOffset::UTC);

        assert_eq!(clock.format(utc), "00:01:01");
    }

    #[test]
    fn counts_use_thousands_separators() {
        assert_eq!(count_label(0), "0");
        assert_eq!(count_label(999), "999");
        assert_eq!(count_label(1_000), "1,000");
        assert_eq!(count_label(12_345_678), "12,345,678");
    }
}
