#!/usr/bin/env bash
set -euo pipefail

# Regression guard for #23: espeak-ng's build silently drops dictsource/extra/ru_listx from the
# compiled data, degrading Russian pronunciation despite EXTRA_ru being on by default. The Russian
# dictionary built with the extra list is about 9 MB; without it, it is far smaller.
min_bytes=5000000

# A cached target/ can hold dictionaries from earlier builds. Run this right after the build, so
# the newest one is the current build's.
dict="$(find target -path '*dengjen-espeak-rs-sys-*/out/share/espeak-ng-data/ru_dict' -printf '%T@ %p\n' | sort -nr | head -n1 | cut -d' ' -f2-)"
if [ -z "$dict" ]; then
  echo "::error::no built ru_dict found under target/; build dengjen-espeak-rs-sys first" >&2
  exit 1
fi

size=$(wc -c < "$dict")
if [ "$size" -lt "$min_bytes" ]; then
  echo "::error::${dict} is ${size} bytes, under ${min_bytes}; the Russian extra dictionary ru_listx was left out of the build (see #23)" >&2
  exit 1
fi
echo "${dict} is ${size} bytes"
