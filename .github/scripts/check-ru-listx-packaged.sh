#!/usr/bin/env bash
set -euo pipefail

# Regression guard for #23: espeak-ng's build silently drops dictsource/extra/ru_listx
# from published data, degrading Russian pronunciation despite EXTRA_ru being on by default.

# Since #66, dictsource files are pre-compressed into bundled/espeak-ng.tar.xz (crates.io's
# 10MiB cap forced this), so this checks inside that bundle instead of a flat file list.
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
