use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceLoadError {
    NotFound(String),
    MalformedConfig(String),
    ModelLoadFailure(String),
    IoFailure(String),
}

impl fmt::Display for VoiceLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "voice not found: {id}"),
            Self::MalformedConfig(msg) => write!(f, "malformed voice config: {msg}"),
            Self::ModelLoadFailure(msg) => write!(f, "failed to load model: {msg}"),
            Self::IoFailure(msg) => write!(f, "voice config i/o failure: {msg}"),
        }
    }
}

impl std::error::Error for VoiceLoadError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhonemizationError {
    Timeout,
    QueueFull,
    BackendFailure(String),
}

impl fmt::Display for PhonemizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(f, "phonemization timed out"),
            Self::QueueFull => write!(f, "phonemization queue is full"),
            Self::BackendFailure(msg) => write!(f, "phonemizer backend failure: {msg}"),
        }
    }
}

impl std::error::Error for PhonemizationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceError {
    ArityMismatch { expected: usize, actual: usize },
    RuntimeFailure(String),
}

impl fmt::Display for InferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArityMismatch { expected, actual } => write!(
                f,
                "model expects {expected} input tensors but voice config implies {actual}"
            ),
            Self::RuntimeFailure(msg) => write!(f, "inference failed: {msg}"),
        }
    }
}

impl std::error::Error for InferenceError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SynthesizeError {
    VoiceNotFound(String),
    Phonemization(PhonemizationError),
    Inference(InferenceError),
}

impl fmt::Display for SynthesizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VoiceNotFound(id) => write!(f, "voice not found: {id}"),
            Self::Phonemization(e) => write!(f, "{e}"),
            Self::Inference(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SynthesizeError {}

impl From<PhonemizationError> for SynthesizeError {
    fn from(e: PhonemizationError) -> Self {
        Self::Phonemization(e)
    }
}

impl From<InferenceError> for SynthesizeError {
    fn from(e: InferenceError) -> Self {
        Self::Inference(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesize_error_converts_from_phonemization_error() {
        let err: SynthesizeError = PhonemizationError::QueueFull.into();
        assert_eq!(
            err,
            SynthesizeError::Phonemization(PhonemizationError::QueueFull)
        );
    }

    #[test]
    fn synthesize_error_converts_from_inference_error() {
        let err: SynthesizeError = InferenceError::ArityMismatch {
            expected: 4,
            actual: 3,
        }
        .into();
        assert_eq!(
            err,
            SynthesizeError::Inference(InferenceError::ArityMismatch {
                expected: 4,
                actual: 3
            })
        );
    }

    #[test]
    fn arity_mismatch_display_names_both_counts() {
        let err = InferenceError::ArityMismatch {
            expected: 4,
            actual: 3,
        };
        assert_eq!(
            err.to_string(),
            "model expects 4 input tensors but voice config implies 3"
        );
    }
}
