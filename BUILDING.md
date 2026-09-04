## Building a distributable binary

`dengjen-espeak-rs-sys`'s build script compiles eSpeak NG's data files (dictionaries,
voices, phoneme tables) and, since it needs *some* directory to build into,
generates them inside `target/<profile>/build/dengjen-espeak-rs-sys-<hash>/out/...` —
an ephemeral, cache-keyed path that disappears the moment `target/` is
deleted or the crate is rebuilt with a different hash. Copying only the
compiled binary out of `target/` and discarding the rest (e.g. `cargo build
--release` followed by `rm -rf target`) breaks it at runtime with:

```text
Failed to initialize eSpeak-ng. Try setting `PIPER_ESPEAKNG_DATA_DIRECTORY`
to the directory that contains the `espeak-ng-data` directory.
```

To make this easy, the build script also copies `espeak-ng-data` to
`target/<profile>/espeak-ng-data`, right next to the binary itself. At
runtime, `dengjen-espeak-rs` looks for `espeak-ng-data` (in this order) in:

1. the directory named by the `PIPER_ESPEAKNG_DATA_DIRECTORY` env var,
2. the current working directory,
3. the directory containing the running executable.

So for a plain `cargo build --release` / `cargo run`, it just works — no env
var needed, since (3) already finds `target/release/espeak-ng-data`.

To ship a binary elsewhere, copy both the binary and its `espeak-ng-data`
directory together and keep them side by side:

```console
cargo build --release
mkdir -p dist
cp target/release/<your-binary> dist/
cp -r target/release/espeak-ng-data dist/
```

If your packaging can't keep them side by side (e.g. the data directory
belongs in a shared system location), set `PIPER_ESPEAKNG_DATA_DIRECTORY` at
runtime to the directory that *contains* `espeak-ng-data`. See issue #10.

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

Maintainers only. `Cargo.toml`'s `[workspace.package].version` is the single source of truth
every published crate tracks in lockstep — 6 of the 7 via `version.workspace = true`;
`crates/espeak-rs-sys` isn't a workspace member (see the comment above `[workspace.package]`), so
it's hand-synced instead, by the same script that does everything else below. There's no more
manually bumping individual `Cargo.toml` files.

1. Run the **Prepare release** workflow (`workflow_dispatch`, from the Actions tab). It computes
   the next semver version from Conventional Commit subjects merged since the last `vX.Y.Z` tag
   (`fix:`/etc → patch, `feat:` → minor, `!`/`BREAKING CHANGE:` → major, only docs/chore/style/
   refactor/test since the last tag means no release) and opens a PR bumping every hand-synced
   copy of it (`scripts/next-version.sh` / `scripts/bump-version.sh`).

   First run only: `next-version.sh` needs a prior `vX.Y.Z` tag to diff Conventional Commits
   from, and fails loudly rather than guessing when none exists — this repo has never tagged a
   release, so push a `v<current-workspace-version>` tag once, manually, to bootstrap it.

2. Merge that PR.

3. Push a `vX.Y.Z` tag at the merge commit (e.g. `git tag v0.3.0 && git push origin v0.3.0`).
   Nothing currently triggers off this tag directly — unlike sibling repos, there's no GitHub
   Release / cargo-dist step here, since piper-rs is a library with no prebuilt binaries to
   distribute. It exists purely as the baseline step 1's next run diffs Conventional Commits
   from.

4. Push a **separate** `publish-N` tag (e.g. `publish-6`) — triggers the
   [`publish` workflow](.github/workflows/publish.yml), which publishes all 7 crates in
   dependency order: `dengjen-espeak-rs-sys` + `dengjen-piper-core` (no internal deps) →
   `dengjen-espeak-rs` + `dengjen-stub-adapter` + `dengjen-fs-voice-repo` + `dengjen-ort-adapter`
   (each needs one tier-1 crate) → `dengjen-espeak-rs-adapter` + `dengjen-piper-rs` (need tier-2
   crates), waiting for each tier to land on the crates.io index before the next tier — which
   depends on it — publishes. The `publish-*` tag pattern is deliberately different from step
   3's `vX.Y.Z`: it's the retry mechanism if a partial publish fails, since `cargo publish` on a
   version crates.io already has is a hard failure, not a no-op — `publish.yml` skips a crate
   it finds already live, so re-pushing a new `publish-N` tag after a partial failure is safe.

Note: Please don't create PR from your main branch. only from new feature branch!

## Install piper-rs-cli from Git

```console
cargo install piper-rs-cli --git https://github.com/ZirekHQ/dengjen-piper-rs
```
