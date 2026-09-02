//! Port traits: the boundary between `piper-core`'s use cases and every
//! concrete adapter (native phonemizer, ONNX runtime, filesystem, ...).
//! Adapter crates depend on this module; this module depends on nothing
//! outside `crate::domain`.

pub mod phonemizer;
