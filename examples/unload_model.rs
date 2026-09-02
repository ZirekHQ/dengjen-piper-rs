/*
Demonstrates unloading a model, e.g. to free memory before loading another
voice in a long-running app (GUI, server, ...).

`Piper` has no `unload()` method because it doesn't need one: dropping it
releases the ONNX Runtime session immediately, like any other Rust value.
Holding it behind `Option<Piper>` lets you unload on demand by assigning
`None`.

git submodule update --init

wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx
wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx.json
cargo run --example unload_model en_US-libritts_r-medium.onnx.json
*/

use dengjen_piper_rs::Piper;
use std::path::Path;

struct TtsState {
    piper: Option<Piper>,
}

impl TtsState {
    fn load(&mut self, onnx_path: &Path, config_path: &Path) {
        self.piper = Some(Piper::new(onnx_path, config_path).unwrap());
    }

    // Dropping the `Piper` here releases its ONNX Runtime session and the
    // native memory backing it — no separate unload call is needed.
    fn unload(&mut self) {
        self.piper = None;
    }
}

fn main() {
    let config_path = std::env::args().nth(1).expect("Please specify config path");
    let onnx_path = config_path.replace(".onnx.json", ".onnx");

    let mut state = TtsState { piper: None };

    state.load(Path::new(&onnx_path), Path::new(&config_path));
    println!("Model loaded.");

    state.unload();
    println!("Model unloaded — its ONNX Runtime session has been released.");
}
