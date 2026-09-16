# Stub Phonemizer

A dependency-free `Phonemizer` port implementation for tests: it recognizes a small fixed set of voices (`en-US`, `en-GB`, `fr-FR`) and splits input text into sentences on `.`, `?`, and `!` without doing any real phonemization. Lets consumers of piper-core exercise the phonemize/synthesize pipeline without linking espeak-ng.
