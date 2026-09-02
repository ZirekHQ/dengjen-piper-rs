//! Application use cases: orchestrate domain logic and port calls. Each use
//! case takes its port dependencies as constructor arguments (dependency
//! injection via plain references — no framework), so tests supply fakes
//! with no adapter crate needed.

pub mod load_voice;
pub mod phonemize;
pub mod synthesize;
