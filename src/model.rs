use std::collections::HashMap;

use ndarray::{Array1, Array2};
use ort::session::Session;
use ort::value::Tensor;
use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

use crate::PiperError;
use crate::PiperResult;

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

// Mirrors Piper's own phonemes_to_ids: BOS ids, then each phoneme's ids
// followed by PAD's ids, then EOS ids. Every mapping is a Vec<i64> because a
// single phoneme can map to more than one id, so each entry is extended in
// full rather than truncated to its first id.
//
// `phoneme_id_map` is built from the model's training vocabulary, which uses
// NFD (decomposed) Unicode — e.g. 'c' + a combining cedilla rather than the
// composed 'ç'. espeak-ng emits composed phonemes, so the input is
// NFD-normalized here before the per-char lookup; without this, composed
// phonemes have no entry in the map and are silently dropped like any other
// unknown phoneme.
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
        // The model is trained on NFD (decomposed) Unicode, e.g. 'c' followed
        // by a combining cedilla (U+0327), but espeak-ng emits the composed
        // form 'ç' (U+00E7) as a single codepoint. Without NFD normalization
        // that composed codepoint isn't in the map and is silently dropped.
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

    // Regression guard for #12 ("output for some french models is low and
    // garbled"): reproduces the reported model end-to-end through
    // phonemization by asserting every NFD-normalized phoneme espeak-ng
    // produces for a battery of French sentences — nasal vowels, cedilla,
    // the œ ligature, accents, expanded numbers — has an entry in the real
    // fr_FR-tom-medium phoneme_id_map, i.e. none would be silently dropped
    // by `phonemes_to_ids`. On this specific model the map already covers
    // both composed and decomposed forms, so synthesis wasn't actually
    // broken by the NFD gap `normalizes_composed_phonemes_to_nfd_before_lookup`
    // fixed for other languages; this pins that down and catches any future
    // regression in this model's phoneme coverage. The fixture is
    // `fr_FR-tom-medium.onnx.json`'s config as published on Hugging Face
    // (rhasspy/piper-voices) — no binary weights, just the phoneme_id_map
    // and metadata this test reads.
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

pub fn infer(
    session: &mut Session,
    config: &ModelConfig,
    phonemes: &str,
    noise_scale: f32,
    length_scale: f32,
    noise_w: f32,
    speaker_id: i64,
) -> PiperResult<Vec<f32>> {
    let ids = phonemes_to_ids(config, phonemes);
    let input_len = ids.len();
    let input = Array2::<i64>::from_shape_vec((1, input_len), ids).unwrap();
    let input_lengths = Array1::<i64>::from_iter([input_len as i64]);
    let scales = Array1::<f32>::from_iter([noise_scale, length_scale, noise_w]);

    let input_t = Tensor::<i64>::from_array((
        [1, input_len],
        input.into_raw_vec_and_offset().0.into_boxed_slice(),
    ))
    .unwrap();
    let lengths_t = Tensor::<i64>::from_array((
        [1],
        input_lengths.into_raw_vec_and_offset().0.into_boxed_slice(),
    ))
    .unwrap();
    let scales_t =
        Tensor::<f32>::from_array(([3], scales.into_raw_vec_and_offset().0.into_boxed_slice()))
            .unwrap();

    let outputs = if config.num_speakers > 1 {
        let sid = Array1::<i64>::from_iter([speaker_id]);
        let sid_t =
            Tensor::<i64>::from_array(([1], sid.into_raw_vec_and_offset().0.into_boxed_slice()))
                .unwrap();
        session.run(ort::inputs![input_t, lengths_t, scales_t, sid_t])
    } else {
        session.run(ort::inputs![input_t, lengths_t, scales_t])
    }
    .map_err(|e| PiperError::InferenceError(format!("Inference failed: {}", e)))?;

    let (_, audio) = outputs[0]
        .try_extract_tensor::<f32>()
        .map_err(|e| PiperError::InferenceError(format!("Failed to extract output: {}", e)))?;

    Ok(audio.to_vec())
}
