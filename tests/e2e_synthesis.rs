#[cfg(feature = "espeak-rs")]
use std::collections::HashMap;

#[cfg(feature = "espeak-rs")]
fn write_voice_config(dir: &std::path::Path, voice_id: &str) {
    let mut phoneme_id_map: HashMap<char, Vec<i64>> = HashMap::new();
    phoneme_id_map.insert('^', vec![1]);
    phoneme_id_map.insert('_', vec![0]);
    phoneme_id_map.insert('$', vec![2]);
    phoneme_id_map.insert('t', vec![10]);
    phoneme_id_map.insert('ˈ', vec![11]);
    phoneme_id_map.insert('ɛ', vec![12]);
    phoneme_id_map.insert('s', vec![13]);

    let config = serde_json::json!({
        "audio": { "sample_rate": 16000 },
        "espeak": { "voice": "en-US" },
        "inference": { "noise_scale": 0.667, "length_scale": 1.0, "noise_w": 0.8 },
        "num_speakers": 1,
        "speaker_id_map": {},
        "phoneme_id_map": phoneme_id_map,
    });

    std::fs::write(
        dir.join(format!("{voice_id}.onnx.json")),
        config.to_string(),
    )
    .expect("write voice config");
}

#[cfg(feature = "espeak-rs")]
#[test]
fn piper_synthesizes_non_empty_audio_end_to_end() {
    let voice_dir = tempfile::tempdir().expect("create temp voice dir");
    write_voice_config(voice_dir.path(), "e2e-voice");
    let config_path = voice_dir.path().join("e2e-voice.onnx.json");
    let model_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/ort-adapter/tests/fixtures/model.onnx");

    let mut piper = dengjen_piper_rs::Piper::new(&model_path, &config_path).expect("load Piper");

    let (samples, sample_rate) = piper
        .create("test", false, None, None, None, None)
        .expect("synthesis should succeed");

    assert!(!samples.is_empty());
    assert_eq!(sample_rate, 16000);
}
