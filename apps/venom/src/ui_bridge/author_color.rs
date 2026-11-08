//! Deterministic author colours derived from display names.
//!
//! Colour selection is presentation, not domain data, so it lives in the UI
//! bridge. Hashing is deliberately stable: the same author must keep the same
//! colour across messages and sessions.

use slint::Color;

/// Colours chosen to stay readable on Venom's dark prototype theme.
const AUTHOR_PALETTE: [(u8, u8, u8); 12] = [
    (0xff, 0x6b, 0x6b),
    (0xff, 0xd1, 0x66),
    (0x7b, 0xd8, 0x8f),
    (0x61, 0xd4, 0xff),
    (0xb3, 0x88, 0xff),
    (0xff, 0x9f, 0x68),
    (0xf7, 0x8f, 0xb3),
    (0x9e, 0xf0, 0xf0),
    (0xe0, 0xb0, 0xff),
    (0xa8, 0xe6, 0xcf),
    (0xff, 0xcc, 0xf9),
    (0xc3, 0xe8, 0x8d),
];

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Returns the stable colour for a display name.
#[must_use]
pub fn for_name(name: &str) -> Color {
    let (red, green, blue) =
        AUTHOR_PALETTE[(stable_hash(name) % AUTHOR_PALETTE.len() as u64) as usize];
    Color::from_rgb_u8(red, green, blue)
}

/// FNV-1a keeps colours stable without Rust's randomly seeded hasher.
fn stable_hash(value: &str) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_name_always_gets_the_same_colour() {
        assert_eq!(for_name("moonlit_mara"), for_name("moonlit_mara"));
    }

    #[test]
    fn palette_covers_more_than_one_colour() {
        let colours = ["moonlit_mara", "pixelpilot", "GG_Bandit", "NovaDrift"].map(for_name);

        assert!(colours.iter().any(|colour| *colour != colours[0]));
    }

    #[test]
    fn hash_is_stable_for_empty_and_ascii_names() {
        assert_eq!(stable_hash(""), FNV_OFFSET_BASIS);
        assert_ne!(stable_hash("a"), stable_hash("b"));
    }
}
