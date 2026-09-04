



use dengjen_piper_rs::Piper;
use std::path::Path;

struct TtsState {
    piper: Option<Piper>,
}

impl TtsState {
    fn load(&mut self, onnx_path: &Path, config_path: &Path) {
        self.piper = Some(Piper::new(onnx_path, config_path).unwrap());
    }

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
