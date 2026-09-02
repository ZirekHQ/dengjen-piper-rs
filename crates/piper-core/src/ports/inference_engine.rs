use crate::domain::audio::SynthesizedAudio;
use crate::domain::errors::InferenceError;
use crate::domain::inference::ResolvedInferenceParams;
use crate::domain::phoneme::PhonemeIdSequence;

/// Runs inference over an encoded phoneme sequence, producing audio.
/// Implemented by adapter crates (e.g. `ort-adapter`) — no `ort` type
/// appears in this trait's signature, by design.
pub trait InferenceEngine: Send + Sync {
    fn infer(
        &mut self,
        ids: &PhonemeIdSequence,
        params: ResolvedInferenceParams,
    ) -> Result<SynthesizedAudio, InferenceError>;

    /// Validates that the loaded model's actual input tensor count matches
    /// what the voice config implies (3 tensors for single-speaker, 4 for
    /// multi-speaker). Called once at `LoadVoice` time (closes
    /// AI_NATIVE_SPEC.md §6 item 5) so an arity mismatch is a load-time
    /// `InferenceError::ArityMismatch`, not an opaque runtime failure on
    /// the first inference request.
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
            output: SynthesizedAudio { samples: vec![0.1, 0.2], sample_rate: 22050 },
        });
        let params = ResolvedInferenceParams {
            noise_scale: 0.667,
            length_scale: 1.0,
            noise_w: 0.8,
            speaker_id: None,
        };
        let result = engine.infer(&PhonemeIdSequence(vec![1, 2, 3]), params).unwrap();
        assert_eq!(result.samples, vec![0.1, 0.2]);
    }
}
