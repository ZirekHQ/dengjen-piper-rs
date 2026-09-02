use crate::domain::errors::VoiceLoadError;
use crate::ports::inference_engine::InferenceEngine;
use crate::ports::voice_repository::VoiceRepository;
use crate::registry::VoiceRegistry;

/// Loads a voice via its `VoiceRepository`, validates it against the
/// `InferenceEngine`'s actual model arity, and registers it into the
/// `VoiceRegistry` for the hot synthesis path to look up later.
pub struct LoadVoice<'a> {
    pub repository: &'a dyn VoiceRepository,
    pub engine: &'a dyn InferenceEngine,
}

impl LoadVoice<'_> {
    pub fn execute(
        &self,
        voice_id: &str,
        registry: &mut VoiceRegistry,
    ) -> Result<(), VoiceLoadError> {
        let voice = self.repository.load(voice_id)?;
        self.engine
            .validate_arity(voice.num_speakers > 1)
            .map_err(|e| VoiceLoadError::ModelLoadFailure(e.to_string()))?;
        registry.register(voice);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::InferenceError;
    use crate::domain::phoneme::PhonemeIdMap;
    use crate::domain::voice::{AudioConfig, InferenceDefaults, SpeakerMap, Voice};

    struct FakeRepository {
        voice: Result<Voice, VoiceLoadError>,
    }

    impl VoiceRepository for FakeRepository {
        fn load(&self, _voice_id: &str) -> Result<Voice, VoiceLoadError> {
            self.voice.clone()
        }
    }

    struct FakeEngine {
        arity_result: Result<(), InferenceError>,
    }

    impl InferenceEngine for FakeEngine {
        fn infer(
            &mut self,
            _ids: &crate::domain::phoneme::PhonemeIdSequence,
            _params: crate::domain::inference::ResolvedInferenceParams,
        ) -> Result<crate::domain::audio::SynthesizedAudio, InferenceError> {
            unimplemented!("not exercised by LoadVoice tests")
        }

        fn validate_arity(&self, _expects_speaker_tensor: bool) -> Result<(), InferenceError> {
            self.arity_result.clone()
        }
    }

    fn voice() -> Voice {
        Voice {
            voice_id: "v1".to_string(),
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
        }
    }

    #[test]
    fn propagates_a_not_found_error_from_the_repository() {
        let repository = FakeRepository {
            voice: Err(VoiceLoadError::NotFound("v1".to_string())),
        };
        let engine = FakeEngine {
            arity_result: Ok(()),
        };
        let use_case = LoadVoice {
            repository: &repository,
            engine: &engine,
        };
        let mut registry = VoiceRegistry::new();

        let result = use_case.execute("v1", &mut registry);

        assert_eq!(result, Err(VoiceLoadError::NotFound("v1".to_string())));
    }

    #[test]
    fn surfaces_an_arity_mismatch_as_a_model_load_failure() {
        let repository = FakeRepository { voice: Ok(voice()) };
        let engine = FakeEngine {
            arity_result: Err(InferenceError::ArityMismatch {
                expected: 4,
                actual: 3,
            }),
        };
        let use_case = LoadVoice {
            repository: &repository,
            engine: &engine,
        };
        let mut registry = VoiceRegistry::new();

        let result = use_case.execute("v1", &mut registry);

        assert!(matches!(result, Err(VoiceLoadError::ModelLoadFailure(_))));
    }

    #[test]
    fn registers_the_voice_when_loading_and_validation_succeed() {
        let repository = FakeRepository { voice: Ok(voice()) };
        let engine = FakeEngine {
            arity_result: Ok(()),
        };
        let use_case = LoadVoice {
            repository: &repository,
            engine: &engine,
        };
        let mut registry = VoiceRegistry::new();

        use_case.execute("v1", &mut registry).unwrap();

        assert!(registry.lookup("v1").is_ok());
    }
}
