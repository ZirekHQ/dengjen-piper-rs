use dengjen_fs_voice_repo::FsVoiceRepository;
use piper_core::ports::voice_repository::VoiceRepository;

fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

piper_core::voice_repository_contract_tests!(
    || FsVoiceRepository::new(fixtures_dir()),
    "fr_FR-tom-medium"
);
