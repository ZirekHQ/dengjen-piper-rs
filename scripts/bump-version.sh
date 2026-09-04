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
git diff --stat -- Cargo.toml Cargo.lock crates/espeak-rs-sys/Cargo.toml
