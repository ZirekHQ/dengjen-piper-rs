## Gotchas

### Link failed on Windows:

If you encounter linking errors such as

```console
error LNK2019: unresolved external symbol __std_mismatch_1 referenced in function "private: class onnxruntime::common::Status
```

Please make sure your visual studio is >= 17.11 (Update through Visual studio installer)

## Cross-compiling for Linux arm64 (e.g. Raspberry Pi)

```console
sudo apt-get install crossbuild-essential-arm64 pkg-config libssl-dev
rustup target add aarch64-unknown-linux-gnu
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  cargo build --release --target aarch64-unknown-linux-gnu
```

No `CMAKE_TOOLCHAIN_FILE`, sysroot variable, or separately-built `libasound`
is needed — the `cmake`/`cc` crates already recognize the standard
`aarch64-linux-gnu-*` cross-compiler naming convention that
`crossbuild-essential-arm64` installs, and espeak-ng's own CMake build skips
its tests and native-only intonation data when `CMAKE_CROSSCOMPILING` is set.
`pkg-config`/`libssl-dev` are only needed on the host, for the `openssl-sys`
build script (a build-time dependency of `ort`), not for the arm64 target
itself. See issue #16 and the `linux-arm64` CI job for a verified recipe.

## Publish new version

Bump the version in the three `Cargo.toml` files (root `piper-rs`, `crates/espeak-rs`,
`crates/espeak-rs-sys` — they're kept in lockstep), merge to `main`, then push a tag matching
`publish-*` (e.g. `git tag publish-0.3.0 && git push origin publish-0.3.0`). The
[`publish` workflow](.github/workflows/publish.yml) publishes espeak-rs-sys, waits for it to
land on the crates.io index, then espeak-rs, waits again, then piper-rs — each crate depends on
the version of the previous one just published, so publishing out of order or without waiting
will fail to resolve.

Note: Please don't create PR from your main branch. only from new feature branch!

## Install piper-rs-cli from Git

```console
cargo install piper-rs-cli --git https://github.com/ZirekHQ/dengjen-piper-rs
```
