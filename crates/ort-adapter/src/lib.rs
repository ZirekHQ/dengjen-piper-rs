//! `dengjen-ort-adapter`: an `InferenceEngine` implementation wrapping the
//! `ort` crate. Ports `src/model.rs::infer()`'s tensor-construction logic
//! (unchanged since it was written) but is simpler: this adapter receives
//! an already-encoded `PhonemeIdSequence` and an already-resolved
//! `speaker_id: Option<i64>` — no `num_speakers` re-check needed here,
//! matching the R7 intent that adapters never re-derive that decision.

use ndarray::{Array1, Array2};
use ort::session::Session;
use ort::value::Tensor;
use piper_core::domain::audio::SynthesizedAudio;
use piper_core::domain::errors::InferenceError;
use piper_core::domain::inference::ResolvedInferenceParams;
use piper_core::domain::phoneme::PhonemeIdSequence;
use piper_core::ports::inference_engine::InferenceEngine;

/// The tensors one inference call needs, before they're handed to
/// `ort::inputs!`. Pure and session-free, so its shape-construction logic
/// is unit-testable without a live ONNX Runtime session or model file.
struct InputTensors {
    input: Tensor<i64>,
    input_lengths: Tensor<i64>,
    scales: Tensor<f32>,
    speaker_id: Option<Tensor<i64>>,
}

/// Builds the tensors `OrtInferenceEngine::infer` will pass to
/// `ort::inputs!`. `params.speaker_id.is_some()` is the sole signal for
/// whether a 4th (speaker-id) tensor is built — this adapter never
/// re-derives that decision from a voice's `num_speakers` itself.
fn build_input_tensors(
    ids: &PhonemeIdSequence,
    params: &ResolvedInferenceParams,
) -> InputTensors {
    let input_len = ids.0.len();
    let input_arr = Array2::<i64>::from_shape_vec((1, input_len), ids.0.clone())
        .expect("phoneme id array has exactly (1, input_len) elements by construction");
    let input_lengths_arr = Array1::<i64>::from_iter([input_len as i64]);
    let scales_arr =
        Array1::<f32>::from_iter([params.noise_scale, params.length_scale, params.noise_w]);

    let input = Tensor::<i64>::from_array((
        [1, input_len],
        input_arr.into_raw_vec_and_offset().0.into_boxed_slice(),
    ))
    .expect("input tensor shape matches the boxed slice's length by construction");
    let input_lengths = Tensor::<i64>::from_array((
        [1],
        input_lengths_arr
            .into_raw_vec_and_offset()
            .0
            .into_boxed_slice(),
    ))
    .expect("input_lengths tensor shape matches the boxed slice's length by construction");
    let scales = Tensor::<f32>::from_array((
        [3],
        scales_arr.into_raw_vec_and_offset().0.into_boxed_slice(),
    ))
    .expect("scales tensor shape matches the boxed slice's length by construction");

    let speaker_id = params.speaker_id.map(|sid| {
        let sid_arr = Array1::<i64>::from_iter([sid]);
        Tensor::<i64>::from_array(([1], sid_arr.into_raw_vec_and_offset().0.into_boxed_slice()))
            .expect("speaker id tensor shape matches the boxed slice's length by construction")
    });

    InputTensors {
        input,
        input_lengths,
        scales,
        speaker_id,
    }
}

/// Wraps one loaded ONNX Runtime session for one voice model. `sample_rate`
/// is supplied at construction (from the paired `Voice.audio.sample_rate`,
/// which `InferenceEngine::infer`'s signature has no way to receive per
/// call) since `SynthesizedAudio` must carry it on every return.
pub struct OrtInferenceEngine {
    session: Session,
    sample_rate: u32,
}

impl OrtInferenceEngine {
    pub fn new(model_path: &std::path::Path, sample_rate: u32) -> Result<Self, InferenceError> {
        let session = Session::builder()
            .map_err(|e| {
                InferenceError::RuntimeFailure(format!("failed to create session builder: {e}"))
            })?
            .commit_from_file(model_path)
            .map_err(|e| {
                InferenceError::RuntimeFailure(format!(
                    "failed to load model `{}`: {e}",
                    model_path.display()
                ))
            })?;
        Ok(Self {
            session,
            sample_rate,
        })
    }
}

impl InferenceEngine for OrtInferenceEngine {
    fn validate_arity(&self, expects_speaker_tensor: bool) -> Result<(), InferenceError> {
        let expected = if expects_speaker_tensor { 4 } else { 3 };
        let actual = self.session.inputs().len();
        if actual != expected {
            return Err(InferenceError::ArityMismatch { expected, actual });
        }
        Ok(())
    }

    fn infer(
        &mut self,
        ids: &PhonemeIdSequence,
        params: ResolvedInferenceParams,
    ) -> Result<SynthesizedAudio, InferenceError> {
        let tensors = build_input_tensors(ids, &params);

        let outputs = if let Some(speaker_id) = tensors.speaker_id {
            self.session.run(ort::inputs![
                tensors.input,
                tensors.input_lengths,
                tensors.scales,
                speaker_id
            ])
        } else {
            self.session.run(ort::inputs![
                tensors.input,
                tensors.input_lengths,
                tensors.scales
            ])
        }
        .map_err(|e| InferenceError::RuntimeFailure(format!("inference failed: {e}")))?;

        let (_, audio) = outputs[0].try_extract_tensor::<f32>().map_err(|e| {
            InferenceError::RuntimeFailure(format!("failed to extract output: {e}"))
        })?;

        Ok(SynthesizedAudio {
            samples: audio.to_vec(),
            sample_rate: self.sample_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(speaker_id: Option<i64>) -> ResolvedInferenceParams {
        ResolvedInferenceParams {
            noise_scale: 0.667,
            length_scale: 1.0,
            noise_w: 0.8,
            speaker_id,
        }
    }

    #[test]
    fn builds_three_tensors_when_speaker_id_is_none() {
        let ids = PhonemeIdSequence(vec![1, 10, 0, 2]);
        let tensors = build_input_tensors(&ids, &params(None));
        assert!(tensors.speaker_id.is_none());
    }

    #[test]
    fn builds_a_fourth_speaker_tensor_when_speaker_id_is_some() {
        let ids = PhonemeIdSequence(vec![1, 10, 0, 2]);
        let tensors = build_input_tensors(&ids, &params(Some(3)));
        assert!(tensors.speaker_id.is_some());
    }

    #[test]
    fn input_tensor_shape_matches_the_id_sequence_length() {
        // `Tensor::try_extract_tensor` returns `(&ort::value::Shape, &[T])`
        // — ort's own `Shape` type (not `ndarray::IxDyn`) and a plain
        // slice (no `.as_slice()` — it's already one).
        let ids = PhonemeIdSequence(vec![1, 10, 0, 20, 0, 2]);
        let tensors = build_input_tensors(&ids, &params(None));
        let (shape, data) = tensors.input.try_extract_tensor::<i64>().unwrap();
        assert_eq!(shape, &ort::value::Shape::new([1i64, 6]));
        assert_eq!(data.to_vec(), vec![1i64, 10, 0, 20, 0, 2]);
    }

    #[test]
    fn scales_tensor_carries_noise_scale_length_scale_noise_w_in_order() {
        let ids = PhonemeIdSequence(vec![1, 2]);
        let tensors = build_input_tensors(&ids, &params(None));
        let (_, data) = tensors.scales.try_extract_tensor::<f32>().unwrap();
        assert_eq!(data.to_vec(), vec![0.667, 1.0, 0.8]);
    }
}
