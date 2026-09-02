//! `dengjen-espeak-rs-adapter`: the real, production `Phonemizer`
//! implementation, wrapping `crates/espeak-rs`'s already-tested
//! `text_to_phonemes`. Owns the Approach-A bounded phonemization worker
//! queue (design §5) — see `worker_pool` for the concurrency design and
//! its rationale.

mod worker_pool;
