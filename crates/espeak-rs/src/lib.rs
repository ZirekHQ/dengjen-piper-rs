use std::env;
use std::ffi::{CStr, CString, c_char, c_void};
use std::mem;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const PIPER_ESPEAKNG_DATA_DIRECTORY: &str = "PIPER_ESPEAKNG_DATA_DIRECTORY";
const ESPEAKNG_DATA_DIR_NAME: &str = "espeak-ng-data";

#[derive(Debug, Clone)]
pub enum ESpeakError {
    Timeout(String),
    Failure(String),
}

impl ESpeakError {
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }
}

impl std::error::Error for ESpeakError {}

impl std::fmt::Display for ESpeakError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout(msg) => write!(f, "eSpeak-ng error: {msg}"),
            Self::Failure(msg) => write!(f, "eSpeak-ng error: {msg}"),
        }
    }
}

pub type ESpeakResult<T> = Result<T, ESpeakError>;

static ESPEAK_LOCK: Mutex<bool> = Mutex::new(false);

const PHONEMIZATION_TIMEOUT: Duration = Duration::from_secs(5);

fn init_espeak() -> ESpeakResult<()> {
    let data_dir = locate_espeak_data();
    let path_cstr = data_dir
        .as_ref()
        .and_then(|p| CString::new(p.to_string_lossy().as_ref()).ok());
    let path_ptr = path_cstr.as_ref().map_or(ptr::null(), |c| c.as_ptr());

    let sample_rate = unsafe {
        espeak_rs_sys::espeak_Initialize(
            espeak_rs_sys::espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_RETRIEVAL,
            0,
            path_ptr,
            espeak_rs_sys::espeakINITIALIZE_DONT_EXIT as i32,
        )
    };

    if sample_rate <= 0 {
        Err(ESpeakError::Failure(format!(
            "Failed to initialize eSpeak-ng (code {sample_rate}). \
            Try setting `{PIPER_ESPEAKNG_DATA_DIRECTORY}` to the directory containing `{ESPEAKNG_DATA_DIR_NAME}`."
        )))
    } else {
        Ok(())
    }
}

fn acquire_or_fail<T>(lock: &Mutex<T>) -> ESpeakResult<std::sync::MutexGuard<'_, T>> {
    lock.lock().map_err(|_| {
        ESpeakError::Failure(
            "espeak-ng lock poisoned by a prior panic; refusing to touch possibly-corrupted state"
                .into(),
        )
    })
}

fn ensure_initialized(
    initialized: &mut bool,
    init: impl FnOnce() -> ESpeakResult<()>,
) -> ESpeakResult<()> {
    if *initialized {
        return Ok(());
    }
    init()?;
    *initialized = true;
    Ok(())
}

fn compiled_in_data_dir() -> Option<PathBuf> {
    let dir = espeak_rs_sys::ESPEAK_NG_DATA_DIR?;
    let p = PathBuf::from(dir);
    p.join(ESPEAKNG_DATA_DIR_NAME).exists().then_some(p)
}

fn locate_espeak_data() -> Option<PathBuf> {
    if let Ok(dir) = env::var(PIPER_ESPEAKNG_DATA_DIRECTORY) {
        let p = PathBuf::from(dir);
        if p.join(ESPEAKNG_DATA_DIR_NAME).exists() {
            return Some(p);
        }
    }
    if let Some(dir) = compiled_in_data_dir() {
        return Some(dir);
    }
    if let Ok(cwd) = env::current_dir()
        && cwd.join(ESPEAKNG_DATA_DIR_NAME).exists()
    {
        return Some(cwd);
    }
    if let Ok(exe) = env::current_exe()
        && let Some(dir) = exe.parent()
        && dir.join(ESPEAKNG_DATA_DIR_NAME).exists()
    {
        return Some(dir.to_path_buf());
    }
    None
}

fn check_deadline(elapsed: Duration, timeout: Duration, language: &str) -> ESpeakResult<()> {
    if elapsed > timeout {
        Err(ESpeakError::Timeout(format!(
            "Timed out phonemizing `{language}` text after {timeout:?}"
        )))
    } else {
        Ok(())
    }
}

