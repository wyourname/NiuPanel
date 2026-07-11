#!/bin/sh
set -eu

DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
exec cargo run --quiet --manifest-path "$DIR/Cargo.toml"
