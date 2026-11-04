#!/bin/sh
# WASM çekirdeğini derler ve web/pkg'ye yazar. Gerekenler: Rust (wasm32-unknown-unknown),
# wasm-bindgen-cli 0.2.130 (cargo install wasm-bindgen-cli --version 0.2.130).
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown -p nestio-wasm
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/release/nestio_wasm.wasm
cp examples/ornek-flans-braket.dxf web/ornek/
ls -la web/pkg
