# eSpeak NG Bindings

Safe Rust wrapper around `dengjen-espeak-rs-sys`, guarding eSpeak-ng's global C state behind a mutex and exposing `text_to_phonemes` with a timeout so a hung call surfaces as an error instead of blocking forever. Used by `dengjen-espeak-rs-adapter` to back piper-core's `Phonemizer` port.

## Dependencies

- [espeak-ng](https://github.com/espeak-ng/espeak-ng)
