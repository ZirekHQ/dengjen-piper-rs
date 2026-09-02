//! The `InferenceEngine` port contract.

#[macro_export]
macro_rules! inference_engine_contract_tests {
    ($make:expr) => {
        #[test]
        fn validate_arity_does_not_panic_for_either_speaker_mode() {
            let engine = $make();
            let _ = engine.validate_arity(false);
            let _ = engine.validate_arity(true);
        }

        #[test]
        fn infer_returns_audio_with_a_positive_sample_rate_on_success() {
            let mut engine = $make();
            let ids = $crate::domain::phoneme::PhonemeIdSequence(vec![1, 2, 3]);
            let params = $crate::domain::inference::ResolvedInferenceParams {
                noise_scale: 0.667,
                length_scale: 1.0,
                noise_w: 0.8,
                speaker_id: None,
            };
            if let Ok(audio) = engine.infer(&ids, params) {
                assert!(audio.sample_rate > 0, "sample rate must be positive");
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::domain::audio::SynthesizedAudio;
    use crate::domain::errors::InferenceError;
    use crate::domain::inference::ResolvedInferenceParams;
    use crate::domain::phoneme::PhonemeIdSequence;
    use crate::ports::inference_engine::InferenceEngine;

    struct ContractFakeEngine;

    impl InferenceEngine for ContractFakeEngine {
        fn infer(
            &mut self,
            _ids: &PhonemeIdSequence,
            _params: ResolvedInferenceParams,
        ) -> Result<SynthesizedAudio, InferenceError> {
            Ok(SynthesizedAudio { samples: vec![0.0], sample_rate: 16000 })
        }

        fn validate_arity(&self, _expects_speaker_tensor: bool) -> Result<(), InferenceError> {
            Ok(())
        }
    }

    crate::inference_engine_contract_tests!(|| Box::new(ContractFakeEngine) as Box<dyn InferenceEngine>);
}
