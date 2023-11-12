# Machine Learning zk-VM Compiler

## Setup

Install [Powdr](https://www.powdr.org/):
```bash
git clone git@github.com:powdr-labs/powdr.git
cd powdr
# write-once-memory branch needed for this project
git checkout write-once-memory
cargo install --path powdr_cli
```

## Compiling a VM

Go inside a directory with a `network.onnx` & `input.json`, for example:
- `examples/onnx/2l_relu_fc`
- `examples/onnx/4l_relu_conv_fc`

Run the following EZKL steps to prepare the network:
```bash
cargo run gen-settings -M network.onnx --param-visibility public --input-visibility private --output-visibility private
cargo run compile-circuit -M network.onnx -S settings.json --compiled-circuit network.ezkl
cargo run gen-witness -M network.ezkl -D input.json
```

Run the `generate-program` command added by this repository:
```bash
cargo run generate-program -M network.ezkl --witness witness.json
```

This will generate 2 files:
- `vm.asm`: The model-agnostic [Powdr VM](src/powdr_template.asm) with the model-specific assembly program inserted
- `memory.csv`: The content of the memory, specific to the input (from `input.json`)

These can be passed to Powdr to compile the VM and make a prove of the program:
```
powdr pil vm.asm -f --witness-values memory.csv --prove-with estark
```

(or without the `--prove-with estark` flag if you just want to check everything works)