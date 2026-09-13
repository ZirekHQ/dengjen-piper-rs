use std::collections::HashMap;

use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

pub const BOS: char = '^';
pub const EOS: char = '$';
pub const PAD: char = '_';

#[derive(Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
}

#[derive(Deserialize)]
pub struct ESpeakConfig {
    pub voice: String,
}

#[derive(Deserialize, Clone)]
pub struct InferenceConfig {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
}

#[derive(Deserialize)]
pub struct ModelConfig {
    pub audio: AudioConfig,
    pub espeak: ESpeakConfig,
    pub inference: InferenceConfig,
    pub num_speakers: u32,
    pub speaker_id_map: HashMap<String, i64>,
    pub phoneme_id_map: HashMap<char, Vec<i64>>,
}

pub(crate) fn model_config_to_voice(
    voice_id: &str,
    config: &ModelConfig,
) -> piper_core::domain::voice::Voice {
    piper_core::domain::voice::Voice {
        voice_id: voice_id.to_string(),
        audio: piper_core::domain::voice::AudioConfig {
            sample_rate: config.audio.sample_rate,
        },
        inference_defaults: piper_core::domain::voice::InferenceDefaults {
            noise_scale: config.inference.noise_scale,
            length_scale: config.inference.length_scale,
            noise_w: config.inference.noise_w,
        },
        num_speakers: config.num_speakers,
        speaker_map: config.speaker_id_map.clone(),
        phoneme_id_map: config.phoneme_id_map.clone(),
        espeak_voice: config.espeak.voice.clone(),
    }
}

#[deprecated(note = "use piper_core::domain::phoneme::encode_phonemes")]
pub fn phonemes_to_ids(config: &ModelConfig, phonemes: &str) -> Vec<i64> {
    let map = &config.phoneme_id_map;
    let default_id = [0i64];
    let bos_ids = map.get(&BOS).map(Vec::as_slice).unwrap_or(&default_id);
    let pad_ids = map.get(&PAD).map(Vec::as_slice).unwrap_or(&default_id);
    let eos_ids = map.get(&EOS).map(Vec::as_slice).unwrap_or(&default_id);

    let mut ids = Vec::with_capacity((phonemes.len() + 1) * 2);
    ids.extend_from_slice(bos_ids);
    for ch in phonemes.nfd() {
        if let Some(phoneme_ids) = map.get(&ch) {
            ids.extend_from_slice(phoneme_ids);
            ids.extend_from_slice(pad_ids);
        }
    }
    ids.extend_from_slice(eos_ids);
    ids
}

#[cfg(test)]
#[allow(deprecated)] // exercises phonemes_to_ids directly; local pre-commit and CI both run clippy --all-targets -D warnings
mod tests {
    use super::*;

    fn config_with_map(phoneme_id_map: HashMap<char, Vec<i64>>) -> ModelConfig {
        ModelConfig {
            audio: AudioConfig { sample_rate: 22050 },
            espeak: ESpeakConfig {
                voice: "en-US".to_string(),
            },
            inference: InferenceConfig {
                noise_scale: 0.667,
                length_scale: 1.0,
                noise_w: 0.8,
            },
            num_speakers: 1,
            speaker_id_map: HashMap::new(),
            phoneme_id_map,
        }
    }

    #[test]
    fn extends_every_id_in_a_multi_id_phoneme_mapping() {
        let config = config_with_map(HashMap::from([
            (BOS, vec![1]),
            (PAD, vec![0]),
            (EOS, vec![2]),
            ('a', vec![10, 11]),
        ]));

        assert_eq!(phonemes_to_ids(&config, "a"), vec![1, 10, 11, 0, 2]);
    }

    #[test]
    fn inserts_pad_after_each_phoneme_but_not_after_bos() {
        let config = config_with_map(HashMap::from([
            (BOS, vec![1]),
            (PAD, vec![0]),
            (EOS, vec![2]),
            ('a', vec![10]),
            ('b', vec![20]),
        ]));

        assert_eq!(phonemes_to_ids(&config, "ab"), vec![1, 10, 0, 20, 0, 2]);
    }

