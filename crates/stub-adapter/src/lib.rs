//! `dengjen-stub-adapter`: a deterministic, dependency-free `Phonemizer`
//! implementation. Deliberately different from `espeak-rs-adapter`'s real
//! backend (whitespace/terminator splitting instead of clause boundaries) —
//! this crate's own test suite proves it satisfies `piper-core`'s
//! `Phonemizer` contract from *outside* that crate, the actual
//! stub-validated-trait proof point the reimagine design calls for.

use piper_core::domain::errors::PhonemizationError;
use piper_core::ports::phonemizer::{Phonemizer, Sentence};

/// Voices this stub recognizes; anything else is treated as unresolvable,
/// mirroring R22's "an unresolvable voice fails before processing begins."
const KNOWN_VOICES: &[&str] = &["en-US", "en-GB", "fr-FR"];

/// A deterministic `Phonemizer` with no native dependency — for tests of
/// other crates that need a `Phonemizer` without pulling in real espeak-ng,
/// and as the first external proof that `piper-core`'s `Phonemizer`
/// contract is genuinely implementable from outside `piper-core` itself.
pub struct StubPhonemizer {
    known_voices: Vec<String>,
}

impl StubPhonemizer {
    pub fn new() -> Self {
        Self {
            known_voices: KNOWN_VOICES.iter().copied().map(str::to_string).collect(),
        }
    }
}

impl Default for StubPhonemizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Phonemizer for StubPhonemizer {
    fn phonemize(&self, text: &str, voice: &str) -> Result<Vec<Sentence>, PhonemizationError> {
        if !self.known_voices.iter().any(|v| v == voice) {
            return Err(PhonemizationError::BackendFailure(format!(
                "unknown voice: {voice}"
            )));
        }
        Ok(split_into_sentences(text))
    }
}

/// Splits on `.`, `?`, `!` (terminator kept on the preceding sentence),
/// trims each segment, and drops empty ones. Text with no terminator
/// becomes a single sentence.
fn split_into_sentences(text: &str) -> Vec<Sentence> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '?' | '!') {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                sentences.push(Sentence(trimmed.to_string()));
            }
            current.clear();
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        sentences.push(Sentence(trimmed.to_string()));
    }
    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_multiple_terminated_sentences() {
        let phonemizer = StubPhonemizer::new();
        let result = phonemizer
            .phonemize("Hello world. How are you?", "en-US")
            .unwrap();
        assert_eq!(
            result,
            vec![
                Sentence("Hello world.".to_string()),
                Sentence("How are you?".to_string()),
            ]
        );
    }

    #[test]
    fn text_with_no_terminator_becomes_one_sentence() {
        let phonemizer = StubPhonemizer::new();
        let result = phonemizer.phonemize("no terminator here", "en-US").unwrap();
        assert_eq!(result, vec![Sentence("no terminator here".to_string())]);
    }

    #[test]
    fn empty_text_returns_no_sentences() {
        let phonemizer = StubPhonemizer::new();
        let result = phonemizer.phonemize("", "en-US").unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn trailing_whitespace_after_the_last_terminator_is_dropped() {
        let phonemizer = StubPhonemizer::new();
        let result = phonemizer.phonemize("One. Two. ", "en-US").unwrap();
        assert_eq!(
            result,
            vec![Sentence("One.".to_string()), Sentence("Two.".to_string())]
        );
    }

    #[test]
    fn an_unknown_voice_returns_a_backend_failure_error() {
        let phonemizer = StubPhonemizer::new();
        let result = phonemizer.phonemize("hello", "not-a-real-voice");
        assert!(matches!(result, Err(PhonemizationError::BackendFailure(_))));
    }
}

#[cfg(test)]
mod contract {
    use super::*;

    piper_core::phonemizer_contract_tests!(
        || Box::new(StubPhonemizer::new()) as Box<dyn Phonemizer>
    );
}
