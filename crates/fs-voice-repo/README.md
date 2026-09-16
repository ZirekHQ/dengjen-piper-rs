# Filesystem Voice Repository

Implements piper-core's `VoiceRepository` port by loading a voice's `model_config` JSON from a directory on disk and mapping it into the `Voice` domain type (audio config, inference defaults, phoneme ID map, speaker map). Rejects voice IDs that contain path separators or otherwise escape the base directory.
