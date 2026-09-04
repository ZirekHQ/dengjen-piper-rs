
#[macro_export]
macro_rules! phonemizer_contract_tests {
    ($make:expr) => {
        #[test]
        fn phonemizing_non_empty_text_returns_at_least_one_sentence() {
            let phonemizer = $make();
            let result = phonemizer.phonemize("hello", "en-US");
            assert!(result.is_ok(), "expected Ok, got {result:?}");
            assert!(
                !result.unwrap().is_empty(),
                "expected at least one sentence"
            );
        }

        #[test]
        fn phonemizing_empty_text_does_not_panic() {
            let phonemizer = $make();
            let _ = phonemizer.phonemize("", "en-US");
        }

        #[test]
        fn an_unresolvable_voice_returns_an_error_not_a_panic() {
            let phonemizer = $make();
            let result = phonemizer.phonemize("hello", "not-a-real-voice-xyz-123");
            assert!(
                result.is_err(),
                "expected an error for an unresolvable voice"
            );
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::domain::errors::PhonemizationError;
    use crate::ports::phonemizer::{Phonemizer, Sentence};

    struct ContractFakePhonemizer;

    impl Phonemizer for ContractFakePhonemizer {
        fn phonemize(&self, text: &str, voice: &str) -> Result<Vec<Sentence>, PhonemizationError> {
            if voice == "not-a-real-voice-xyz-123" {
                return Err(PhonemizationError::BackendFailure(
                    "unknown voice".to_string(),
                ));
            }
            Ok(text
                .split_whitespace()
                .map(|w| Sentence(w.to_string()))
                .collect())
        }
    }

    crate::phonemizer_contract_tests!(|| Box::new(ContractFakePhonemizer) as Box<dyn Phonemizer>);
}
