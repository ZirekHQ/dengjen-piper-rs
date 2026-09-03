//! Proves `OrtInferenceEngine` satisfies the `InferenceEngine` port
//! contract, using the same real `.onnx` model file as `tests/real_model.rs`
//! (see that file's doc comment for how to fetch it). Ignored by default
//! for the same reason: no real model ships in this repo.

use dengjen_ort_adapter::OrtInferenceEngine;
use piper_core::ports::inference_engine::InferenceEngine;

piper_core::inference_engine_contract_tests!(
    || {
        let model_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/model.onnx");
        Box::new(OrtInferenceEngine::new(&model_path, 22050).expect("real model should load"))
            as Box<dyn InferenceEngine>
    },
    #[ignore = "requires a real .onnx model file; see tests/real_model.rs"]
);
