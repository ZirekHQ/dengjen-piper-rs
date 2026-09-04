#!/usr/bin/env bash
set -euo pipefail

# Regression guard for #23: espeak-ng's data.cmake wires
# dictsource/extra/ru_listx into the Russian dictionary automatically
# (EXTRA_ru option, default ON) whenever compile-espeak-intonations is
# enabled, but the espeak-ng data bundling can silently drop that file
# from what actually gets published, degrading Russian pronunciation for
# anyone building against the published crate.
#
# Since #66, the raw espeak-ng/dictsource/** files no longer appear
# directly in Cargo.toml's `include`: they're pre-compressed into a single
# bundled/espeak-ng.tar.xz (crates.io's 10MiB .crate cap forced this --
# see crates/espeak-rs-sys/scripts/bundle-espeak-ng.sh), so the check now
# has to look inside that generated bundle instead of `cargo package
# --list`'s flat file list.
sys_crate_dir="crates/espeak-rs-sys"
bundle="$sys_crate_dir/bundled/espeak-ng.tar.xz"
needle="dictsource/extra/ru_listx"

if [ ! -f "$bundle" ]; then
  bash "$sys_crate_dir/scripts/bundle-espeak-ng.sh"
fi

listing=$(cargo package --list --allow-dirty -p dengjen-espeak-rs-sys)
if ! grep -qxF "bundled/espeak-ng.tar.xz" <<<"$listing"; then
  echo "::error::bundled/espeak-ng.tar.xz is missing from the dengjen-espeak-rs-sys package file list; the published crate would ship without any espeak-ng data at all" >&2
  exit 1
fi

if ! xz -dc "$bundle" | tar -tf - | grep -qxF "$needle"; then
  echo "::error::${needle} is missing from ${bundle}; the published crate would ship without the Russian extra dictionary (see #23)" >&2
  exit 1
fi

echo "${needle} is present in ${bundle}, and the bundle is part of the dengjen-espeak-rs-sys package file list"
