/*
git submodule update --init

wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx
wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx.json
cargo run --example usage en_US-libritts_r-medium.onnx.json 80
*/

use piper_rs::Piper;
use rodio::buffer::SamplesBuffer;
use std::num::NonZero;
use std::path::Path;

fn main() {
    let config_path = std::env::args().nth(1).expect("Please specify config path");
    let speaker_id: Option<i64> = std::env::args()
        .nth(2)
        .map(|s| s.parse().expect("Speaker ID must be a number"));

    let onnx_path = config_path.replace(".onnx.json", ".onnx");
    let mut piper = Piper::new(Path::new(&onnx_path), Path::new(&config_path)).unwrap();

    let text = "Hello! I'm playing audio from memory directly with piper-rs.";
    let (samples, sample_rate) = piper
        .create(text, false, speaker_id, None, None, None)
        .unwrap();

    let device = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
    let player = rodio::Player::connect_new(device.mixer());
    let channels = NonZero::new(1).unwrap();
    let sample_rate = NonZero::new(sample_rate).unwrap();
    player.append(SamplesBuffer::new(channels, sample_rate, samples));
    player.sleep_until_end();
}
