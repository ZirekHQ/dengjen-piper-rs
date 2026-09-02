use crate::domain::errors::VoiceLoadError;
use crate::domain::voice::Voice;

/// Loads a `Voice` given its id. Implemented by adapter crates (e.g.
/// `fs-voice-repo`, which loads from a model+config path pair) — this
/// trait carries no filesystem or JSON-schema detail.
pub trait VoiceRepository: Send + Sync {
    fn load(&self, voice_id: &str) -> Result<Voice, VoiceLoadError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::phoneme::PhonemeIdMap;
    use crate::domain::voice::{AudioConfig, InferenceDefaults, SpeakerMap};

    struct FixedVoiceRepository;

    impl VoiceRepository for FixedVoiceRepository {
        fn load(&self, voice_id: &str) -> Result<Voice, VoiceLoadError> {
            if voice_id == "known-voice" {
                Ok(Voice {
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
                })
            } else {
                Err(VoiceLoadError::NotFound(voice_id.to_string()))
            }
        }
    }

    #[test]
    fn trait_object_returns_not_found_for_an_unknown_voice() {
        let repo: &dyn VoiceRepository = &FixedVoiceRepository;
        let result = repo.load("unknown-voice");
        assert_eq!(result, Err(VoiceLoadError::NotFound("unknown-voice".to_string())));
    }

    #[test]
    fn trait_object_returns_the_voice_for_a_known_id() {
        let repo: &dyn VoiceRepository = &FixedVoiceRepository;
        let result = repo.load("known-voice").unwrap();
        assert_eq!(result.voice_id, "known-voice");
    }
}
