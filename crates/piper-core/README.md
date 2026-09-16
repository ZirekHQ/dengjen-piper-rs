# Piper Core

Hexagonal core of dengjen-piper-rs: domain types (`Voice`, phoneme IDs, inference parameters, audio), port traits (`InferenceEngine`, `Phonemizer`, `VoiceRepository`), and the use cases (`load_voice`, `phonemize`, `synthesize`) that compose them, plus a `VoiceRegistry` for looking up loaded voices by ID. Adapter crates (ort-adapter, espeak-rs-adapter, stub-adapter, fs-voice-repo) implement its ports; this crate has no dependency on any of them.
