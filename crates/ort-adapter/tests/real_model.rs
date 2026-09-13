use dengjen_ort_adapter::OrtInferenceEngine;
use piper_core::domain::inference::ResolvedInferenceParams;
use piper_core::domain::phoneme::PhonemeIdSequence;
use piper_core::ports::inference_engine::InferenceEngine;

#[test]
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

#[test]
fn from_session_uses_the_given_sample_rate_and_can_infer() {
    let model_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/model.onnx");
    let session = ort::session::Session::builder()
        .expect("session builder")
        .commit_from_file(&model_path)
        .expect("real model should load");
    let mut engine = OrtInferenceEngine::from_session(session, 16000);

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
    assert_eq!(audio.sample_rate, 16000);
}
