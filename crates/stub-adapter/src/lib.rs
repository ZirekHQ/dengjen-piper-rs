use piper_core::domain::errors::PhonemizationError;
use piper_core::ports::phonemizer::{Phonemizer, Sentence};

const KNOWN_VOICES: &[&str] = &["en-US", "en-GB", "fr-FR"];

pub struct StubPhonemizer {
    known_voices: &'static [&'static str],
}

impl StubPhonemizer {
    pub fn new() -> Self {
        Self {
            known_voices: KNOWN_VOICES,
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
        if !self.known_voices.contains(&voice) {
            return Err(PhonemizationError::BackendFailure(format!(
                "unknown voice: {voice}"
            )));
        }
        Ok(split_into_sentences(text))
    }
}

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