    #[test]
    fn skips_phonemes_missing_from_the_map() {
        let config = config_with_map(HashMap::from([
            (BOS, vec![1]),
            (PAD, vec![0]),
            (EOS, vec![2]),
            ('a', vec![10]),
        ]));

        assert_eq!(phonemes_to_ids(&config, "ab"), vec![1, 10, 0, 2]);
    }

    #[test]
    fn falls_back_to_zero_when_bos_pad_eos_are_missing() {
        let config = config_with_map(HashMap::from([('a', vec![5])]));

        assert_eq!(phonemes_to_ids(&config, "a"), vec![0, 5, 0, 0]);
    }

    #[test]
    fn normalizes_composed_phonemes_to_nfd_before_lookup() {
        let config = config_with_map(HashMap::from([
            (BOS, vec![1]),
            (PAD, vec![0]),
            (EOS, vec![2]),
            ('c', vec![10]),
            ('\u{0327}', vec![11]),
        ]));

        assert_eq!(
            phonemes_to_ids(&config, "\u{00E7}"),
            vec![1, 10, 0, 11, 0, 2]
        );
    }

    #[test]
    fn maps_every_model_config_field_onto_the_voice() {
        let mut phoneme_id_map = HashMap::new();
        phoneme_id_map.insert('a', vec![10i64]);
        let mut speaker_id_map = HashMap::new();
        speaker_id_map.insert("alice".to_string(), 0i64);
        let config = ModelConfig {
            audio: AudioConfig { sample_rate: 22050 },
            espeak: ESpeakConfig {
                voice: "en-US".to_string(),
            },
            inference: InferenceConfig {
                noise_scale: 0.667,
                length_scale: 1.0,
                noise_w: 0.8,
            },
            num_speakers: 2,
            speaker_id_map: speaker_id_map.clone(),
            phoneme_id_map: phoneme_id_map.clone(),
        };

        let voice = model_config_to_voice("test-voice", &config);

        assert_eq!(voice.voice_id, "test-voice");
        assert_eq!(voice.audio.sample_rate, 22050);
        assert_eq!(voice.espeak_voice, "en-US");
        assert_eq!(voice.num_speakers, 2);
        assert_eq!(voice.inference_defaults.noise_scale, 0.667);
        assert_eq!(voice.inference_defaults.length_scale, 1.0);
        assert_eq!(voice.inference_defaults.noise_w, 0.8);
        assert_eq!(voice.speaker_map, speaker_id_map);
        assert_eq!(voice.phoneme_id_map, phoneme_id_map);
    }

    #[cfg(feature = "espeak-rs")]
    #[test]
    fn french_tom_medium_maps_every_phoneme_from_real_sentences() {
        let config: ModelConfig =
            serde_json::from_str(include_str!("../tests/fixtures/fr_FR-tom-medium.onnx.json"))
                .expect("fixture should deserialize as ModelConfig");

        let sentences = [
            "Le garçon a une leçon de français.",
            "Voilà, c'est ça, ça va ? Où êtes-vous ? Il a vécu à Noël.",
            "Un œuf, une sœur, un cœur.",
            "123 personnes étaient présentes le 4 juillet.",
        ];

        for sentence in sentences {
            let phonemes = espeak_rs::text_to_phonemes(sentence, &config.espeak.voice, None)
                .expect("espeak-ng should phonemize French text")
                .join(" ");

            for ch in phonemes.nfd() {
                assert!(
                    config.phoneme_id_map.contains_key(&ch),
                    "phoneme {ch:?} (U+{:04X}) from {sentence:?} has no entry in \
                     fr_FR-tom-medium's phoneme_id_map and would be silently dropped",
                    ch as u32
                );
            }
        }
    }
}
