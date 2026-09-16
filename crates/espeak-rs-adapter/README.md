# eSpeak Phonemizer Adapter

Implements piper-core's `Phonemizer` port on top of `dengjen-espeak-rs`, dispatching every call to a single dedicated worker thread over a bounded queue, since espeak-ng's global C state can only be driven by one thread at a time. The queue rejects new work with an error instead of blocking once it's full.
