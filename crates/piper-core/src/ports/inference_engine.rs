use crate::domain::audio::SynthesizedAudio;
use crate::domain::errors::InferenceError;
use crate::domain::inference::ResolvedInferenceParams;
use crate::domain::phoneme::PhonemeIdSequence;

pub trait InferenceEngine: Send + Sync {
    fn infer(
        &mut self,
        ids: &PhonemeIdSequence,
        params: ResolvedInferenceParams,
    ) -> Result<SynthesizedAudio, InferenceError>;

    fn validate_arity(&self, expects_speaker_tensor: bool) -> Result<(), InferenceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedOutputEngine {
        output: SynthesizedAudio,
    }

    impl InferenceEngine for FixedOutputEngine {
        fn infer(
            &mut self,
            _ids: &PhonemeIdSequence,
            _params: ResolvedInferenceParams,
        ) -> Result<SynthesizedAudio, InferenceError> {
            Ok(self.output.clone())
        }

        fn validate_arity(&self, _expects_speaker_tensor: bool) -> Result<(), InferenceError> {
            Ok(())
        }
    }

    #[test]
    fn trait_object_can_be_called_through_a_dyn_reference() {
        let mut engine: Box<dyn InferenceEngine> = Box::new(FixedOutputEngine {
            output: SynthesizedAudio {
                samples: vec![0.1, 0.2],
                sample_rate: 22050,
            },
        });
        let params = ResolvedInferenceParams {
            noise_scale: 0.667,
            length_scale: 1.0,
            noise_w: 0.8,
            speaker_id: None,
        };
        let result = engine
            .infer(&PhonemeIdSequence(vec![1, 2, 3]), params)
            .unwrap();
        assert_eq!(result.samples, vec![0.1, 0.2]);
    }
}
