use std::collections::HashMap;
use std::path::PathBuf;

use piper_core::domain::errors::VoiceLoadError;
use piper_core::domain::phoneme::PhonemeIdMap;
use piper_core::domain::voice::{AudioConfig, InferenceDefaults, Voice};
use piper_core::ports::voice_repository::VoiceRepository;
use serde::Deserialize;

#[derive(Deserialize)]
struct RawModelConfig {
    audio: RawAudioConfig,
    espeak: RawEspeakConfig,
    inference: RawInferenceConfig,
    num_speakers: u32,
    speaker_id_map: HashMap<String, i64>,
    phoneme_id_map: PhonemeIdMap,
}

#[derive(Deserialize)]
struct RawAudioConfig {
    sample_rate: u32,
}

#[derive(Deserialize)]
struct RawEspeakConfig {
    voice: String,
}

#[derive(Deserialize)]
struct RawInferenceConfig {
    noise_scale: f32,
    length_scale: f32,
    noise_w: f32,
}

fn raw_config_to_voice(voice_id: &str, raw: RawModelConfig) -> Voice {
    Voice {
        voice_id: voice_id.to_string(),
        audio: AudioConfig {
            sample_rate: raw.audio.sample_rate,
        },
        inference_defaults: InferenceDefaults {
            noise_scale: raw.inference.noise_scale,
            length_scale: raw.inference.length_scale,
            noise_w: raw.inference.noise_w,
        },
        num_speakers: raw.num_speakers,
        speaker_map: raw.speaker_id_map,
        phoneme_id_map: raw.phoneme_id_map,
        espeak_voice: raw.espeak.voice,
    }
}

pub struct FsVoiceRepository {
    base_dir: PathBuf,
}

impl FsVoiceRepository {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

fn is_safe_voice_id(voice_id: &str) -> bool {
    !voice_id.contains('/')
        && !voice_id.contains('\\')
        && matches!(
            std::path::Path::new(voice_id)
                .components()
                .collect::<Vec<_>>()
                .as_slice(),
            [std::path::Component::Normal(_)]
        )
}

impl VoiceRepository for FsVoiceRepository {
    fn load(&self, voice_id: &str) -> Result<Voice, VoiceLoadError> {
        if !is_safe_voice_id(voice_id) {
            return Err(VoiceLoadError::NotFound(voice_id.to_string()));
        }
        let config_path = self.base_dir.join(format!("{voice_id}.onnx.json"));
        let file = std::fs::File::open(&config_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                VoiceLoadError::NotFound(voice_id.to_string())
            } else {
                VoiceLoadError::IoFailure(e.to_string())
            }
        })?;
        let raw: RawModelConfig = serde_json::from_reader(file)
            .map_err(|e| VoiceLoadError::MalformedConfig(e.to_string()))?;
        Ok(raw_config_to_voice(voice_id, raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &std::path::Path, voice_id: &str, contents: &str) {
        std::fs::write(dir.join(format!("{voice_id}.onnx.json")), contents).unwrap();
    }

    #[test]
    fn rejects_an_absolute_voice_id_instead_of_escaping_base_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = FsVoiceRepository::new(tmp.path());

        let result = repo.load("/etc/passwd");

        assert_eq!(
            result,
            Err(VoiceLoadError::NotFound("/etc/passwd".to_string()))
        );
    }

    #[test]
    fn rejects_a_voice_id_containing_dot_dot_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = FsVoiceRepository::new(tmp.path());

        let result = repo.load("../../../etc/passwd");

        assert_eq!(
            result,
            Err(VoiceLoadError::NotFound("../../../etc/passwd".to_string()))
        );
    }

    #[test]
    fn rejects_a_voice_id_with_a_trailing_separator() {
        assert!(!is_safe_voice_id("foo/"));
        assert!(!is_safe_voice_id("foo//"));
    }

    #[test]
    fn rejects_a_voice_id_containing_a_path_separator() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = FsVoiceRepository::new(tmp.path());

        let result = repo.load("sub/voice");

        assert_eq!(
            result,
            Err(VoiceLoadError::NotFound("sub/voice".to_string()))
        );
    }

    #[test]
    fn returns_not_found_for_a_missing_config_file() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = FsVoiceRepository::new(tmp.path());

        let result = repo.load("does-not-exist");

        assert_eq!(
            result,
            Err(VoiceLoadError::NotFound("does-not-exist".to_string()))
        );
    }

    #[test]
    fn returns_malformed_config_for_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        write_config(tmp.path(), "bad-voice", "{ not valid json");
        let repo = FsVoiceRepository::new(tmp.path());

        let result = repo.load("bad-voice");

        assert!(matches!(result, Err(VoiceLoadError::MalformedConfig(_))));
    }

    #[test]
    #[cfg(unix)]
    fn returns_io_failure_for_a_config_file_unreadable_due_to_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        write_config(tmp.path(), "locked-voice", "{}");
        let config_path = tmp.path().join("locked-voice.onnx.json");
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let running_as_root = std::fs::File::open(&config_path).is_ok();

        let result = if running_as_root {
            None
        } else {
            let repo = FsVoiceRepository::new(tmp.path());
            Some(repo.load("locked-voice"))
        };

        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o644)).unwrap();
        if let Some(result) = result {
            assert!(matches!(result, Err(VoiceLoadError::IoFailure(_))));
        }
    }

    #[test]
    fn loads_a_single_speaker_voice_from_a_valid_config() {
        let tmp = tempfile::tempdir().unwrap();
        write_config(
            tmp.path(),
            "test-voice",
            r#"{
                "audio": { "sample_rate": 22050 },
                "espeak": { "voice": "en-US" },
                "inference": { "noise_scale": 0.667, "length_scale": 1.0, "noise_w": 0.8 },
                "num_speakers": 1,
                "speaker_id_map": {},
                "phoneme_id_map": { "a": [10] }
            }"#,
        );
        let repo = FsVoiceRepository::new(tmp.path());

        let voice = repo.load("test-voice").unwrap();

        assert_eq!(voice.voice_id, "test-voice");
        assert_eq!(voice.audio.sample_rate, 22050);
        assert_eq!(voice.espeak_voice, "en-US");
        assert_eq!(voice.num_speakers, 1);
        assert_eq!(voice.inference_defaults.noise_scale, 0.667);
        assert_eq!(voice.phoneme_id_map.get(&'a'), Some(&vec![10i64]));
        assert_eq!(voice.speakers(), None);
    }

    #[test]
    fn loads_a_multi_speaker_voice_with_named_speakers() {
        let tmp = tempfile::tempdir().unwrap();
        write_config(
            tmp.path(),
            "multi-voice",
            r#"{
                "audio": { "sample_rate": 16000 },
                "espeak": { "voice": "en-GB" },
                "inference": { "noise_scale": 0.5, "length_scale": 1.1, "noise_w": 0.7 },
                "num_speakers": 2,
                "speaker_id_map": { "alice": 0, "bob": 1 },
                "phoneme_id_map": {}
            }"#,
        );
        let repo = FsVoiceRepository::new(tmp.path());

        let voice = repo.load("multi-voice").unwrap();

        assert_eq!(voice.num_speakers, 2);
        let speakers = voice
            .speakers()
            .expect("multi-speaker voice should return Some");
        assert_eq!(speakers.get("alice"), Some(&0i64));
        assert_eq!(speakers.get("bob"), Some(&1i64));
    }
}
