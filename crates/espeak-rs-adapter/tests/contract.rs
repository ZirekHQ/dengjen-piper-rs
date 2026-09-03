use dengjen_espeak_rs_adapter::EspeakRsPhonemizer;
use piper_core::ports::phonemizer::Phonemizer;

piper_core::phonemizer_contract_tests!(
    || Box::new(EspeakRsPhonemizer::default()) as Box<dyn Phonemizer>
);
