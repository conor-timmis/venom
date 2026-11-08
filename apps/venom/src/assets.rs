//! Bounded in-memory cache for Phase 0 placeholder emote assets.
//!
//! Real providers eventually supply decoded frames through the asset manager
//! described in MAIN.md §11. Until then this module synthesises tiny RGBA
//! frames deterministically from the cache key, so the renderer exercises the
//! same lookup, sharing, and eviction path with no downloaded assets.

use std::{collections::HashMap, hash::Hash, rc::Rc};

#[cfg(test)]
use slint::Model;
use slint::{Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};
use venom_domain::EmoteProvider;

/// Displayed size of a placeholder emote, in logical pixels.
pub const EMOTE_SIZE_PX: u32 = 20;

/// Number of frames generated for an animated placeholder.
pub const ANIMATED_FRAME_COUNT: usize = 4;

/// Delay between animated placeholder frames.
pub const FRAME_DELAY_MS: u16 = 140;

/// Default bound for decoded emote assets shared by every panel.
pub const DEFAULT_CACHE_CAPACITY: usize = 256;

const PALETTE: [(u8, u8, u8); 8] = [
    (0x9d, 0x6b, 0xff),
    (0x53, 0xfc, 0x18),
    (0xff, 0x4d, 0x4d),
    (0x1f, 0x8e, 0xfa),
    (0xff, 0xc4, 0x3d),
    (0xec, 0x4c, 0xb3),
    (0x00, 0xc2, 0xa8),
    (0xff, 0x8a, 0x3d),
];

const ANIMATED_NAMES: [&str; 4] = ["catJAM", "DanceTime", "PartyParrot", "Hype"];
const ANIMATED_DOT_OFFSETS: [(u32, u32); ANIMATED_FRAME_COUNT] =
    [(10, 3), (16, 10), (10, 16), (4, 10)];

/// Identity of an emote asset.
///
/// Provider is part of the key because emote names are not globally unique
/// (MAIN.md §7.7).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmoteKey {
    provider: EmoteProvider,
    id: String,
}

impl EmoteKey {
    pub fn new(provider: EmoteProvider, id: impl Into<String>) -> Self {
        Self {
            provider,
            id: id.into(),
        }
    }

    #[must_use]
    pub const fn provider(&self) -> EmoteProvider {
        self.provider
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        self.id.rsplit(':').next().unwrap_or(&self.id)
    }
}

/// A decoded emote plus its frame timing.
#[derive(Clone)]
pub struct EmoteAsset {
    frames: ModelRc<Image>,
    frame_delay_ms: u16,
}

impl EmoteAsset {
    #[must_use]
    pub fn frames(&self) -> ModelRc<Image> {
        self.frames.clone()
    }

    #[must_use]
    pub const fn frame_delay_ms(&self) -> u16 {
        self.frame_delay_ms
    }

    #[cfg(test)]
    #[must_use]
    pub fn is_animated(&self) -> bool {
        self.frames.row_count() > 1
    }
}

/// Counters surfaced by the prototype status bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetStats {
    pub entries: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
}

/// Least-recently-used cache with a hard entry bound.
pub struct LruCache<K, V> {
    capacity: usize,
    entries: HashMap<K, CacheEntry<V>>,
    clock: u64,
}

struct CacheEntry<V> {
    value: V,
    last_used: u64,
}

impl<K: Eq + Hash + Clone, V> LruCache<K, V> {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "asset cache capacity must be positive");
        Self {
            capacity,
            entries: HashMap::with_capacity(capacity),
            clock: 0,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.clock += 1;
        let entry = self.entries.get_mut(key)?;
        entry.last_used = self.clock;
        Some(&entry.value)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.clock += 1;

        if !self.entries.contains_key(&key) && self.entries.len() >= self.capacity {
            self.evict_least_recently_used();
        }

        self.entries.insert(
            key,
            CacheEntry {
                value,
                last_used: self.clock,
            },
        );
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    fn evict_least_recently_used(&mut self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| key.clone());

        if let Some(key) = oldest {
            self.entries.remove(&key);
        }
    }
}

/// Assets shared by every rendered row.
pub struct AssetStore {
    cache: LruCache<EmoteKey, Rc<EmoteAsset>>,
    hits: u64,
    misses: u64,
}

impl AssetStore {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(capacity),
            hits: 0,
            misses: 0,
        }
    }

    /// Returns the cached asset, generating a placeholder on a miss.
    pub fn emote(&mut self, provider: EmoteProvider, id: &str) -> Rc<EmoteAsset> {
        let key = EmoteKey::new(provider, id);

        if let Some(asset) = self.cache.get(&key) {
            self.hits += 1;
            return Rc::clone(asset);
        }

        self.misses += 1;
        let asset = Rc::new(placeholder_asset(&key));
        self.cache.insert(key, Rc::clone(&asset));
        asset
    }

    #[must_use]
    pub fn stats(&self) -> AssetStats {
        AssetStats {
            entries: self.cache.len(),
            capacity: self.cache.capacity(),
            hits: self.hits,
            misses: self.misses,
        }
    }
}

fn placeholder_asset(key: &EmoteKey) -> EmoteAsset {
    let palette = palette_color(key);
    let frames = if is_animated(key) {
        (0..ANIMATED_FRAME_COUNT)
            .map(|index| placeholder_frame(palette, Some(index)))
            .collect()
    } else {
        vec![placeholder_frame(palette, None)]
    };

    EmoteAsset {
        frames: ModelRc::new(VecModel::from(frames)),
        frame_delay_ms: FRAME_DELAY_MS,
    }
}

fn is_animated(key: &EmoteKey) -> bool {
    ANIMATED_NAMES
        .iter()
        .any(|name| name.eq_ignore_ascii_case(key.name()))
}

