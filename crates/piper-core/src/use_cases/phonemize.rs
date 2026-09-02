use std::fmt;

use crate::domain::errors::{PhonemizationError, VoiceLoadError};
use crate::ports::phonemizer::{Phonemizer, Sentence};
use crate::registry::VoiceRegistry;

/// Failure modes for the `Phonemize` use case: a voice lookup miss, or a
/// phonemizer backend failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhonemizeError {
    VoiceNotFound(String),
    Phonemization(PhonemizationError),
}

impl fmt::Display for PhonemizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VoiceNotFound(id) => write!(f, "voice not found: {id}"),
            Self::Phonemization(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PhonemizeError {}

impl From<VoiceLoadError> for PhonemizeError {
    fn from(e: VoiceLoadError) -> Self {
        match e {
            VoiceLoadError::NotFound(id) => Self::VoiceNotFound(id),
            other => Self::VoiceNotFound(other.to_string()),
        }
    }
}

impl From<PhonemizationError> for PhonemizeError {
    fn from(e: PhonemizationError) -> Self {
        Self::Phonemization(e)
    }
}

/// Converts input text into phonemes for a registered voice, via the
/// `Phonemizer` port. This is the direct implementation of the
/// `/v1/voices/{id}/phonemize` capability (AI_NATIVE_SPEC.md C2, C6, C7).
pub struct Phonemize<'a> {
    pub phonemizer: &'a dyn Phonemizer,
}

impl Phonemize<'_> {
    pub fn execute(
        &self,
        registry: &VoiceRegistry,
        voice_id: &str,
        text: &str,
    ) -> Result<Vec<Sentence>, PhonemizeError> {
        let voice = registry.lookup(voice_id)?;
        let sentences = self.phonemizer.phonemize(text, &voice.espeak_voice)?;
        Ok(sentences)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::phoneme::PhonemeIdMap;
    use crate::domain::voice::{AudioConfig, InferenceDefaults, SpeakerMap, Voice};

    struct FakePhonemizer {
        result: Result<Vec<Sentence>, PhonemizationError>,
    }

    impl Phonemizer for FakePhonemizer {
        fn phonemize(&self, _text: &str, _voice: &str) -> Result<Vec<Sentence>, PhonemizationError> {
            self.result.clone()
        }
    }

    fn registry_with_voice(voice_id: &str) -> VoiceRegistry {
        let mut registry = VoiceRegistry::new();
        registry.register(Voice {
            voice_id: voice_id.to_string(),
            audio: AudioConfig { sample_rate: 22050 },
            inference_defaults: InferenceDefaults {
                noise_scale: 0.667,
                length_scale: 1.0,
                noise_w: 0.8,
            },
            num_speakers: 1,
            speaker_map: SpeakerMap::new(),
            phoneme_id_map: PhonemeIdMap::new(),
            espeak_voice: "en-US".to_string(),
        });
        registry
    }

    #[test]
    fn returns_voice_not_found_for_an_unregistered_voice() {
        let phonemizer = FakePhonemizer { result: Ok(vec![]) };
        let use_case = Phonemize { phonemizer: &phonemizer };
        let registry = VoiceRegistry::new();

        let result = use_case.execute(&registry, "missing", "hello");

        assert_eq!(result, Err(PhonemizeError::VoiceNotFound("missing".to_string())));
    }

    #[test]
    fn propagates_a_phonemization_error() {
        let phonemizer = FakePhonemizer { result: Err(PhonemizationError::QueueFull) };
        let use_case = Phonemize { phonemizer: &phonemizer };
        let registry = registry_with_voice("v1");

        let result = use_case.execute(&registry, "v1", "hello");

        assert_eq!(result, Err(PhonemizeError::Phonemization(PhonemizationError::QueueFull)));
    }

    #[test]
    fn returns_the_phonemized_sentences_on_success() {
        let phonemizer = FakePhonemizer {
            result: Ok(vec![Sentence("hɛloʊ.".to_string())]),
        };
        let use_case = Phonemize { phonemizer: &phonemizer };
        let registry = registry_with_voice("v1");

        let result = use_case.execute(&registry, "v1", "hello").unwrap();

        assert_eq!(result, vec![Sentence("hɛloʊ.".to_string())]);
    }
}
