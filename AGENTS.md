# AGENTS.md

Rust workspace that runs [Piper](https://github.com/OHF-Voice/piper1-gpl) TTS models. Edition 2024, MSRV 1.88, GPL-3.0-or-later.

## Architecture

Hexagonal. `piper-core` holds the domain, use cases, and port traits; adapters implement the ports. Dependencies point inward only: adapters depend on `piper-core`, never the reverse.

| Crate | Role |
|---|---|
| `crates/piper-core` | Domain model, use cases (`load_voice`, `phonemize`, `synthesize`), ports (`InferenceEngine`, `Phonemizer`, `VoiceRepository`), contract tests behind the `testing` feature |
| `crates/ort-adapter` | `InferenceEngine` over ONNX Runtime |
| `crates/espeak-rs-adapter` | `Phonemizer` over `espeak-rs` with a bounded worker queue |
| `crates/fs-voice-repo` | Filesystem `VoiceRepository` |
| `crates/stub-adapter` | Deterministic phonemizer test double |
| `crates/espeak-rs` | Safe wrapper over espeak-ng; owns all `unsafe` FFI calls |
| `crates/espeak-rs-sys` | bindgen bindings; vendors espeak-ng as a git submodule |
| `src/` | `dengjen-piper-rs` facade crate wiring the adapters together |

Architecture write-up: `docs/modules/ROOT/pages/architecture.adoc`.

## Build and test

Clone with `--recurse-submodules` (or `git submodule update --init`). System packages: `pkg-config libssl-dev clang libclang-dev llvm-dev espeak-ng libasound2-dev`.

```console
cargo test -p dengjen-piper-core
cargo test -p dengjen-piper-rs
cargo test -p dengjen-piper-rs --no-default-features --features espeak-ng
cargo test -p dengjen-espeak-rs -- --test-threads=1
cargo test -p dengjen-espeak-rs-adapter -- --test-threads=1
```

- espeak-ng keeps global voice-table state. Tests that touch it MUST run with `--test-threads=1`; the parallel runner segfaults.
- Features `espeak-rs` (default) and `espeak-ng` are mutually exclusive; test each separately.
- Run every CI lane in a container: `docker compose run --rm lanes` (`Dockerfile.lanes`, `scripts/run-lanes.sh`).

## Lint

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo audit
```

Clippy warnings fail CI. `deny.toml` restricts licenses, registries, and git sources.

## Conventions

- New behavior needs tests first, including FFI and `unsafe` paths. New adapters must pass the shared contract tests in `piper-core::testing`.
- Ports return `Result` with domain errors from `piper-core::domain::errors`. No `unwrap`/`expect` in non-test library code.
- `unsafe` stays in `espeak-rs`. Every espeak call goes through `ESPEAK_LOCK`.
- Commits use Conventional Commits (`feat:`, `fix:`, `docs:`, `ci:`, `chore(deps):`) with the PR number appended by the squash merge.
- GitHub Actions are pinned by commit SHA; `zizmor` runs in CI and pre-commit.
- `.cargo/config.toml` sets a macOS-only `BINDGEN_EXTRA_CLANG_ARGS`; override it with the env var on Linux if bindgen picks up the wrong headers.