fn palette_color(key: &EmoteKey) -> (u8, u8, u8) {
    let mut hash = 2_166_136_261_u32;
    for byte in key.provider().as_str().bytes().chain(key.id().bytes()) {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }

    PALETTE[(hash as usize) % PALETTE.len()]
}

fn placeholder_frame((red, green, blue): (u8, u8, u8), animation: Option<usize>) -> Image {
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(EMOTE_SIZE_PX, EMOTE_SIZE_PX);
    let center = EMOTE_SIZE_PX / 2;
    let radius_squared = center * center;
    let pixels = buffer.make_mut_slice();

    for y in 0..EMOTE_SIZE_PX {
        for x in 0..EMOTE_SIZE_PX {
            let dx = x.abs_diff(center);
            let dy = y.abs_diff(center);

            if dx * dx + dy * dy > radius_squared {
                continue;
            }

            let (red, green, blue) = match animation {
                Some(index) if is_animation_dot(x, y, index) => (255, 255, 255),
                _ => (red, green, blue),
            };

            pixels[pixel_index(x, y)] = Rgba8Pixel {
                r: red,
                g: green,
                b: blue,
                a: 255,
            };
        }
    }

    Image::from_rgba8(buffer)
}

/// Draws a moving 3x3 highlight so animated placeholders are visibly distinct.
fn is_animation_dot(x: u32, y: u32, frame_index: usize) -> bool {
    let (dot_x, dot_y) = ANIMATED_DOT_OFFSETS[frame_index % ANIMATED_FRAME_COUNT];

    x.abs_diff(dot_x) <= 1 && y.abs_diff(dot_y) <= 1
}

fn pixel_index(x: u32, y: u32) -> usize {
    (y * EMOTE_SIZE_PX + x) as usize
}

#[cfg(test)]
mod tests {
    use slint::Model;
    use venom_domain::EmoteProvider;

    use super::{
        ANIMATED_FRAME_COUNT, AssetStore, DEFAULT_CACHE_CAPACITY, EmoteKey, LruCache,
        placeholder_asset,
    };

    #[test]
    fn cache_returns_values_and_counts_entries() {
        let mut cache = LruCache::new(2);
        cache.insert("a", 1);
        cache.insert("b", 2);

        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.capacity(), 2);
    }

    #[test]
    fn inserting_past_capacity_evicts_the_least_recently_used_entry() {
        let mut cache = LruCache::new(2);
        cache.insert("a", 1);
        cache.insert("b", 2);
        assert_eq!(cache.get(&"a"), Some(&1));

        cache.insert("c", 3);

        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"c"), Some(&3));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn replacing_a_key_does_not_evict_another_entry() {
        let mut cache = LruCache::new(2);
        cache.insert("a", 1);
        cache.insert("b", 2);

        cache.insert("a", 10);

        assert_eq!(cache.get(&"a"), Some(&10));
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn every_read_counts_as_a_hit() {
        let mut store = AssetStore::new(DEFAULT_CACHE_CAPACITY);
        store.emote(EmoteProvider::Twitch, "twitch:Kappa");
        store.emote(EmoteProvider::Twitch, "twitch:Kappa");
        store.emote(EmoteProvider::Twitch, "twitch:Kappa");

        let stats = store.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.entries, 1);
    }

    #[test]
    fn provider_is_part_of_the_cache_key() {
        let mut store = AssetStore::new(DEFAULT_CACHE_CAPACITY);
        let twitch = store.emote(EmoteProvider::Twitch, "Kappa");
        let seven_tv = store.emote(EmoteProvider::SevenTv, "Kappa");

        assert!(!std::rc::Rc::ptr_eq(&twitch, &seven_tv));
        assert_eq!(store.stats().entries, 2);
    }

    #[test]
    fn repeated_lookups_return_the_same_shared_asset() {
        let mut store = AssetStore::new(DEFAULT_CACHE_CAPACITY);
        let first = store.emote(EmoteProvider::Twitch, "twitch:Kappa");
        let second = store.emote(EmoteProvider::Twitch, "twitch:Kappa");

        assert!(std::rc::Rc::ptr_eq(&first, &second));
    }

    #[test]
    fn entries_never_exceed_the_configured_bound() {
        let mut store = AssetStore::new(2);
        for index in 0..10 {
            store.emote(EmoteProvider::Kick, &format!("kick:emote-{index}"));
        }

        let stats = store.stats();
        assert_eq!(stats.entries, 2);
        assert_eq!(stats.capacity, 2);
        assert_eq!(stats.misses, 10);
    }

    #[test]
    fn placeholder_static_emote_has_one_frame() {
        let key = EmoteKey::new(EmoteProvider::Twitch, "twitch:Kappa");
        let asset = placeholder_asset(&key);

        assert_eq!(asset.frames().row_count(), 1);
        assert!(!asset.is_animated());
    }

    #[test]
    fn placeholder_animated_emote_has_a_frame_sequence() {
        let key = EmoteKey::new(EmoteProvider::SevenTv, "7tv:catJAM");
        let asset = placeholder_asset(&key);

        assert_eq!(asset.frames().row_count(), ANIMATED_FRAME_COUNT);
        assert!(asset.is_animated());
    }

    #[test]
    fn placeholders_are_deterministic_for_a_key() {
        let key = EmoteKey::new(EmoteProvider::BetterTv, "bttv:PartyParrot");
        let first = placeholder_asset(&key);
        let second = placeholder_asset(&key);

        assert_eq!(first.frames().row_count(), second.frames().row_count());
        assert_eq!(first.frame_delay_ms(), second.frame_delay_ms());
    }
}
