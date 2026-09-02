use super::voice::Voice;

/// Per-call overrides for a `Voice`'s default inference parameters. A
/// `None` field falls back to the voice's `InferenceDefaults`.
#[derive(Debug, Clone, Copy, Default)]
pub struct InferenceOverrides {
    pub speaker_id: Option<i64>,
    pub length_scale: Option<f32>,
    pub noise_scale: Option<f32>,
    pub noise_w: Option<f32>,
}

/// Fully-resolved inference parameters, ready to hand to an
/// `InferenceEngine` port implementation. `speaker_id` is `Some` only when
/// the voice is multi-speaker (R7): a single-speaker voice's inference
/// engine adapter must never build a speaker-id tensor, regardless of what
/// the caller requested, so that decision is made once here rather than
/// re-derived in every adapter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedInferenceParams {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
    pub speaker_id: Option<i64>,
}

impl Voice {
    /// Resolves per-call `overrides` against this voice's
    /// `InferenceDefaults`, individually falling back to the default for
    /// each unset field (R8), defaulting an unset speaker id to `0` (R9),
    /// and omitting the speaker id entirely for single-speaker voices (R7).
    pub fn resolve_inference_params(
        &self,
        overrides: InferenceOverrides,
    ) -> ResolvedInferenceParams {
        let defaults = &self.inference_defaults;
        ResolvedInferenceParams {
            noise_scale: overrides.noise_scale.unwrap_or(defaults.noise_scale),
            length_scale: overrides.length_scale.unwrap_or(defaults.length_scale),
            noise_w: overrides.noise_w.unwrap_or(defaults.noise_w),
            speaker_id: (self.num_speakers > 1).then(|| overrides.speaker_id.unwrap_or(0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::phoneme::PhonemeIdMap;
    use crate::domain::voice::{AudioConfig, InferenceDefaults, SpeakerMap};

    fn voice(num_speakers: u32) -> Voice {
        Voice {
            voice_id: "test-voice".to_string(),
            audio: AudioConfig { sample_rate: 22050 },
            inference_defaults: InferenceDefaults {
                noise_scale: 0.667,
                length_scale: 1.0,
                noise_w: 0.8,
            },
            num_speakers,
            speaker_map: SpeakerMap::new(),
            phoneme_id_map: PhonemeIdMap::new(),
            espeak_voice: "en-US".to_string(),
        }
    }

    #[test]
    fn overrides_fall_back_individually_to_defaults() {
        let v = voice(1);
        let overrides = InferenceOverrides {
            length_scale: Some(1.2),
            ..Default::default()
        };

        let resolved = v.resolve_inference_params(overrides);

        assert_eq!(resolved.length_scale, 1.2);
        assert_eq!(resolved.noise_scale, 0.667);
        assert_eq!(resolved.noise_w, 0.8);
    }

    #[test]
    fn speaker_id_is_none_for_single_speaker_voices_regardless_of_override() {
        let v = voice(1);
        let overrides = InferenceOverrides {
            speaker_id: Some(7),
            ..Default::default()
        };

        let resolved = v.resolve_inference_params(overrides);

        assert_eq!(resolved.speaker_id, None);
    }

    #[test]
    fn speaker_id_defaults_to_zero_for_multi_speaker_voices_when_unset() {
        let v = voice(2);

        let resolved = v.resolve_inference_params(InferenceOverrides::default());

        assert_eq!(resolved.speaker_id, Some(0));
    }

    #[test]
    fn speaker_id_uses_the_override_when_set_on_a_multi_speaker_voice() {
        let v = voice(3);
        let overrides = InferenceOverrides {
            speaker_id: Some(2),
            ..Default::default()
        };

        let resolved = v.resolve_inference_params(overrides);

        assert_eq!(resolved.speaker_id, Some(2));
    }
}
