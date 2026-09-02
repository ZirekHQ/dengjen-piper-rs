//! `piper-core`: domain model, use cases, and port traits for the
//! dengjen-piper-rs TTS platform. This crate has no dependency on any
//! transport (HTTP/gRPC), any native phonemizer, or `ort` — those live in
//! adapter crates that depend on this one, never the reverse.

pub mod domain;
pub mod ports;
pub mod registry;
pub mod use_cases;
