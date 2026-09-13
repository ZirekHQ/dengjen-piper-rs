#!/usr/bin/env python3
"""Generates the minimal synthetic .onnx fixtures used by ort-adapter's
real-model tests. Not run by CI — run manually and commit the output:

    python3 -m venv .venv && .venv/bin/pip install onnx==1.22.0
    .venv/bin/python crates/ort-adapter/tests/fixtures/generate_model.py
"""
import onnx
from onnx import helper, TensorProto


def build_model(num_inputs: int, path: str) -> None:
    input_tensor = helper.make_tensor_value_info(
        "input", TensorProto.INT64, [1, "N"]
    )
    input_lengths = helper.make_tensor_value_info(
        "input_lengths", TensorProto.INT64, [1]
    )
    scales = helper.make_tensor_value_info("scales", TensorProto.FLOAT, [3])
    inputs = [input_tensor, input_lengths, scales]
    if num_inputs == 4:
        speaker_id = helper.make_tensor_value_info(
            "speaker_id", TensorProto.INT64, [1]
        )
        inputs.append(speaker_id)

    cast_node = helper.make_node(
        "Cast", ["input"], ["input_as_float"], to=TensorProto.FLOAT
    )
    output = helper.make_tensor_value_info(
        "output", TensorProto.FLOAT, [1, "N"]
    )

    graph = helper.make_graph(
        [cast_node],
        "synthetic-fixture",
        inputs,
        [output],
    )
    graph.node[0].output[0] = "output"

    model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", 17)])
    model.ir_version = 9
    onnx.checker.check_model(model)
    onnx.save(model, path)


if __name__ == "__main__":
    build_model(3, "crates/ort-adapter/tests/fixtures/model.onnx")
    build_model(4, "crates/ort-adapter/tests/fixtures/model_multi_speaker.onnx")
