#!/usr/bin/env bash
# Runs every CI lane, continues past failures, and exits non-zero if any lane failed.
set -uo pipefail

cd "${WORKSPACE:-/workspace}"
git config --global --add safe.directory '*'
git config --global protocol.file.allow always

failed=()

lane() {
  local name=$1
  shift
  echo "::: lane: ${name}"
  if "$@"; then
    echo "::: PASS ${name}"
  else
    echo "::: FAIL ${name}"
    failed+=("${name}")
  fi
}

has_avx2() { grep -qw avx2 /proc/cpuinfo; }

# The prebuilt ONNX Runtime dies with SIGILL on CPUs without AVX2.
avx2_lane() {
  if has_avx2; then
    lane "$@"
  else
    echo "::: SKIP $1 (CPU lacks AVX2)"
  fi
}

lane fmt cargo fmt --all -- --check
lane clippy-lib-default cargo clippy --workspace --lib --bins --locked -- -D warnings
lane clippy-lib-espeak-ng cargo clippy --lib --bins --locked --no-default-features --features espeak-ng -- -D warnings
lane clippy-all-targets cargo clippy --workspace --all-targets --locked -- -D warnings

avx2_lane test-root cargo test --locked -p dengjen-piper-rs
avx2_lane test-root-espeak-ng cargo test --locked -p dengjen-piper-rs --no-default-features --features espeak-ng
lane test-espeak-rs cargo test --locked -p dengjen-espeak-rs -- --test-threads=1
lane test-espeak-rs-sys cargo test --locked -p dengjen-espeak-rs-sys --test build_script_unit_tests
lane test-piper-core cargo test --locked -p dengjen-piper-core
lane test-stub-adapter cargo test --locked -p dengjen-stub-adapter
lane test-fs-voice-repo cargo test --locked -p dengjen-fs-voice-repo
avx2_lane test-ort-adapter cargo test --locked -p dengjen-ort-adapter
lane test-espeak-rs-adapter cargo test --locked -p dengjen-espeak-rs-adapter -- --test-threads=1
lane ru-listx-packaged bash .github/scripts/check-ru-listx-packaged.sh

lane deny cargo deny check
lane audit cargo audit

echo
if ((${#failed[@]})); then
  echo "FAILED lanes: ${failed[*]}"
  exit 1
fi
echo "All lanes passed"
