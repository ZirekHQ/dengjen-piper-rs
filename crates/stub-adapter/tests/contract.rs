use dengjen_stub_adapter::StubPhonemizer;
use piper_core::ports::phonemizer::Phonemizer;

piper_core::phonemizer_contract_tests!(|| Box::new(StubPhonemizer::new()) as Box<dyn Phonemizer>);
