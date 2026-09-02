use crate::domain::errors::PhonemizationError;

/// One phonemized sentence: joined clause text with reconstructed
/// terminator punctuation. The output unit of a `Phonemizer` port call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence(pub String);

/// Converts input text into phonemes for a named voice. Implemented by
/// adapter crates (e.g. `espeak-rs-adapter`) — this trait carries no
/// backend-specific detail (no espeak-ng terminator bit layout, no FFI
/// types), by design: any leakage here would defeat the point of the port.
pub trait Phonemizer: Send + Sync {
    fn phonemize(&self, text: &str, voice: &str) -> Result<Vec<Sentence>, PhonemizationError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal in-module fake proving the trait is object-safe and usable
    /// by a caller — not the full port contract (see `crate::testing`,
    /// Task 13), and not the `stub-adapter` crate (Phase 2), which is a
    /// standalone crate other adapters are contract-tested against.
    struct EchoPhonemizer;

    impl Phonemizer for EchoPhonemizer {
        fn phonemize(&self, text: &str, _voice: &str) -> Result<Vec<Sentence>, PhonemizationError> {
            Ok(vec![Sentence(text.to_string())])
        }
    }

    #[test]
    fn trait_object_can_be_called_through_a_dyn_reference() {
        let phonemizer: &dyn Phonemizer = &EchoPhonemizer;
        let result = phonemizer.phonemize("hello", "en-US").unwrap();
        assert_eq!(result, vec![Sentence("hello".to_string())]);
    }
}
