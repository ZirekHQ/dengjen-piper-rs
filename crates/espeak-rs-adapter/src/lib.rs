//! `dengjen-espeak-rs-adapter`: the real, production `Phonemizer`
//! implementation, wrapping `crates/espeak-rs`'s already-tested
//! `text_to_phonemes`. Owns the Approach-A bounded phonemization worker
//! queue (design §5) — see `worker_pool` for the concurrency design and
//! its rationale.

mod worker_pool;

use piper_core::domain::errors::PhonemizationError;
use piper_core::ports::phonemizer::{Phonemizer, Sentence};
use worker_pool::PhonemizerWorkerPool;

/// No production load data exists yet to size this against — a starting
/// point, not a measured value. Revisit once `piper-service` (Phase 3)
/// puts this path under real traffic.
const DEFAULT_QUEUE_CAPACITY: usize = 16;

/// The real espeak-ng-backed `Phonemizer`. Its public surface (this
/// struct's constructor and its one trait method) carries no
/// thread-specific type — `PhonemizerWorkerPool` is a private
/// implementation detail of this crate, so swapping it for Approach B's
/// multi-process pool later requires no change here or in any caller.
pub struct EspeakRsPhonemizer {
    pool: PhonemizerWorkerPool,
}

impl EspeakRsPhonemizer {
    pub fn new(queue_capacity: usize) -> Self {
        Self {
            pool: PhonemizerWorkerPool::new(queue_capacity),
        }
    }
}

impl Default for EspeakRsPhonemizer {
    fn default() -> Self {
        Self::new(DEFAULT_QUEUE_CAPACITY)
    }
}

impl Phonemizer for EspeakRsPhonemizer {
    fn phonemize(&self, text: &str, voice: &str) -> Result<Vec<Sentence>, PhonemizationError> {
        let raw_sentences = self.pool.phonemize(text, voice)?;
        Ok(raw_sentences.into_iter().map(Sentence).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phonemizes_real_text_via_the_real_espeak_ng_backend() {
        let phonemizer = EspeakRsPhonemizer::default();
        let result = phonemizer.phonemize("test", "en-US").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn an_unresolvable_voice_returns_an_error() {
        let phonemizer = EspeakRsPhonemizer::default();
        let result = phonemizer.phonemize("hello", "not-a-real-voice-xyz-123");
        assert!(result.is_err());
    }

    /// Closes AI_NATIVE_SPEC.md's R15 (Medium confidence, no legacy
    /// end-to-end test): a semicolon should render like a comma, since
    /// espeak-ng's terminator signal carries no bit distinguishing them.
    #[test]
    fn semicolons_render_like_commas() {
        let phonemizer = EspeakRsPhonemizer::default();
        let sentences = phonemizer.phonemize("Wait; then go.", "en-US").unwrap();
        let joined: String = sentences
            .into_iter()
            .map(|s| s.0)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains(','),
            "semicolon should render as a comma: {joined:?}"
        );
    }

    /// The fitness test the reimagine design's §5 calls for ("a test
    /// asserts espeak-rs-adapter's public API carries no thread-specific
    /// leakage"): calling `EspeakRsPhonemizer` only through `&dyn
    /// Phonemizer` — the same way any caller outside this crate must —
    /// proves nothing beyond the trait's own signature is required to use
    /// it. `worker_pool`'s types are `pub(crate)`, so this is also a
    /// compile-time guarantee, not just a runtime one.
    #[test]
    fn is_usable_purely_through_the_phonemizer_trait_object() {
        let phonemizer: Box<dyn Phonemizer> = Box::new(EspeakRsPhonemizer::default());
        let result = phonemizer.phonemize("test", "en-US");
        assert!(result.is_ok());
    }
}
