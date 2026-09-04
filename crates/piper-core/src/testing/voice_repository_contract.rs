
#[macro_export]
macro_rules! voice_repository_contract_tests {
    ($make:expr, $known_voice_id:expr) => {
        #[test]
        fn loading_an_unknown_voice_id_returns_not_found() {
            let repository = $make();
            let result = repository.load("definitely-not-a-real-voice-id-xyz");
            assert!(
                matches!(
                    result,
                    Err($crate::domain::errors::VoiceLoadError::NotFound(_))
                ),
                "expected NotFound, got {result:?}"
            );
        }

        #[test]
        fn loading_a_known_voice_id_returns_a_voice_with_a_matching_id() {
            let repository = $make();
            let voice = repository
                .load($known_voice_id)
                .expect("known voice should load");
            assert_eq!(voice.voice_id, $known_voice_id);
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::domain::errors::VoiceLoadError;
    use crate::domain::phoneme::PhonemeIdMap;
    use crate::domain::voice::{AudioConfig, InferenceDefaults, SpeakerMap, Voice};
    use crate::ports::voice_repository::VoiceRepository;

    struct ContractFakeRepository;

    impl VoiceRepository for ContractFakeRepository {
        fn load(&self, voice_id: &str) -> Result<Voice, VoiceLoadError> {
            if voice_id == "known" {
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

    crate::voice_repository_contract_tests!(|| ContractFakeRepository, "known");
}
