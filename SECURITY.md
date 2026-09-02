# Security Policy

## Reporting a vulnerability

Report suspected vulnerabilities privately via
[GitHub Private Vulnerability Reporting](https://github.com/ZirekHQ/dengjen-piper-rs/security/advisories/new)
(Security tab → Report a vulnerability). Do not open a public issue for a suspected
vulnerability.

Include, where possible: the affected crate/version, a minimal reproduction, and the impact
(memory safety, DoS, information disclosure, etc.).

## Scope

dengjen-piper-rs embeds espeak-ng (vendored as a git submodule in `crates/espeak-rs-sys`) and
onnxruntime (via the `ort` crate) through FFI. `unsafe` code in `crates/espeak-rs-sys` and
`crates/espeak-rs` is in scope, as is any panic or undefined behavior reachable from malformed
ONNX voice models or espeak-ng dictionary/config input. Resource-exhaustion reports against
large-but-well-formed inputs are lower priority.

## Supported versions

This project does not yet maintain parallel release branches — security fixes land on `main`.
