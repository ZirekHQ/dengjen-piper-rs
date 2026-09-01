## Gotchas

### Link failed on Windows:

If you encounter linking errors such as

```console
error LNK2019: unresolved external symbol __std_mismatch_1 referenced in function "private: class onnxruntime::common::Status
```

Please make sure your visual studio is >= 17.11 (Update through Visual studio installer)

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
cargo install piper-rs-cli --git https://github.com/thewh1teagle/piper-rs
```
