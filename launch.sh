#!/usr/bin/env bash
set -euo pipefail

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

if command -v rustup >/dev/null 2>&1 &&
    ! rustup target list --installed | grep -Fxq wasm32-unknown-unknown; then
    echo "Installing the wasm32-unknown-unknown Rust target..." >&2
    rustup target add wasm32-unknown-unknown
fi

if ! command -v rust-lld >/dev/null 2>&1; then
    if command -v rustup >/dev/null 2>&1; then
        echo "Installing Rust's LLVM tools for the missing rust-lld linker..." >&2
        rustup component add llvm-tools-preview

        sysroot="$(rustc --print sysroot)"
        for linker in "$sysroot"/lib/rustlib/*/bin/rust-lld; do
            if [[ -x "$linker" ]]; then
                export PATH="$(dirname "$linker"):$PATH"
                break
            fi
        done
    fi
fi

if ! command -v rust-lld >/dev/null 2>&1; then
    if ! command -v apt-get >/dev/null 2>&1; then
        echo "rust-lld is missing; install Rust's llvm-tools-preview component or system lld." >&2
        exit 1
    fi

    if ! command -v lld >/dev/null 2>&1; then
        if [[ "$EUID" -eq 0 ]]; then
            apt-get update
            DEBIAN_FRONTEND=noninteractive apt-get install -y lld
        elif command -v sudo >/dev/null 2>&1 && sudo -n true; then
            sudo -n apt-get update
            sudo -n env DEBIAN_FRONTEND=noninteractive apt-get install -y lld
        else
            echo "rust-lld is missing; install lld with apt or run this script with package-install privileges." >&2
            exit 1
        fi
    fi

    if command -v lld >/dev/null 2>&1; then
        mkdir -p "$CARGO_BIN"
        ln -sf "$(command -v lld)" "$CARGO_BIN/rust-lld"
        echo "Using system lld as rust-lld." >&2
    else
        echo "Failed to install the lld linker required for WebAssembly builds." >&2
        exit 1
    fi
fi

exec trunk serve --address 0.0.0.0 --port 8080
