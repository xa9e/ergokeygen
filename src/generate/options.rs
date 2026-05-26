use crate::charset_by_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupeMode {
    /// Do not deduplicate generated output. This is the default because the
    /// current beam path normally produces unique strings when the charset is
    /// already deduplicated. Keeping it off preserves the fastest hot path.
    Off,
    /// Exact output dedupe with `HashSet<String>`. No false positives, but it
    /// clones every emitted candidate into the set and is therefore mostly for
    /// debugging or future multi-family generators.
    Exact,
    /// Fast approximate output dedupe. Stores a 64-bit FNV-1a fingerprint and
    /// uses an identity hasher for `u64`, so it avoids cryptographic hashing and
    /// large string keys. Collisions are possible but extremely unlikely for
    /// normal wordlist sizes; a collision means a candidate can be skipped.
    Fast,
}

impl DedupeMode {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "off" | "none" | "no" | "false" | "0" => Ok(Self::Off),
            "exact" | "safe" | "lossless" => Ok(Self::Exact),
            "fast" | "hash" | "approx" | "approximate" | "unsafe" | "1" | "true" => Ok(Self::Fast),
            _ => Err(format!("unknown dedupe mode: {raw}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GenerationEngine {
    /// Contract engine for the current Rust model. Keep this path conservative:
    /// deterministic single-thread expansion, no speculative shortcuts.
    Reference,
    /// First production engine. It preserves reference scoring and ordering,
    /// but uses compact states and optional Rayon expansion.
    #[default]
    FastV1,
}

impl GenerationEngine {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "reference" | "ref" | "contract" | "baseline" => Ok(Self::Reference),
            "fast-v1" | "fast1" | "v1" | "current" | "default" => Ok(Self::FastV1),
            _ => Err(format!("unknown generation engine: {raw}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenerateOptions {
    pub min_len: usize,
    pub max_len: usize,
    pub limit: usize,
    pub beam: usize,
    pub charset: Vec<char>,
    pub max_avg_cost: Option<f64>,
    pub max_total_cost: Option<f64>,
    pub dedupe: DedupeMode,
    pub engine: GenerationEngine,
    /// Parallelize beam expansion with Rayon. This preserves the same scoring
    /// and final ordering as the reference single-threaded path because all
    /// candidates are still merged and selected by one deterministic comparator.
    pub parallel: bool,
    /// Avoid Rayon overhead for tiny frontiers. The first 1-2 depths are usually
    /// cheaper to expand sequentially; parallelism starts once work is large.
    pub parallel_threshold: usize,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            min_len: 4,
            max_len: 8,
            limit: usize::MAX,
            beam: 4096,
            charset: charset_by_name("lower")
                .unwrap_or_else(|_| "abcdefghijklmnopqrstuvwxyz".chars().collect()),
            max_avg_cost: None,
            max_total_cost: None,
            dedupe: DedupeMode::Off,
            engine: GenerationEngine::default(),
            parallel: true,
            parallel_threshold: 8192,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedSequence {
    pub text: String,
    pub total: f64,
    pub average: f64,
}
