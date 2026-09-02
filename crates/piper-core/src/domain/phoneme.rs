use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

pub const BOS: char = '^';
pub const EOS: char = '$';
pub const PAD: char = '_';

/// Training-vocabulary lookup table: phoneme character to one or more model
/// input ids. Owned by a `Voice`, populated from its model config.
pub type PhonemeIdMap = HashMap<char, Vec<i64>>;

/// The encoded tensor input derived from a `PhonemeIdMap` plus BOS/PAD/EOS
/// sentinels. Feeds directly into an `InferenceEngine` port implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhonemeIdSequence(pub Vec<i64>);

/// Which sentinel (BOS/PAD/EOS) a `PhonemizationWarning::MissingSentinel`
/// refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sentinel {
    Bos,
    Pad,
    Eos,
}

/// A non-fatal degradation encountered while encoding phonemes into ids.
/// Callers still get synthesized audio; this is attached alongside it so
/// the degradation is observable instead of silent (see AI_NATIVE_SPEC.md
/// §6 item 1 — resolved in the reimagine design as "return audio + a
/// warning", not a silent drop and not a hard failure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhonemizationWarning {
    /// A phoneme in the input had no entry in the voice's `PhonemeIdMap`
    /// and was omitted from the encoded sequence entirely.
    UnmappedPhoneme { phoneme: char },
    /// One of BOS/PAD/EOS had no entry in the voice's `PhonemeIdMap`; id
    /// `0` was substituted.
    MissingSentinel { sentinel: Sentinel },
}

/// Result of encoding a phonemized string into model input ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhonemeEncoding {
    pub ids: PhonemeIdSequence,
    pub warnings: Vec<PhonemizationWarning>,
}

/// Encodes `phonemes` into the model's expected id sequence: BOS ids, then
/// each phoneme's ids followed by PAD's ids, then EOS ids. A phoneme
/// mapping to more than one id has every id emitted, not just the first.
pub fn encode_phonemes(map: &PhonemeIdMap, phonemes: &str) -> PhonemeEncoding {
    let default_id = [0i64];
    let mut warnings = Vec::new();

    let mut sentinel_ids = |ch: char, sentinel: Sentinel| -> Vec<i64> {
        match map.get(&ch) {
            Some(ids) => ids.clone(),
            None => {
                warnings.push(PhonemizationWarning::MissingSentinel { sentinel });
                default_id.to_vec()
            }
        }
    };
    let bos_ids = sentinel_ids(BOS, Sentinel::Bos);
    let pad_ids = sentinel_ids(PAD, Sentinel::Pad);
    let eos_ids = sentinel_ids(EOS, Sentinel::Eos);

    let mut ids = Vec::with_capacity((phonemes.len() + 1) * 2);
    ids.extend_from_slice(&bos_ids);
    for ch in phonemes.nfd() {
        match map.get(&ch) {
            Some(phoneme_ids) => {
                ids.extend_from_slice(phoneme_ids);
                ids.extend_from_slice(&pad_ids);
            }
            None => warnings.push(PhonemizationWarning::UnmappedPhoneme { phoneme: ch }),
        }
    }
    ids.extend_from_slice(&eos_ids);

    PhonemeEncoding {
        ids: PhonemeIdSequence(ids),
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: impl IntoIterator<Item = (char, Vec<i64>)>) -> PhonemeIdMap {
        HashMap::from_iter(entries)
    }

    #[test]
    fn extends_every_id_in_a_multi_id_phoneme_mapping() {
        let m = map([(BOS, vec![1]), (PAD, vec![0]), (EOS, vec![2]), ('a', vec![10, 11])]);

        let encoding = encode_phonemes(&m, "a");

        assert_eq!(encoding.ids, PhonemeIdSequence(vec![1, 10, 11, 0, 2]));
        assert_eq!(encoding.warnings, vec![]);
    }

    #[test]
    fn inserts_pad_after_each_phoneme_but_not_after_bos() {
        let m = map([
            (BOS, vec![1]),
            (PAD, vec![0]),
            (EOS, vec![2]),
            ('a', vec![10]),
            ('b', vec![20]),
        ]);

        let encoding = encode_phonemes(&m, "ab");

        assert_eq!(encoding.ids, PhonemeIdSequence(vec![1, 10, 0, 20, 0, 2]));
        assert_eq!(encoding.warnings, vec![]);
    }

    #[test]
    fn drops_unmapped_phonemes_and_warns() {
        let m = map([(BOS, vec![1]), (PAD, vec![0]), (EOS, vec![2]), ('a', vec![10])]);

        let encoding = encode_phonemes(&m, "ab");

        assert_eq!(encoding.ids, PhonemeIdSequence(vec![1, 10, 0, 2]));
        assert_eq!(
            encoding.warnings,
            vec![PhonemizationWarning::UnmappedPhoneme { phoneme: 'b' }]
        );
    }

    #[test]
    fn defaults_missing_sentinels_to_zero_and_warns() {
        let m = map([('a', vec![5])]);

        let encoding = encode_phonemes(&m, "a");

        assert_eq!(encoding.ids, PhonemeIdSequence(vec![0, 5, 0, 0]));
        assert_eq!(
            encoding.warnings,
            vec![
                PhonemizationWarning::MissingSentinel { sentinel: Sentinel::Bos },
                PhonemizationWarning::MissingSentinel { sentinel: Sentinel::Pad },
                PhonemizationWarning::MissingSentinel { sentinel: Sentinel::Eos },
            ]
        );
    }

    #[test]
    fn normalizes_composed_phonemes_to_nfd_before_lookup() {
        // The model is trained on NFD (decomposed) Unicode, e.g. 'c' followed
        // by a combining cedilla (U+0327), but a phonemizer backend may emit
        // the composed form 'ç' (U+00E7) as a single codepoint. Without NFD
        // normalization that composed codepoint isn't in the map and would
        // be silently dropped (regression: legacy issue #12).
        let m = map([
            (BOS, vec![1]),
            (PAD, vec![0]),
            (EOS, vec![2]),
            ('c', vec![10]),
            ('\u{0327}', vec![11]),
        ]);

        let encoding = encode_phonemes(&m, "\u{00E7}");

        assert_eq!(encoding.ids, PhonemeIdSequence(vec![1, 10, 0, 11, 0, 2]));
        assert_eq!(encoding.warnings, vec![]);
    }
}
