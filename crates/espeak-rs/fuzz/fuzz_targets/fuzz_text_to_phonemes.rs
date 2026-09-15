#![no_main]

use libfuzzer_sys::fuzz_target;

// lang_index must come before text: fuzz_target! decodes via arbitrary_take_rest,
// which only applies to a derived struct's last field -- text needs the full
// remaining buffer, not a length-prefixed slice of it, or seed files stop
// decoding into the text they were written to contain.
#[derive(arbitrary::Arbitrary, Debug)]
struct Input {
    lang_index: u8,
    text: String,
}

const LANGUAGES: [&str; 4] = ["en-US", "ar", "de", "es"];

fuzz_target!(|input: Input| {
    let language = LANGUAGES[input.lang_index as usize % LANGUAGES.len()];
    let _ = dengjen_espeak_rs::text_to_phonemes(&input.text, language, None);
});
