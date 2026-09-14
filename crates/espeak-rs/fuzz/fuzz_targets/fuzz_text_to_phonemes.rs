#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(arbitrary::Arbitrary, Debug)]
struct Input {
    text: String,
    lang_index: u8,
}

const LANGUAGES: [&str; 4] = ["en-US", "ar", "de", "es"];

fuzz_target!(|input: Input| {
    let language = LANGUAGES[input.lang_index as usize % LANGUAGES.len()];
    let _ = dengjen_espeak_rs::text_to_phonemes(&input.text, language, None);
});