fn cursor_advanced(prev_text_ptr: *const c_char, text_ptr: *const c_char) -> bool {
    text_ptr != prev_text_ptr
}

const CLAUSE_INTONATION_MASK: i32 = 0x7000;
const CLAUSE_INTONATION_FULL_STOP: i32 = 0x0000;
const CLAUSE_INTONATION_COMMA: i32 = 0x1000;
const CLAUSE_INTONATION_QUESTION: i32 = 0x2000;
const CLAUSE_INTONATION_EXCLAMATION: i32 = 0x3000;
const CLAUSE_TYPE_SENTENCE: i32 = 0x80000;

fn terminator_char(terminator: i32) -> Option<char> {
    let is_sentence = terminator & CLAUSE_TYPE_SENTENCE != 0;
    match terminator & CLAUSE_INTONATION_MASK {
        CLAUSE_INTONATION_FULL_STOP if is_sentence => Some('.'),
        CLAUSE_INTONATION_FULL_STOP => Some(':'),
        CLAUSE_INTONATION_COMMA => Some(','),
        CLAUSE_INTONATION_QUESTION => Some('?'),
        CLAUSE_INTONATION_EXCLAMATION => Some('!'),
        _ => None,
    }
}

fn strip_lang_switches(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth: usize = 0;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

pub fn text_to_phonemes(
    text: &str,
    language: &str,
    phoneme_separator: Option<char>,
) -> ESpeakResult<Vec<String>> {
    let mut initialized = acquire_or_fail(&ESPEAK_LOCK)?;

    ensure_initialized(&mut initialized, init_espeak)?;

    let lang_cstr = CString::new(language)
        .map_err(|_| ESpeakError::Failure("Language name contains a null byte".into()))?;
    let set_voice = unsafe { espeak_rs_sys::espeak_SetVoiceByName(lang_cstr.as_ptr()) };
    if set_voice != espeak_rs_sys::espeak_ERROR_EE_OK {
        return Err(ESpeakError::Failure(format!(
            "Failed to set voice: `{language}`"
        )));
    }

    let phoneme_mode = match phoneme_separator {
        Some(c) => ((c as u32) << 8) | espeak_rs_sys::espeakINITIALIZE_PHONEME_IPA,
        None => espeak_rs_sys::espeakINITIALIZE_PHONEME_IPA,
    } as i32;

    let mut sentences: Vec<String> = Vec::new();
    let mut current = String::new();
    let started_at = Instant::now();

    for line in text.lines() {
        let text_cstr = CString::new(line)
            .map_err(|_| ESpeakError::Failure("Text contains a null byte".into()))?;

        let mut text_ptr: *const c_char = text_cstr.as_ptr();

        while !text_ptr.is_null() {
            check_deadline(started_at.elapsed(), PHONEMIZATION_TIMEOUT, language)?;

            let prev_text_ptr = text_ptr;
            let text_ptr_slot = &mut text_ptr as *mut *const c_char as *mut *const c_void;
            let mut terminator: i32 = 0;
            let terminator_ptr: *mut i32 = &mut terminator;

            // A segfault/abort() in this C call takes down the whole process; can't be caught from Rust.
            let res = unsafe {
                espeak_rs_sys::espeak_TextToPhonemesWithTerminator(
                    text_ptr_slot,
                    espeak_rs_sys::espeakCHARS_UTF8 as i32,
                    phoneme_mode,
                    terminator_ptr,
                )
            };

            check_deadline(started_at.elapsed(), PHONEMIZATION_TIMEOUT, language)?;

            if res.is_null() {
                if !cursor_advanced(prev_text_ptr, text_ptr) {
                    return Err(ESpeakError::Failure(format!(
                        "espeak_TextToPhonemes returned NULL without advancing past `{language}` text (stuck clause cursor)"
                    )));
                }
                continue;
            }

            let clause = unsafe { CStr::from_ptr(res).to_string_lossy().into_owned() };
            current.push_str(&strip_lang_switches(&clause));
            if let Some(c) = terminator_char(terminator) {
                current.push(c);
            }

            if terminator & CLAUSE_TYPE_SENTENCE != 0 && !current.is_empty() {
                sentences.push(mem::take(&mut current));
            }

            if !cursor_advanced(prev_text_ptr, text_ptr) {
                return Err(ESpeakError::Failure(format!(
                    "espeak_TextToPhonemes did not advance past `{language}` text (stuck clause cursor)"
                )));
            }
        }

        if !current.is_empty() {
            sentences.push(mem::take(&mut current));
        }
    }

    Ok(sentences)
}

#[cfg(test)]
mod espeak_error_tests {
    use super::*;

    #[test]
    fn is_timeout_true_only_for_the_timeout_variant() {
        assert!(ESpeakError::Timeout("timed out".to_string()).is_timeout());
        assert!(!ESpeakError::Failure("boom".to_string()).is_timeout());
    }
}

#[cfg(test)]
mod deadline_tests {
    use super::*;

    #[test]
    fn errs_when_elapsed_exceeds_timeout() {
        let result = check_deadline(Duration::from_secs(6), Duration::from_secs(5), "en-US");
        assert!(matches!(result, Err(ESpeakError::Timeout(_))));
    }

    #[test]
    fn ok_when_elapsed_is_within_timeout() {
        let result = check_deadline(Duration::from_secs(4), Duration::from_secs(5), "en-US");
        assert!(result.is_ok());
    }

    #[test]
    fn ok_when_elapsed_equals_timeout_exactly() {
        let result = check_deadline(Duration::from_secs(5), Duration::from_secs(5), "en-US");
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod acquire_or_fail_tests {
    use super::*;

    #[test]
    fn ok_when_the_lock_is_not_poisoned() {
        let lock = Mutex::new(0);
        assert!(acquire_or_fail(&lock).is_ok());
    }

    #[test]
    fn errs_instead_of_recovering_a_poisoned_lock() {
        let lock = Mutex::new(0);
        let poison_result = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _guard = lock.lock().unwrap();
                    panic!("simulated corruption while holding the lock");
                })
                .join()
        });
        assert!(poison_result.is_err());

        let result = acquire_or_fail(&lock);
        assert!(matches!(result, Err(ESpeakError::Failure(_))));
    }
}

