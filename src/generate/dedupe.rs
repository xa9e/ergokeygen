use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hasher};

use super::DedupeMode;

#[derive(Default)]
pub(crate) struct IdentityHasher(u64);

impl Hasher for IdentityHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0 = fnv1a64(bytes);
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }

    fn write_usize(&mut self, value: usize) {
        self.0 = value as u64;
    }
}

type FastDedupeSet = HashSet<u64, BuildHasherDefault<IdentityHasher>>;

/// Output dedupe filter used only when requested by CLI/options.
///
/// It is intentionally not enabled by default: with the current single-path
/// beam generator and deduplicated charset, duplicate strings should not be
/// produced. The filter exists for custom charsets, future family-based
/// generators, and shell pipelines where a lossy but cheap guard is useful.
pub(crate) enum DedupeFilter {
    Off,
    Exact(HashSet<String>),
    Fast(FastDedupeSet),
}

impl DedupeFilter {
    pub(crate) fn new(mode: DedupeMode) -> Self {
        match mode {
            DedupeMode::Off => Self::Off,
            DedupeMode::Exact => Self::Exact(HashSet::new()),
            DedupeMode::Fast => Self::Fast(FastDedupeSet::default()),
        }
    }

    pub(crate) fn accept(&mut self, text: &str) -> bool {
        match self {
            Self::Off => true,
            Self::Exact(seen) => seen.insert(text.to_string()),
            Self::Fast(seen) => seen.insert(fnv1a64(text.as_bytes())),
        }
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    // Non-cryptographic, stable, fast enough for dedupe fingerprints. This is
    // deliberately not a security hash; a collision in `--dedupe fast` may skip
    // one candidate, which is acceptable for speed-oriented wordlist generation.
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash ^= bytes.len() as u64;
    hash
}
