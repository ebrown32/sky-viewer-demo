#!/usr/bin/env bash
set -euo pipefail

if ! command -v trunk >/dev/null 2>&1; then
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Trunk is not installed and Cargo is unavailable; install Rust/Cargo first." >&2
        exit 1
    fi

    echo "Trunk not found; installing it with Cargo..." >&2
    cargo install --locked trunk
    export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
fi

exec trunk serve --address 0.0.0.0 --port 8080
