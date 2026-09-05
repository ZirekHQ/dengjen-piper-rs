use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

pub const BOS: char = '^';
pub const EOS: char = '$';
pub const PAD: char = '_';

pub type PhonemeIdMap = HashMap<char, Vec<i64>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhonemeIdSequence(pub Vec<i64>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sentinel {
    Bos,
    Pad,
    Eos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhonemizationWarning {
    UnmappedPhoneme { phoneme: char },
    MissingSentinel { sentinel: Sentinel },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhonemeEncoding {
    pub ids: PhonemeIdSequence,
    pub warnings: Vec<PhonemizationWarning>,
}

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
        let m = map([
            (BOS, vec![1]),
            (PAD, vec![0]),
            (EOS, vec![2]),
            ('a', vec![10, 11]),
        ]);

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
        let m = map([
            (BOS, vec![1]),
            (PAD, vec![0]),
            (EOS, vec![2]),
            ('a', vec![10]),
        ]);

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
                PhonemizationWarning::MissingSentinel {
                    sentinel: Sentinel::Bos
                },
                PhonemizationWarning::MissingSentinel {
                    sentinel: Sentinel::Pad
                },
                PhonemizationWarning::MissingSentinel {
                    sentinel: Sentinel::Eos
                },
            ]
        );
    }

    #[test]
    fn normalizes_composed_phonemes_to_nfd_before_lookup() {
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
