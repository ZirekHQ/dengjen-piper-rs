use std::collections::HashMap;

use super::phoneme::PhonemeIdMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioConfig {
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InferenceDefaults {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
}

pub type SpeakerMap = HashMap<String, i64>;

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
    pub fn speakers(&self) -> Option<&SpeakerMap> {
        if self.num_speakers > 1 {
            Some(&self.speaker_map)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice_with(num_speakers: u32, speaker_map: SpeakerMap) -> Voice {
        Voice {
            voice_id: "test-voice".to_string(),
            audio: AudioConfig { sample_rate: 22050 },
            inference_defaults: InferenceDefaults {
                noise_scale: 0.667,
                length_scale: 1.0,
                noise_w: 0.8,
            },
            num_speakers,
            speaker_map,
            phoneme_id_map: PhonemeIdMap::new(),
            espeak_voice: "en-US".to_string(),
        }
    }

    #[test]
    fn returns_none_for_a_single_speaker_voice_with_an_empty_map() {
        let voice = voice_with(1, SpeakerMap::new());
        assert_eq!(voice.speakers(), None);
    }

    #[test]
    fn returns_the_map_for_a_multi_speaker_voice() {
        let map = SpeakerMap::from([("a".to_string(), 0i64), ("b".to_string(), 1i64)]);
        let voice = voice_with(2, map.clone());
        assert_eq!(voice.speakers(), Some(&map));
    }

    #[test]
    fn returns_some_for_a_multi_speaker_voice_even_with_an_incomplete_map() {
        let voice = voice_with(2, SpeakerMap::new());
        assert_eq!(voice.speakers(), Some(&SpeakerMap::new()));
    }
}
