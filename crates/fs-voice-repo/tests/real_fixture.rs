use dengjen_fs_voice_repo::FsVoiceRepository;
use piper_core::ports::voice_repository::VoiceRepository;

#[test]
fn loads_the_real_fr_fr_tom_medium_fixture() {
    let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let repo = FsVoiceRepository::new(fixtures_dir);

    let voice = repo
        .load("fr_FR-tom-medium")
        .expect("real fixture should parse");

    assert_eq!(voice.voice_id, "fr_FR-tom-medium");
    assert!(!voice.espeak_voice.is_empty());
    assert!(
        !voice.phoneme_id_map.is_empty(),
        "real fixture should carry a non-empty phoneme_id_map"
    );
}
