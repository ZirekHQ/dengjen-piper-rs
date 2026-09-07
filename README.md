# dengjen-piper-rs

[![Crates](https://img.shields.io/crates/v/dengjen-piper-rs?logo=rust&color=F07B3C)](https://crates.io/crates/dengjen-piper-rs/)

[Project board](https://github.com/orgs/ZirekHQ/projects/1) — live roadmap and status for this repo's issues.

Use [Piper](https://github.com/OHF-Voice/piper1-gpl) TTS models in Rust.

## Features

-  Compatibility with all Piper TTS models
-  Support for multiple languages
-  High performance with pure Rust implementation

## Install

```console
cargo add dengjen-piper-rs
```

## Examples

See [examples](examples)

## Models

All pretrained models available at [huggingface.co/rhasspy/piper-voices](https://huggingface.co/rhasspy/piper-voices/tree/main)

## Unloading models

`Piper` has no `unload()` method because it doesn't need one: dropping a
`Piper` value releases its ONNX Runtime session, and the native memory
backing it, immediately. To unload a model on demand — e.g. to swap voices
in a long-running app — hold it behind an `Option<Piper>` and assign `None`
to it. See [`examples/unload_model.rs`](examples/unload_model.rs).

## Credits

This project is inspired by [sonata](https://github.com/mush42/sonata), originally created by [mush42](https://github.com/mush42).

---

## 💝 Support This Project

If this repository saves you time and effort, please consider supporting it!

- ⭐ [Star on GitHub](https://github.com/ZirekHQ/dengjen-piper-rs)
- 🐦 [Share on Twitter](https://twitter.com/intent/tweet?text=dengjen-piper-rs%20-%20Piper%20TTS%20models%20in%20Rust&url=https%3A%2F%2Fgithub.com%2FZirekHQ%2Fdengjen-piper-rs)
- 💖 [More ways to support](https://github.com/ZirekHQ) — Open Collective coming soon