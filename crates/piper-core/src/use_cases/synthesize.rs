use crate::domain::audio::SynthesizedAudio;
use crate::domain::errors::{SynthesizeError, VoiceLoadError};
use crate::domain::inference::InferenceOverrides;
use crate::domain::phoneme::{PhonemizationWarning, encode_phonemes};
use crate::ports::inference_engine::InferenceEngine;
use crate::ports::phonemizer::Phonemizer;
use crate::registry::VoiceRegistry;

use super::phonemize::{Phonemize, PhonemizeError};

/// The result of a successful synthesis: audio plus any non-fatal
/// degradation encountered while encoding phonemes (design §6 — unmapped
/// phonemes return audio with a warning, not a silent drop or hard failure).
#[derive(Debug, Clone, PartialEq)]
pub struct SynthesizeOutcome {
    pub audio: SynthesizedAudio,
    pub warnings: Vec<PhonemizationWarning>,
}

impl From<VoiceLoadError> for SynthesizeError {
    fn from(e: VoiceLoadError) -> Self {
        match e {
            VoiceLoadError::NotFound(id) => Self::VoiceNotFound(id),
            other => Self::VoiceNotFound(other.to_string()),
        }
    }
}

impl From<PhonemizeError> for SynthesizeError {
    fn from(e: PhonemizeError) -> Self {
        match e {
            PhonemizeError::VoiceNotFound(id) => Self::VoiceNotFound(id),
            PhonemizeError::Phonemization(err) => Self::Phonemization(err),
        }
    }
}

/// Orchestrates the full hot-path synthesis flow (design §4): look up the
/// voice, phonemize the text, encode phonemes to ids, resolve inference
/// parameters against the voice's defaults, then run inference.
pub struct Synthesize<'a> {
    pub phonemizer: &'a dyn Phonemizer,
    pub engine: &'a mut dyn InferenceEngine,
}

