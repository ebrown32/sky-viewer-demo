#!/usr/bin/env bash
set -euo pipefail

if (($# > 1)); then
    echo "Usage: $0 [--debug|--release]" >&2
    exit 2
fi

trunk_args=()
case "${1:---debug}" in
    --debug) ;;
    --release) trunk_args+=(--release) ;;
    *)
        echo "Usage: $0 [--debug|--release]" >&2
        exit 2
        ;;
esac

CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
export PATH="$CARGO_BIN:$PATH"

if ! command -v trunk >/dev/null 2>&1; then
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Trunk is not installed and Cargo is unavailable; install Rust/Cargo first." >&2
        exit 1
    fi

    echo "Trunk not found; installing it with Cargo..." >&2
    cargo install --locked trunk
fi

if ! command -v rustc >/dev/null 2>&1; then
    echo "Rust is not installed; install rustc and cargo first." >&2
    exit 1
fi

target_libdir="$(rustc --print target-libdir --target wasm32-unknown-unknown)"
packages=()
if [[ ! -d "$target_libdir" ]]; then
    packages+=(libstd-rust-dev-wasm32)
fi
if ! command -v rust-lld >/dev/null 2>&1 && ! command -v lld >/dev/null 2>&1; then
    packages+=(lld)
fi

if ((${#packages[@]})); then
    if ! command -v apt-get >/dev/null 2>&1; then
        echo "Missing WASM build dependencies (${packages[*]}); apt-get is unavailable." >&2
        exit 1
    fi

    if [[ "$EUID" -eq 0 ]]; then
        apt-get update
        DEBIAN_FRONTEND=noninteractive apt-get install -y "${packages[@]}"
    elif command -v sudo >/dev/null 2>&1 && sudo -n true; then
        sudo -n apt-get update
        sudo -n env DEBIAN_FRONTEND=noninteractive apt-get install -y "${packages[@]}"
    else
        echo "Missing WASM build dependencies (${packages[*]}); install them with apt-get." >&2
        exit 1
    fi
fi

if ! command -v rust-lld >/dev/null 2>&1 && command -v lld >/dev/null 2>&1; then
    mkdir -p "$CARGO_BIN"
    ln -sf "$(command -v lld)" "$CARGO_BIN/rust-lld"
    echo "Using system lld as rust-lld." >&2
fi

if ! command -v rust-lld >/dev/null 2>&1; then
    echo "rust-lld is unavailable after installing lld with apt-get." >&2
    exit 1
fi

exec trunk serve --address 0.0.0.0 --port 8080 "${trunk_args[@]}"
