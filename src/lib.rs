mod model;

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use ort::session::Session;
use ort_adapter::OrtInferenceEngine;
use piper_core::domain::errors::{InferenceError, PhonemizationError};
use piper_core::domain::inference::InferenceOverrides;
use piper_core::domain::phoneme::encode_phonemes;
use piper_core::domain::voice::Voice;
use piper_core::ports::inference_engine::InferenceEngine;
use piper_core::ports::phonemizer::{Phonemizer, Sentence};

use model::model_config_to_voice;
pub use model::{AudioConfig, ESpeakConfig, InferenceConfig, ModelConfig};
#[allow(deprecated)] // re-export; the deprecation itself is intentional
pub use model::{BOS, EOS, PAD, phonemes_to_ids};

#[derive(Debug)]
pub enum PiperError {
    FailedToLoadResource(String),
    PhonemizationError(String),
    InferenceError(String),
}

impl std::fmt::Display for PiperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToLoadResource(msg) => write!(f, "Failed to load resource: {}", msg),
            Self::PhonemizationError(msg) => write!(f, "Phonemization error: {}", msg),
            Self::InferenceError(msg) => write!(f, "Inference error: {}", msg),
        }
    }
}

impl std::error::Error for PiperError {}

pub type PiperResult<T> = Result<T, PiperError>;

const INTERNAL_VOICE_ID: &str = "piper";

#[cfg(feature = "espeak-ng")]
const PIPER_ESPEAKNG_DATA_DIRECTORY: &str = "PIPER_ESPEAKNG_DATA_DIRECTORY";
#[cfg(feature = "espeak-ng")]
const ESPEAKNG_DATA_DIR_NAME: &str = "espeak-ng-data";

#[cfg(feature = "espeak-ng")]
fn locate_espeak_ng_data_dir() -> Option<std::path::PathBuf> {
    let dir = std::env::var(PIPER_ESPEAKNG_DATA_DIRECTORY).ok()?;
    let candidate = std::path::PathBuf::from(dir).join(ESPEAKNG_DATA_DIR_NAME);
    candidate.is_dir().then_some(candidate)
}

#[cfg(feature = "espeak-ng")]
struct EspeakNgPhonemizer;

#[cfg(feature = "espeak-ng")]
impl Phonemizer for EspeakNgPhonemizer {
    fn phonemize(&self, text: &str, voice: &str) -> Result<Vec<Sentence>, PhonemizationError> {
        let translator = espeak_ng::Translator::new(voice, locate_espeak_ng_data_dir().as_deref())
            .map_err(|e| PhonemizationError::BackendFailure(e.to_string()))?;
        let ipa = translator
            .text_to_ipa(text)
            .map_err(|e| PhonemizationError::BackendFailure(e.to_string()))?;
        Ok(vec![Sentence(ipa)])
    }
}

fn build_phonemizer() -> Box<dyn Phonemizer> {
    #[cfg(feature = "espeak-rs")]
    {
        Box::new(espeak_rs_adapter::EspeakRsPhonemizer::default())
    }

    #[cfg(feature = "espeak-ng")]
    {
        Box::new(EspeakNgPhonemizer)
    }

    #[cfg(all(feature = "espeak-rs", feature = "espeak-ng"))]
    {
        compile_error!("Only one of `espeak-rs` or `espeak-ng` can be enabled at a time")
    }

    #[cfg(not(any(feature = "espeak-rs", feature = "espeak-ng")))]
    {
        compile_error!("One of `espeak-rs` or `espeak-ng` must be enabled")
    }
}

pub struct Piper {
    voice: Voice,
    engine: OrtInferenceEngine,
    phonemizer: Box<dyn Phonemizer>,
}

impl Piper {
    pub fn new(model_path: &Path, config_path: &Path) -> PiperResult<Self> {
        let file = File::open(config_path).map_err(|e| {
            PiperError::FailedToLoadResource(format!(
                "Failed to open config `{}`: {}",
                config_path.display(),
                e
            ))
        })?;
        let config: ModelConfig = serde_json::from_reader(file).map_err(|e| {
            PiperError::FailedToLoadResource(format!("Failed to parse config: {}", e))
        })?;
        let voice = model_config_to_voice(INTERNAL_VOICE_ID, &config);
        let engine = OrtInferenceEngine::new(model_path, voice.audio.sample_rate).map_err(|e| {
            PiperError::FailedToLoadResource(format!(
                "Failed to load model `{}`: {}",
                model_path.display(),
                e
            ))
        })?;
        Ok(Self {
            voice,
            engine,
            phonemizer: build_phonemizer(),
        })
    }

