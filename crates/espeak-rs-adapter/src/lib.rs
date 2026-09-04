
mod worker_pool;

use piper_core::domain::errors::PhonemizationError;
use piper_core::ports::phonemizer::{Phonemizer, Sentence};
use worker_pool::PhonemizerWorkerPool;

const DEFAULT_QUEUE_CAPACITY: usize = 16;

pub struct EspeakRsPhonemizer {
    pool: PhonemizerWorkerPool,
}

impl EspeakRsPhonemizer {
    pub fn new(queue_capacity: usize) -> Self {
        Self {
            pool: PhonemizerWorkerPool::new(queue_capacity.max(1)),
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
    fn a_capacity_of_zero_is_clamped_so_a_single_call_still_succeeds() {
        let phonemizer = EspeakRsPhonemizer::new(0);
        let result = phonemizer.phonemize("test", "en-US");
        assert!(result.is_ok());
    }

    #[test]
    fn an_unresolvable_voice_returns_an_error() {
        let phonemizer = EspeakRsPhonemizer::default();
        let result = phonemizer.phonemize("hello", "not-a-real-voice-xyz-123");
        assert!(result.is_err());
    }

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

    #[test]
    fn is_usable_purely_through_the_phonemizer_trait_object() {
        let phonemizer: Box<dyn Phonemizer> = Box::new(EspeakRsPhonemizer::default());
        let result = phonemizer.phonemize("test", "en-US");
        assert!(result.is_ok());
    }
}
