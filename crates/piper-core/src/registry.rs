use std::collections::HashMap;

use crate::domain::errors::VoiceLoadError;
use crate::domain::voice::Voice;

/// In-memory store of loaded voices. Per the design's data flow (§4): voice
/// loading is a cold path (via `LoadVoice`, populating this registry once);
/// the hot synthesis path only ever does a `lookup` here — no disk I/O or
/// JSON parsing per request.
#[derive(Default)]
pub struct VoiceRegistry {
    voices: HashMap<String, Voice>,
}

impl VoiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, voice: Voice) {
        self.voices.insert(voice.voice_id.clone(), voice);
    }

    pub fn lookup(&self, voice_id: &str) -> Result<&Voice, VoiceLoadError> {
        self.voices
            .get(voice_id)
            .ok_or_else(|| VoiceLoadError::NotFound(voice_id.to_string()))
    }

    pub fn list(&self) -> impl Iterator<Item = &Voice> {
        self.voices.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::phoneme::PhonemeIdMap;
    use crate::domain::voice::{AudioConfig, InferenceDefaults, SpeakerMap};

    fn voice(voice_id: &str) -> Voice {
        Voice {
            voice_id: voice_id.to_string(),
            audio: AudioConfig { sample_rate: 22050 },
            inference_defaults: InferenceDefaults {
                noise_scale: 0.667,
                length_scale: 1.0,
                noise_w: 0.8,
            },
            num_speakers: 1,
            speaker_map: SpeakerMap::new(),
            phoneme_id_map: PhonemeIdMap::new(),
            espeak_voice: "en-US".to_string(),
        }
    }

    #[test]
    fn lookup_returns_not_found_for_an_unregistered_voice() {
        let registry = VoiceRegistry::new();
        assert_eq!(
            registry.lookup("missing"),
            Err(VoiceLoadError::NotFound("missing".to_string()))
        );
    }

    #[test]
    fn lookup_returns_a_registered_voice() {
        let mut registry = VoiceRegistry::new();
        registry.register(voice("v1"));

        let found = registry.lookup("v1").unwrap();

        assert_eq!(found.voice_id, "v1");
    }

    #[test]
    fn list_returns_every_registered_voice() {
        let mut registry = VoiceRegistry::new();
        registry.register(voice("v1"));
        registry.register(voice("v2"));

        let mut ids: Vec<_> = registry.list().map(|v| v.voice_id.clone()).collect();
        ids.sort();

        assert_eq!(ids, vec!["v1".to_string(), "v2".to_string()]);
    }
}
