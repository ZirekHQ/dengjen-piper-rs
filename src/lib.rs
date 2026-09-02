mod model;

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use ort::session::Session;

use model::infer;
pub use model::{AudioConfig, ESpeakConfig, InferenceConfig, ModelConfig};
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

#[cfg(feature = "espeak-ng")]
const PIPER_ESPEAKNG_DATA_DIRECTORY: &str = "PIPER_ESPEAKNG_DATA_DIRECTORY";
#[cfg(feature = "espeak-ng")]
const ESPEAKNG_DATA_DIR_NAME: &str = "espeak-ng-data";

/// Mirrors `espeak-rs`'s own resolution of `PIPER_ESPEAKNG_DATA_DIRECTORY`
/// (see `crates/espeak-rs/src/lib.rs`): the variable names the directory that
/// *contains* `espeak-ng-data`, not the data directory itself. Returns `None`
/// when the var is unset or doesn't point at a real `espeak-ng-data`
/// directory, so the caller can fall back to `espeak-ng`'s own default
/// resolution (`ESPEAK_DATA_PATH`, exe-relative, cwd-relative, `/usr/share`).
#[cfg(feature = "espeak-ng")]
fn locate_espeak_ng_data_dir() -> Option<std::path::PathBuf> {
    let dir = std::env::var(PIPER_ESPEAKNG_DATA_DIRECTORY).ok()?;
    let candidate = std::path::PathBuf::from(dir).join(ESPEAKNG_DATA_DIR_NAME);
    candidate.is_dir().then_some(candidate)
}

#[cfg(feature = "espeak-ng")]
fn phonemize_espeak_ng(voice: &str, text: &str) -> PiperResult<String> {
    let translator = espeak_ng::Translator::new(voice, locate_espeak_ng_data_dir().as_deref())
        .map_err(|e| PiperError::PhonemizationError(format!("{}", e)))?;
    translator
        .text_to_ipa(text)
        .map_err(|e| PiperError::PhonemizationError(format!("{}", e)))
}

/// Owns a loaded ONNX Runtime session for one voice model.
///
/// There is no separate "unload" call: `Piper` releases the underlying
/// ONNX Runtime session (and the native memory backing it) as soon as it is
/// dropped, like any other Rust value. To unload a model on demand — e.g. to
/// swap voices in a long-running app — hold it behind an `Option<Piper>` (or
/// similar) and assign `None` to it; see `examples/unload_model.rs`.
pub struct Piper {
    config: ModelConfig,
    session: Session,
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
        let session = Session::builder()
            .map_err(|e| {
                PiperError::FailedToLoadResource(format!("Failed to create session builder: {}", e))
            })?
            .commit_from_file(model_path)
            .map_err(|e| {
                PiperError::FailedToLoadResource(format!(
                    "Failed to load model `{}`: {}",
                    model_path.display(),
                    e
                ))
            })?;
        Ok(Self { config, session })
    }

    pub fn from_session(session: Session, config: ModelConfig) -> Self {
        Self { session, config }
    }

    /// Synthesize speech from text or phonemes.
    ///
    /// Returns `(samples, sample_rate)` where samples are f32 PCM audio.
    pub fn create(
        &mut self,
        text: &str,
        is_phonemes: bool,
        speaker_id: Option<i64>,
        length_scale: Option<f32>,
        noise_scale: Option<f32>,
        noise_w: Option<f32>,
    ) -> PiperResult<(Vec<f32>, u32)> {
        let phonemes = if is_phonemes {
            text.to_string()
        } else {
            #[cfg(feature = "espeak-rs")]
            {
                espeak_rs::text_to_phonemes(text, &self.config.espeak.voice, None)
                    .map_err(|e| PiperError::PhonemizationError(format!("{}", e)))?
                    .join(" ")
            }

            #[cfg(feature = "espeak-ng")]
            {
                phonemize_espeak_ng(&self.config.espeak.voice, text)?
            }

            #[cfg(all(feature = "espeak-rs", feature = "espeak-ng"))]
            {
                compile_error!("Only one of `espeak-rs` or `espeak-ng` can be enabled at a time")
            }

            #[cfg(not(any(feature = "espeak-rs", feature = "espeak-ng")))]
            {
                compile_error!("One of `espeak-rs` or `espeak-ng` must be enabled")
            }
        };

        let inf = &self.config.inference;
        let samples = infer(
            &mut self.session,
            &self.config,
            &phonemes,
            noise_scale.unwrap_or(inf.noise_scale),
            length_scale.unwrap_or(inf.length_scale),
            noise_w.unwrap_or(inf.noise_w),
            speaker_id.unwrap_or(0),
        )?;

        Ok((samples, self.config.audio.sample_rate))
    }

    /// Returns the speaker name→id map, or `None` for single-speaker models.
    pub fn voices(&self) -> Option<&HashMap<String, i64>> {
        if self.config.speaker_id_map.is_empty() {
            None
        } else {
            Some(&self.config.speaker_id_map)
        }
    }
}

#[cfg(all(test, feature = "espeak-ng"))]
mod espeak_ng_data_dir_tests {
    use super::*;
    use std::sync::Mutex;

    // `locate_espeak_ng_data_dir` reads a process-global env var; serialize
    // the tests that touch it so they can't observe each other's value, and
    // restore whatever value (if any) preceded the test so a var inherited
    // from the outer environment survives the test run.
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
