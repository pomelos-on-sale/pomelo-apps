#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

echo "=== Building all Pomelo OS apps to WebAssembly ==="
cargo build --workspace --exclude pomelo-apps --target wasm32-unknown-unknown --release

OUT_DIR="$DIR/apps-dist"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

cat << 'EOF' > "$OUT_DIR/apps.toml"
default_app = "app-launcher"
EOF

APPS=(
    "app-launcher:app_launcher.wasm"
    "calculator:calculator.wasm"
    "counter:counter.wasm"
    "hello:hello.wasm"
    "music-player:music_player.wasm"
    "settings:settings.wasm"
    "terminal:terminal.wasm"
)

for item in "${APPS[@]}"; do
    app="${item%%:*}"
    wasm_name="${item##*:}"
    echo "Bundling $app ($wasm_name)..."
    mkdir -p "$OUT_DIR/$app"
    cp "apps/$app/manifest.toml" "$OUT_DIR/$app/manifest.toml"
    cp "target/wasm32-unknown-unknown/release/$wasm_name" "$OUT_DIR/$app/$wasm_name"
done

echo "=== All apps bundled successfully into $OUT_DIR ==="
ls -lh "$OUT_DIR"/*
