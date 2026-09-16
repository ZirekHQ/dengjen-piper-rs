# ONNX Runtime Inference Adapter

Implements piper-core's `InferenceEngine` port on top of the `ort` ONNX Runtime bindings: builds the VITS model's input tensors (phoneme IDs, input lengths, noise/length/noise-w scales, and an optional speaker ID) and runs them through a loaded `.onnx` session to produce synthesized audio.
