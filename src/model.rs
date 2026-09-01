use std::collections::HashMap;

use ndarray::{Array1, Array2};
use ort::session::Session;
use ort::value::Tensor;
use serde::Deserialize;

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
pub fn phonemes_to_ids(config: &ModelConfig, phonemes: &str) -> Vec<i64> {
    let map = &config.phoneme_id_map;
    let default_id = [0i64];
    let bos_ids = map.get(&BOS).map(Vec::as_slice).unwrap_or(&default_id);
    let pad_ids = map.get(&PAD).map(Vec::as_slice).unwrap_or(&default_id);
    let eos_ids = map.get(&EOS).map(Vec::as_slice).unwrap_or(&default_id);

    let mut ids = Vec::with_capacity((phonemes.len() + 1) * 2);
    ids.extend_from_slice(bos_ids);
    for ch in phonemes.chars() {
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
