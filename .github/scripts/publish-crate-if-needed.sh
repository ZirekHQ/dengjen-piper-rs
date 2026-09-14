#!/usr/bin/env bash
# Lets publish.yml retry a partial failure (one crate published, the next 403s) by
# re-running from scratch without cargo publish hard-failing on an already-published version.

# crates.io 403s any request without a descriptive User-Agent (bare "curl/x.y.z" doesn't
# qualify) -- every lookup here would fail before checking anything without one.

# curl -sf can't tell 404 (not published) from a transient 5xx -- both are just nonzero
# exit, so capture the real code and branch on a value actually seen.
set -euo pipefail

crate="${1:?usage: publish-crate-if-needed.sh <crate-name> <version>}"
version="${2:?usage: publish-crate-if-needed.sh <crate-name> <version>}"

status="$(curl -s -o /dev/null -w '%{http_code}' \
  -H "User-Agent: dengjen-piper-rs-publish-ci (https://github.com/ZirekHQ/dengjen-piper-rs)" \
  "https://crates.io/api/v1/crates/${crate}/${version}")"
# dengjen-espeak-rs-sys's Cargo.toml includes the gitignored bundled/espeak-ng.tar.xz --
# cargo's dirty-check flags it regardless, so only this crate needs --allow-dirty.
extra_args=()
if [ "$crate" = "dengjen-espeak-rs-sys" ]; then
  extra_args+=(--allow-dirty)
fi

case "$status" in
  200) echo "${crate} ${version} is already published -- skipping" ;;
  404)
    if [ "${DRY_RUN:-}" = "true" ]; then
      cargo publish -p "$crate" --locked --dry-run "${extra_args[@]}"
    else
      cargo publish -p "$crate" --locked "${extra_args[@]}"
    fi
    ;;
  *)
    echo "::error::Unexpected status ${status} checking crates.io for ${crate} ${version}" >&2
    exit 1
    ;;
esac