    pub fn from_session(session: Session, config: ModelConfig) -> Self {
        let voice = model_config_to_voice(INTERNAL_VOICE_ID, &config);
        let engine = OrtInferenceEngine::from_session(session, voice.audio.sample_rate);
        Self {
            voice,
            engine,
            phonemizer: build_phonemizer(),
        }
    }

    pub fn create(
        &mut self,
        text: &str,
        is_phonemes: bool,
        speaker_id: Option<i64>,
        length_scale: Option<f32>,
        noise_scale: Option<f32>,
        noise_w: Option<f32>,
    ) -> PiperResult<(Vec<f32>, u32)> {
        let sentences = if is_phonemes {
            vec![Sentence(text.to_string())]
        } else {
            self.phonemizer
                .phonemize(text, &self.voice.espeak_voice)
                .map_err(|e: PhonemizationError| PiperError::PhonemizationError(e.to_string()))?
        };
        let phonemes: String = sentences
            .into_iter()
            .map(|s| s.0)
            .collect::<Vec<_>>()
            .join(" ");
        let encoding = encode_phonemes(&self.voice.phoneme_id_map, &phonemes);

        let overrides = InferenceOverrides {
            speaker_id,
            length_scale,
            noise_scale,
            noise_w,
        };
        let params = self.voice.resolve_inference_params(overrides);

        let audio = self
            .engine
            .infer(&encoding.ids, params)
            .map_err(|e: InferenceError| PiperError::InferenceError(e.to_string()))?;

        Ok((audio.samples, audio.sample_rate))
    }

    pub fn voices(&self) -> Option<&HashMap<String, i64>> {
        self.voice.speakers()
    }
}

#[cfg(all(test, feature = "espeak-ng"))]
mod espeak_ng_data_dir_tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvVarGuard<'a> {
        _lock: std::sync::MutexGuard<'a, ()>,
        original: Option<std::ffi::OsString>,
    }

    impl Drop for EnvVarGuard<'_> {
        fn drop(&mut self) {
            match self.original.take() {
                Some(v) => unsafe { std::env::set_var(PIPER_ESPEAKNG_DATA_DIRECTORY, v) },
                None => unsafe { std::env::remove_var(PIPER_ESPEAKNG_DATA_DIRECTORY) },
            }
        }
    }

    fn set_env_var(dir: &std::path::Path) -> EnvVarGuard<'static> {
        let lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let original = std::env::var_os(PIPER_ESPEAKNG_DATA_DIRECTORY);
        unsafe { std::env::set_var(PIPER_ESPEAKNG_DATA_DIRECTORY, dir) };
        EnvVarGuard {
            _lock: lock,
            original,
        }
    }

    fn unset_env_var() -> EnvVarGuard<'static> {
        let lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let original = std::env::var_os(PIPER_ESPEAKNG_DATA_DIRECTORY);
        unsafe { std::env::remove_var(PIPER_ESPEAKNG_DATA_DIRECTORY) };
        EnvVarGuard {
            _lock: lock,
            original,
        }
    }

    fn with_env_var<T>(dir: &std::path::Path, f: impl FnOnce() -> T) -> T {
        let _guard = set_env_var(dir);
        f()
    }

    #[test]
    fn returns_none_when_env_var_unset() {
        let _guard = unset_env_var();
        assert_eq!(locate_espeak_ng_data_dir(), None);
    }

    #[test]
    fn returns_none_when_directory_has_no_espeak_ng_data_subdir() {
        let tmp = std::env::temp_dir().join("piper-rs-test-no-data-29");
        std::fs::create_dir_all(&tmp).unwrap();

        let result = with_env_var(&tmp, locate_espeak_ng_data_dir);

        std::fs::remove_dir_all(&tmp).ok();
        assert_eq!(result, None);
    }

    #[test]
    fn returns_none_when_espeak_ng_data_subdir_is_a_regular_file() {
        let tmp = std::env::temp_dir().join("piper-rs-test-file-not-dir-29");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join(ESPEAKNG_DATA_DIR_NAME), b"not a directory").unwrap();

        let result = with_env_var(&tmp, locate_espeak_ng_data_dir);

        std::fs::remove_dir_all(&tmp).ok();
        assert_eq!(result, None);
    }

    #[test]
    fn resolves_espeak_ng_data_subdir_of_env_var_like_espeak_rs_backend() {
        let tmp = std::env::temp_dir().join("piper-rs-test-with-data-29");
        let data_dir = tmp.join(ESPEAKNG_DATA_DIR_NAME);
        std::fs::create_dir_all(&data_dir).unwrap();

        let result = with_env_var(&tmp, locate_espeak_ng_data_dir);

        std::fs::remove_dir_all(&tmp).ok();
        assert_eq!(result, Some(data_dir));
    }
}
