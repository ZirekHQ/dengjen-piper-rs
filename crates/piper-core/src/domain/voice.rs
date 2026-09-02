use std::collections::HashMap;

use super::phoneme::PhonemeIdMap;

/// Declares the PCM sample rate a `Voice`'s underlying model emits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioConfig {
    pub sample_rate: u32,
}

/// Default synthesis hyperparameters baked into a `Voice`; overridable per
/// call via `InferenceOverrides`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InferenceDefaults {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
}

/// Named-speaker to integer id table for multi-speaker voices; empty for
/// single-speaker voices.
pub type SpeakerMap = HashMap<String, i64>;

/// The aggregate root of the synthesis domain: one loaded, servable
/// synthesis target. Its config plus loaded inference session (held by the
/// `InferenceEngine` adapter, not here) form the consistency boundary — this
/// struct is the config half, port-agnostic.
#[derive(Debug, Clone, PartialEq)]
pub struct Voice {
    pub voice_id: String,
    pub audio: AudioConfig,
    pub inference_defaults: InferenceDefaults,
    pub num_speakers: u32,
    pub speaker_map: SpeakerMap,
    pub phoneme_id_map: PhonemeIdMap,
    pub espeak_voice: String,
}

impl Voice {
    /// Returns the speaker name→id map, or `None` for single-speaker
    /// voices — distinguishing "no speaker support" from "supports
    /// speakers but none are listed" (R10).
    pub fn speakers(&self) -> Option<&SpeakerMap> {
        if self.speaker_map.is_empty() {
            None
        } else {
            Some(&self.speaker_map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice_with_speaker_map(speaker_map: SpeakerMap) -> Voice {
        Voice {
            voice_id: "test-voice".to_string(),
            audio: AudioConfig { sample_rate: 22050 },
            inference_defaults: InferenceDefaults {
                noise_scale: 0.667,
                length_scale: 1.0,
                noise_w: 0.8,
            },
            num_speakers: if speaker_map.is_empty() { 1 } else { speaker_map.len() as u32 },
            speaker_map,
            phoneme_id_map: PhonemeIdMap::new(),
            espeak_voice: "en-US".to_string(),
        }
    }

    #[test]
    fn returns_none_for_an_empty_speaker_map() {
        let voice = voice_with_speaker_map(SpeakerMap::new());
        assert_eq!(voice.speakers(), None);
    }

    #[test]
    fn returns_the_map_when_it_has_entries() {
        let map = SpeakerMap::from([("default".to_string(), 0i64)]);
        let voice = voice_with_speaker_map(map.clone());
        assert_eq!(voice.speakers(), Some(&map));
    }
}