#[cfg(test)]
mod ensure_initialized_tests {
    use super::*;

    #[test]
    fn retries_after_a_previous_init_failure() {
        let mut initialized = false;

        let first = ensure_initialized(&mut initialized, || {
            Err(ESpeakError::Failure("boom".to_string()))
        });
        assert!(first.is_err());
        assert!(!initialized);

        let second = ensure_initialized(&mut initialized, || Ok(()));
        assert!(second.is_ok());
        assert!(initialized);
    }

    #[test]
    fn does_not_reinitialize_once_successful() {
        let mut initialized = true;
        let mut calls = 0;

        let result = ensure_initialized(&mut initialized, || {
            calls += 1;
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(calls, 0);
    }
}

#[cfg(test)]
mod compiled_in_data_dir_tests {
    use super::*;

    #[test]
    fn returns_a_path_when_the_compiled_in_dir_actually_has_the_data() {
        // espeak-rs-sys's build script always produces a real data dir in a normal workspace build.
        assert!(compiled_in_data_dir().is_some());
    }
}

#[cfg(test)]
mod locate_espeak_data_tests {
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
                Some(v) => unsafe { env::set_var(PIPER_ESPEAKNG_DATA_DIRECTORY, v) },
                None => unsafe { env::remove_var(PIPER_ESPEAKNG_DATA_DIRECTORY) },
            }
        }
    }

    fn set_env_var(dir: &std::path::Path) -> EnvVarGuard<'static> {
        let lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let original = env::var_os(PIPER_ESPEAKNG_DATA_DIRECTORY);
        unsafe { env::set_var(PIPER_ESPEAKNG_DATA_DIRECTORY, dir) };
        EnvVarGuard {
            _lock: lock,
            original,
        }
    }

    #[test]
    fn env_var_wins_over_the_compiled_in_data_dir_when_both_resolve() {
        // Must actually resolve here, or this test would pass for the wrong reason (env var only).
        assert!(compiled_in_data_dir().is_some());

        let override_dir = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(override_dir.path().join(ESPEAKNG_DATA_DIR_NAME))
            .expect("create espeak-ng-data subdir");

        let _guard = set_env_var(override_dir.path());

        assert_eq!(
            locate_espeak_data(),
            Some(override_dir.path().to_path_buf())
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT_ALICE: &str =
        "Who are you? said the Caterpillar. Replied Alice , rather shyly, I hardly know, sir!";

    #[test]
    fn test_basic_en() -> ESpeakResult<()> {
        let phonemes = text_to_phonemes("test", "en-US", None)?.join("");
        assert_eq!(phonemes, "tˈɛst.");
        Ok(())
    }

    #[test]
    fn test_it_splits_sentences() -> ESpeakResult<()> {
        let phonemes = text_to_phonemes(TEXT_ALICE, "en-US", None)?;
        assert_eq!(phonemes.len(), 3);
        Ok(())
    }

    #[test]
    fn test_it_adds_phoneme_separator() -> ESpeakResult<()> {
        let phonemes = text_to_phonemes("test", "en-US", Some('_'))?.join("");
        assert_eq!(phonemes, "t_ˈɛ_s_t.");
        Ok(())
    }

    #[test]
    fn test_it_preserves_clause_breakers() -> ESpeakResult<()> {
        let phonemes = text_to_phonemes(TEXT_ALICE, "en-US", None)?.join("");
        for c in ['.', ',', '?', '!'] {
            assert!(phonemes.contains(c), "Clause breaker `{c}` not preserved");
        }
        Ok(())
    }

    #[test]
    fn test_arabic() -> ESpeakResult<()> {
        let phonemes = text_to_phonemes("مَرْحَبَاً بِكَ أَيُّهَا الْرَّجُلْ", "ar", None)?.join("");
        assert_eq!(phonemes, "mˈarħabˌaː bikˌa ʔaˈiuːhˌaː alrrdʒˈul.");
        Ok(())
    }

    #[test]
    fn test_lang_switch_markers_stripped() -> ESpeakResult<()> {
        let phonemes = text_to_phonemes("Hello معناها مرحباً", "ar", None)?.join("");
        assert!(!phonemes.contains("(en)"));
        assert!(!phonemes.contains("(ar)"));
        Ok(())
    }

    #[test]
    fn test_line_splitting() -> ESpeakResult<()> {
        let phonemes = text_to_phonemes("Hello\nThere\nAnd\nWelcome", "en-US", None)?;
        assert_eq!(phonemes.len(), 4);
        Ok(())
    }

    #[test]
    fn test_it_distinguishes_colon_from_period() -> ESpeakResult<()> {
        let phonemes = text_to_phonemes("Note: it works", "en-US", None)?.join("");
        assert!(phonemes.contains(':'), "Colon not preserved: {phonemes:?}");
        Ok(())
    }

    #[test]
    fn test_cursor_advanced() {
        let buf = *b"abc\0";
        let start: *const c_char = buf.as_ptr() as *const c_char;
        let advanced: *const c_char = unsafe { start.add(1) };

        assert!(cursor_advanced(start, advanced));
        assert!(!cursor_advanced(start, start));
    }

    #[test]
    fn test_concurrent_calls_do_not_crash() {
        use std::thread;

        let inputs: [(&str, &str); 4] = [
            ("Who are you? said the Caterpillar.", "en-US"),
            ("مَرْحَبَاً بِكَ أَيُّهَا الْرَّجُلْ", "ar"),
            ("Hello\nThere\nAnd\nWelcome", "en-US"),
            ("Replied Alice, rather shyly, I hardly know, sir!", "en-US"),
        ];

        let handles: Vec<_> = (0..16)
            .map(|i| {
                let (text, lang) = inputs[i % inputs.len()];
                thread::spawn(move || text_to_phonemes(text, lang, None))
            })
            .collect();

        for handle in handles {
            let result = handle.join().expect("worker thread panicked");
            assert!(result.is_ok(), "text_to_phonemes failed: {result:?}");
        }
    }
}
