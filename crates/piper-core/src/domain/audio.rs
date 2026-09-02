/// The final product of a synthesis call: raw PCM audio paired with the
/// sample rate it was produced at.
#[derive(Debug, Clone, PartialEq)]
pub struct SynthesizedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}
