#!/usr/bin/env bash
# Bumps every hand-synced copy of the workspace version to the given value.
#
# Cargo.toml's [workspace.package].version is the source of truth for the 6
# actual workspace members (they all use `version.workspace = true`).
# crates/espeak-rs-sys is deliberately NOT a workspace member (see the
# comment above [workspace.package] in Cargo.toml) and can't consume
# `version.workspace = true`, so its Cargo.toml is hand-synced here instead
# -- same pattern this script's tashkeel counterpart uses for
# bindings/java/build.gradle.kts.
#
# Usage: scripts/bump-version.sh 0.3.0
set -euo pipefail

new_version="${1:?usage: scripts/bump-version.sh <new-version>}"
if ! echo "$new_version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "::error::'${new_version}' doesn't look like a semver version (X.Y.Z)" >&2
  exit 1
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

old_version="$(awk '/^\[workspace\.package\]/{f=1;next} /^\[/{f=0} f && /^version = /{gsub(/version = "|"/,""); print; exit}' Cargo.toml)"
if [ -z "$old_version" ]; then
  echo "::error::Couldn't find [workspace.package].version in Cargo.toml" >&2
  exit 1
fi

sed -i.bak "0,/^version = \"${old_version}\"\$/s//version = \"${new_version}\"/" Cargo.toml
rm -f Cargo.toml.bak

sed -i.bak "0,/^version = \"${old_version}\"\$/s//version = \"${new_version}\"/" crates/espeak-rs-sys/Cargo.toml
rm -f crates/espeak-rs-sys/Cargo.toml.bak

# Every internal cross-crate dependency (piper-rs -> espeak-rs, espeak-rs ->
# espeak-rs-sys, the adapters -> piper-core, etc.) pins a `version`
# requirement floor alongside its `path`, required by `cargo publish` for
# any real (non-dev) path dependency. Unlike tashkeel's equivalent script,
# these floors DO need bumping every release here, not just on a real API
# break: every crate in this repo is still pre-1.0, where Cargo's caret
# requirement treats the minor version the way post-1.0 treats major (`^0.2.0`
# means `>=0.2.0, <0.3.0`) -- so bumping the shared version to 0.3.0 while
# these floors stay at "0.2.0" makes `cargo check` below (and any real
# `cargo publish`) fail outright with "no matching package" for every
# internal dependency. Restricted to lines that also name a `dengjen-*`
# package so this can't touch an unrelated third-party dependency whose
# version happens to equal the same string.
for manifest in Cargo.toml crates/*/Cargo.toml; do
  sed -i.bak -E "/package = \"dengjen-/ s/version = \"${old_version}\"/version = \"${new_version}\"/" "$manifest"
  rm -f "${manifest}.bak"
done

# Cargo.lock pins each workspace member's own version (matched via --locked
# in several CI steps, e.g. cargo publish), so it goes stale the moment
# Cargo.toml's version changes. cargo check only re-resolves entries that
# are actually inconsistent with Cargo.toml -- since nothing here changed
# any external dependency's version constraint, this touches only the
# workspace-local package entries, not third-party deps. Not --offline: a
# fresh CI runner (e.g. prepare-release.yml) has no pre-warmed registry
# index, and --offline would fail outright rather than just fetch it.
cargo check --quiet

echo "Bumped ${old_version} -> ${new_version}:"
git diff --stat -- Cargo.toml Cargo.lock crates/*/Cargo.toml
