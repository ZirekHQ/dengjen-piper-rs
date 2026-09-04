
use std::collections::BTreeMap;

use espeak_rs_adapter::EspeakRsPhonemizer;
use fs_voice_repo::FsVoiceRepository;
use ort_adapter::OrtInferenceEngine;
use piper_core::domain::inference::InferenceOverrides;
use piper_core::domain::phoneme::{BOS, EOS, PAD};
use piper_core::ports::phonemizer::Phonemizer;
use piper_core::ports::voice_repository::VoiceRepository;
use piper_core::registry::VoiceRegistry;
use piper_core::use_cases::load_voice::LoadVoice;
use piper_core::use_cases::phonemize::Phonemize;
use piper_core::use_cases::synthesize::Synthesize;
use stub_adapter::StubPhonemizer;
use unicode_normalization::UnicodeNormalization;

const VOICE_ID: &str = "demo-voice";
const ESPEAK_VOICE: &str = "en-US";
const TEXT: &str = "Hello world. This is piper-rs.";

fn phoneme_id_map_for(phonemes: &str) -> BTreeMap<char, Vec<i64>> {
    let mut map = BTreeMap::new();
    map.insert(PAD, vec![0]);
    map.insert(BOS, vec![1]);
    map.insert(EOS, vec![2]);

    let mut next_id = 3i64;
    for ch in phonemes.nfd() {
        map.entry(ch).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            vec![id]
        });
    }
    map
}

fn voice_config(phoneme_id_map: &BTreeMap<char, Vec<i64>>) -> String {
    serde_json::json!({
        "audio": { "sample_rate": 22050 },
        "espeak": { "voice": ESPEAK_VOICE },
        "inference": { "noise_scale": 0.667, "length_scale": 1.0, "noise_w": 0.8 },
        "num_speakers": 1,
        "speaker_id_map": {},
        "phoneme_id_map": phoneme_id_map,
    })
    .to_string()
}

fn main() {
    let espeak_phonemizer = EspeakRsPhonemizer::default();

    let espeak_sentences = espeak_phonemizer
        .phonemize(TEXT, ESPEAK_VOICE)
        .expect("phonemize with the real espeak-ng backend");
    let joined_phonemes = espeak_sentences
        .iter()
        .map(|s| s.0.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let phoneme_id_map = phoneme_id_map_for(&joined_phonemes);

    let voice_dir = tempfile::tempdir().expect("create temp voice dir");
    std::fs::write(
        voice_dir.path().join(format!("{VOICE_ID}.onnx.json")),
        voice_config(&phoneme_id_map),
    )
    .expect("write demo voice config");

    let repository = FsVoiceRepository::new(voice_dir.path());
    let voice = repository.load(VOICE_ID).expect("load demo voice");
    println!(
        "Loaded voice {:?} ({} speaker(s), {} Hz)",
        voice.voice_id, voice.num_speakers, voice.audio.sample_rate
    );

    let mut registry = VoiceRegistry::new();
    registry.register(voice.clone());

    let stub_phonemizer = StubPhonemizer::new();
    let stub_sentences = Phonemize {
        phonemizer: &stub_phonemizer,
    }
    .execute(&registry, VOICE_ID, TEXT)
    .expect("phonemize with the dependency-free stub backend");
    println!("StubPhonemizer:     {stub_sentences:?}");

    println!("EspeakRsPhonemizer:  {espeak_sentences:?}");

    let Some(model_path) = std::env::args().nth(1) else {
        println!(
            "\nNo .onnx model path given — skipping inference. Pass one as \
             the first argument to hear the full pipeline, e.g.:\n  \
             cargo run --example hexagonal_pipeline path/to/voice.onnx"
        );
        return;
    };

    let mut engine =
        OrtInferenceEngine::new(std::path::Path::new(&model_path), voice.audio.sample_rate)
            .expect("load the given onnx model");

    let mut registry = VoiceRegistry::new();
    let load_voice = LoadVoice {
        repository: &repository,
        engine: &engine,
    };
    load_voice
        .execute(VOICE_ID, &mut registry)
        .expect("validate arity and register the voice");

    let outcome = Synthesize {
        phonemizer: &espeak_phonemizer,
        engine: &mut engine,
    }
    .execute(&registry, VOICE_ID, TEXT, InferenceOverrides::default())
    .expect("synthesize audio");

    assert!(
        outcome.warnings.is_empty(),
        "demo voice config's phoneme_id_map should cover every phoneme in \
         TEXT by construction, but got warnings: {:?}",
        outcome.warnings
    );

    println!(
        "\nSynthesized {} sample(s) at {} Hz ({} warning(s))",
        outcome.audio.samples.len(),
        outcome.audio.sample_rate,
        outcome.warnings.len()
    );
}
