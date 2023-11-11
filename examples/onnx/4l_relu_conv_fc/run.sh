cargo run gen-settings -M network.onnx --param-visibility public --input-visibility private --output-visibility private
cargo run compile-circuit -M network.onnx -S settings.json --compiled-circuit network.ezkl
cargo run gen-witness -M network.ezkl -D input.json
cargo run generate-program -M network.ezkl --witness witness.json