impl Synthesize<'_> {
    pub fn execute(
        &mut self,
        registry: &VoiceRegistry,
        voice_id: &str,
        text: &str,
        overrides: InferenceOverrides,
    ) -> Result<SynthesizeOutcome, SynthesizeError> {
        let voice = registry.lookup(voice_id)?;

        let phonemize = Phonemize {
            phonemizer: self.phonemizer,
        };
        let sentences = phonemize.execute(registry, voice_id, text)?;
        let phonemes: String = sentences
            .into_iter()
            .map(|s| s.0)
            .collect::<Vec<_>>()
            .join(" ");

        let encoding = encode_phonemes(&voice.phoneme_id_map, &phonemes);
        let params = voice.resolve_inference_params(overrides);
        let audio = self.engine.infer(&encoding.ids, params)?;

        Ok(SynthesizeOutcome {
            audio,
            warnings: encoding.warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::{InferenceError, PhonemizationError};
    use crate::domain::inference::ResolvedInferenceParams;
    use crate::domain::phoneme::{BOS, EOS, PAD, PhonemeIdMap, PhonemeIdSequence};
    use crate::domain::voice::{AudioConfig, InferenceDefaults, SpeakerMap, Voice};
    use crate::ports::phonemizer::Sentence;

    struct FakePhonemizer {
        result: Result<Vec<Sentence>, PhonemizationError>,
    }

    impl Phonemizer for FakePhonemizer {
        fn phonemize(
            &self,
            _text: &str,
            _voice: &str,
        ) -> Result<Vec<Sentence>, PhonemizationError> {
            self.result.clone()
        }
    }

    struct FakeEngine {
        infer_result: Result<SynthesizedAudio, InferenceError>,
        received_params: Option<ResolvedInferenceParams>,
        received_ids: Option<PhonemeIdSequence>,
    }

    impl InferenceEngine for FakeEngine {
        fn infer(
            &mut self,
            ids: &PhonemeIdSequence,
            params: ResolvedInferenceParams,
        ) -> Result<SynthesizedAudio, InferenceError> {
            self.received_params = Some(params);
            self.received_ids = Some(ids.clone());
            self.infer_result.clone()
        }

        fn validate_arity(&self, _expects_speaker_tensor: bool) -> Result<(), InferenceError> {
            Ok(())
        }
    }

    fn registry_with_voice(phoneme_id_map: PhonemeIdMap) -> VoiceRegistry {
        let mut registry = VoiceRegistry::new();
        registry.register(Voice {
            voice_id: "v1".to_string(),
            audio: AudioConfig { sample_rate: 22050 },
            inference_defaults: InferenceDefaults {
                noise_scale: 0.667,
                length_scale: 1.0,
                noise_w: 0.8,
            },
            num_speakers: 1,
            speaker_map: SpeakerMap::new(),
            phoneme_id_map,
            espeak_voice: "en-US".to_string(),
        });
        registry
    }

    fn full_map() -> PhonemeIdMap {
        PhonemeIdMap::from([
            (BOS, vec![1]),
            (PAD, vec![0]),
            (EOS, vec![2]),
            ('a', vec![10]),
        ])
    }

    #[test]
    fn returns_voice_not_found_for_an_unregistered_voice() {
        let phonemizer = FakePhonemizer { result: Ok(vec![]) };
        let mut engine = FakeEngine {
            infer_result: Ok(SynthesizedAudio {
                samples: vec![],
                sample_rate: 22050,
            }),
            received_params: None,
            received_ids: None,
        };
        let mut use_case = Synthesize {
            phonemizer: &phonemizer,
            engine: &mut engine,
        };
        let registry = VoiceRegistry::new();

        let result = use_case.execute(&registry, "missing", "a", InferenceOverrides::default());

        assert_eq!(
            result,
            Err(SynthesizeError::VoiceNotFound("missing".to_string()))
        );
    }

    #[test]
    fn propagates_a_phonemization_error() {
        let phonemizer = FakePhonemizer {
            result: Err(PhonemizationError::Timeout),
        };
        let mut engine = FakeEngine {
            infer_result: Ok(SynthesizedAudio {
                samples: vec![],
                sample_rate: 22050,
            }),
            received_params: None,
            received_ids: None,
        };
        let mut use_case = Synthesize {
            phonemizer: &phonemizer,
            engine: &mut engine,
        };
        let registry = registry_with_voice(full_map());

        let result = use_case.execute(&registry, "v1", "a", InferenceOverrides::default());

        assert_eq!(
            result,
            Err(SynthesizeError::Phonemization(PhonemizationError::Timeout))
        );
    }

    #[test]
    fn propagates_an_inference_error() {
        let phonemizer = FakePhonemizer {
            result: Ok(vec![Sentence("a".to_string())]),
        };
        let mut engine = FakeEngine {
            infer_result: Err(InferenceError::RuntimeFailure("boom".to_string())),
            received_params: None,
            received_ids: None,
        };
        let mut use_case = Synthesize {
            phonemizer: &phonemizer,
            engine: &mut engine,
        };
        let registry = registry_with_voice(full_map());

        let result = use_case.execute(&registry, "v1", "a", InferenceOverrides::default());

        assert_eq!(
            result,
            Err(SynthesizeError::Inference(InferenceError::RuntimeFailure(
                "boom".to_string()
            )))
        );
    }

    #[test]
    fn returns_audio_and_a_warning_for_an_unmapped_phoneme_instead_of_failing() {
        let phonemizer = FakePhonemizer {
            result: Ok(vec![Sentence("ab".to_string())]),
        };
        let mut engine = FakeEngine {
            infer_result: Ok(SynthesizedAudio {
                samples: vec![0.5],
                sample_rate: 22050,
            }),
            received_params: None,
            received_ids: None,
        };
        // 'b' is deliberately absent from the map.
        let registry = registry_with_voice(full_map());

        let outcome = {
            let mut use_case = Synthesize {
                phonemizer: &phonemizer,
                engine: &mut engine,
            };
            use_case
                .execute(&registry, "v1", "ab", InferenceOverrides::default())
                .unwrap()
        };

        assert_eq!(outcome.audio.samples, vec![0.5]);
        assert_eq!(
            outcome.warnings,
            vec![PhonemizationWarning::UnmappedPhoneme { phoneme: 'b' }]
        );
        // Proves the dropped phoneme's absence isn't just asserted at the
        // domain layer (Task 1's drops_unmapped_phonemes_and_warns) but
        // actually reaches the engine this way: 'a' -> [10], 'b' dropped
        // entirely (no ids, no trailing PAD for it), wrapped in BOS/EOS.
        assert_eq!(
            engine.received_ids,
            Some(PhonemeIdSequence(vec![1, 10, 0, 2]))
        );
    }

    #[test]
    fn resolves_inference_params_against_voice_defaults_before_calling_the_engine() {
        let phonemizer = FakePhonemizer {
            result: Ok(vec![Sentence("a".to_string())]),
        };
        let mut engine = FakeEngine {
            infer_result: Ok(SynthesizedAudio {
                samples: vec![],
                sample_rate: 22050,
            }),
            received_params: None,
            received_ids: None,
        };
        let overrides = InferenceOverrides {
            length_scale: Some(1.5),
            ..Default::default()
        };
        let registry = registry_with_voice(full_map());

        {
            let mut use_case = Synthesize {
                phonemizer: &phonemizer,
                engine: &mut engine,
            };
            use_case.execute(&registry, "v1", "a", overrides).unwrap();
        }

        let params = engine
            .received_params
            .expect("infer should have been called");
        assert_eq!(params.length_scale, 1.5);
        assert_eq!(params.noise_scale, 0.667);
    }
}
