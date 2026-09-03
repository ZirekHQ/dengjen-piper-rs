//! Requires a real Piper `.onnx` model, downloaded manually — matching how
//! `examples/*.rs` at the repo root already require manual setup (`wget`
//! from rhasspy/piper-voices). Run with:
//!
//! ```text
//! wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx \
//!   -O crates/ort-adapter/tests/fixtures/model.onnx
//! cargo test -p dengjen-ort-adapter --test real_model -- --ignored
//! ```

use dengjen_ort_adapter::OrtInferenceEngine;
use piper_core::domain::inference::ResolvedInferenceParams;
use piper_core::domain::phoneme::PhonemeIdSequence;
use piper_core::ports::inference_engine::InferenceEngine;

#[test]
#[ignore = "requires a real .onnx model file; see this file's doc comment"]
fn infers_against_a_real_onnx_model() {
    let model_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/model.onnx");
    let mut engine = OrtInferenceEngine::new(&model_path, 22050).expect("real model should load");

    engine
        .validate_arity(false)
        .expect("single-speaker model should have 3 inputs");

    let ids = PhonemeIdSequence(vec![1, 10, 0, 2]);
    let params = ResolvedInferenceParams {
        noise_scale: 0.667,
        length_scale: 1.0,
        noise_w: 0.8,
        speaker_id: None,
    };
    let audio = engine
        .infer(&ids, params)
        .expect("inference should succeed");

    assert!(!audio.samples.is_empty());
    assert_eq!(audio.sample_rate, 22050);
}
