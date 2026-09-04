use crate::domain::errors::PhonemizationError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence(pub String);

pub trait Phonemizer: Send + Sync {
    fn phonemize(&self, text: &str, voice: &str) -> Result<Vec<Sentence>, PhonemizationError>;
}

#[cfg(test)]
mod tests {
    use super::*;

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
