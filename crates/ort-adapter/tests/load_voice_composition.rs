//! Proves the two-step composition contract documented on `LoadVoice` and
//! `OrtInferenceEngine::new` (issue #54): a `VoiceRepository::load` call
//! must happen before `OrtInferenceEngine::new`, to learn the voice's
//! `sample_rate`; only then can the already-built engine be handed to
//! `LoadVoice`. Requires a real Piper `.onnx` model; see `real_model.rs`'s
//! doc comment for how to fetch one.

use dengjen_ort_adapter::OrtInferenceEngine;
use piper_core::domain::errors::VoiceLoadError;
use piper_core::domain::phoneme::PhonemeIdMap;
use piper_core::domain::voice::{AudioConfig, InferenceDefaults, SpeakerMap, Voice};
use piper_core::ports::voice_repository::VoiceRepository;
use piper_core::registry::VoiceRegistry;
use piper_core::use_cases::load_voice::LoadVoice;

struct FixedVoiceRepository(Voice);

impl VoiceRepository for FixedVoiceRepository {
    fn load(&self, _voice_id: &str) -> Result<Voice, VoiceLoadError> {
        Ok(self.0.clone())
    }
}

#[test]
#[ignore = "requires a real .onnx model file; see real_model.rs's doc comment"]
fn engine_built_from_the_loaded_voice_s_sample_rate_passes_load_voice() {
    let model_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/model.onnx");
    let voice = Voice {
        voice_id: "v1".to_string(),
        audio: AudioConfig { sample_rate: 22050 },
        inference_defaults: InferenceDefaults {
            noise_scale: 0.667,
            length_scale: 1.0,
            noise_w: 0.8,
        },
        // The fixture at `tests/fixtures/model.onnx` (see `real_model.rs`'s
        // doc comment) is a multi-speaker model (4 input tensors); match
        // that here so this test exercises the real arity check.
        num_speakers: 2,
        speaker_map: SpeakerMap::new(),
        phoneme_id_map: PhonemeIdMap::new(),
        espeak_voice: "en-US".to_string(),
    };
    let repository = FixedVoiceRepository(voice.clone());

    // Step 1: load the voice config first — the only place `sample_rate`
    // is known before the engine exists.
    let loaded = repository.load(&voice.voice_id).unwrap();

    // Step 2: construct the engine now that its sample_rate is known.
    let engine = OrtInferenceEngine::new(&model_path, loaded.audio.sample_rate)
        .expect("real model should load");

    // Step 3: hand both to `LoadVoice`, which re-loads the voice and
    // validates the engine's arity against it.
    let mut registry = VoiceRegistry::new();
    LoadVoice {
        repository: &repository,
        engine: &engine,
    }
    .execute(&voice.voice_id, &mut registry)
    .expect("load should succeed with a matching sample rate and arity");

    assert_eq!(
        registry.lookup(&voice.voice_id).unwrap().audio.sample_rate,
        22050
    );
}
