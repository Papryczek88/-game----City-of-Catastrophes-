#!/bin/sh
cd "$(dirname "$0")"
if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/Cargo is not installed or is not in PATH."
  echo "Install Rust from https://rustup.rs/ and try again."
  exit 1
fi
cargo run --release
status=$?
if [ "$status" -ne 0 ]; then
  echo "The game could not start. Check the error above."
  printf "Press Enter to close..."
  read -r _
fi
exit "$status"
