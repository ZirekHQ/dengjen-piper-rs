#!/usr/bin/env bash
set -euo pipefail

# Regression guard for #23: espeak-ng's data.cmake wires
# dictsource/extra/ru_listx into the Russian dictionary automatically
# (EXTRA_ru option, default ON) whenever compile-espeak-intonations is
# enabled, but Cargo.toml's `include` list can silently drop that file
# from what actually gets published, degrading Russian pronunciation for
# anyone building against the published crate. `cargo package --list`
# reports exactly the file set that would ship, so assert the dictionary
# is part of it rather than relying on a local build (which reads
# straight from the source tree and can't observe this class of bug).
needle="espeak-ng/dictsource/extra/ru_listx"

listing=$(cargo package --list --allow-dirty -p dengjen-espeak-rs-sys)
if ! grep -qxF "$needle" <<<"$listing"; then
  echo "::error::${needle} is missing from the dengjen-espeak-rs-sys package file list; the published crate would ship without the Russian extra dictionary (see #23)" >&2
  exit 1
fi

echo "${needle} is present in the dengjen-espeak-rs-sys package file list"
