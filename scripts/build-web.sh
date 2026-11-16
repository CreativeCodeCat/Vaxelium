#!/usr/bin/env bash
# Builds the web client: the desktop client compiled for the browser, with the files the chat server
# serves it from (server/web/dist). Needs the wasm32 target (on Arch, the rust-wasm package) and
# wasm-bindgen-cli at the same version as in Cargo.lock; wasm-opt (binaryen) makes it smaller.
set -euo pipefail
cd "$(dirname "$0")/.."

out=server/web/dist
wasm=target/wasm32-unknown-unknown/web/client.wasm
bindgen=$(command -v wasm-bindgen || echo "$HOME/.cargo/bin/wasm-bindgen")

cargo build -p client --profile web --target wasm32-unknown-unknown
rm -rf "$out"
mkdir -p "$out"
"$bindgen" --target web --no-typescript --out-dir "$out" --out-name client "$wasm"
if command -v wasm-opt >/dev/null; then
    wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int -o "$out/client_bg.wasm" "$out/client_bg.wasm"
fi
cp server/web/index.html "$out/index.html"
# Compressed copies, which the server sends to browsers that accept them.
gzip -k -f -9 "$out/client_bg.wasm" "$out/client.js"
# Emoji artwork is fetched as it is needed, so it is served as files rather than built in.
cp -r client/ui/emoji "$out/emoji"
echo "Built the web client in $out: $(du -h "$out/client_bg.wasm" | cut -f1) of WebAssembly, $(du -h "$out/client_bg.wasm.gz" | cut -f1) compressed."
