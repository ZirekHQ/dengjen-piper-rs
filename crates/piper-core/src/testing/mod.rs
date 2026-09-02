//! Port contract test macros. Adapter crates depend on `piper-core` as a
//! `[dev-dependencies]` entry and invoke these macros from their own test
//! modules to prove they satisfy each port's documented contract — the
//! same contract every other implementation (including `stub-adapter`) is
//! held to.

pub mod inference_engine_contract;
pub mod phonemizer_contract;
pub mod voice_repository_contract;
