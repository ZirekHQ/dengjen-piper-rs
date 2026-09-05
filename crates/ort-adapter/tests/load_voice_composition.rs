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
        num_speakers: 2,
        speaker_map: SpeakerMap::new(),
        phoneme_id_map: PhonemeIdMap::new(),
        espeak_voice: "en-US".to_string(),
    };
    let repository = FixedVoiceRepository(voice.clone());

    let loaded = repository.load(&voice.voice_id).unwrap();

    let engine = OrtInferenceEngine::new(&model_path, loaded.audio.sample_rate)
        .expect("real model should load");

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
