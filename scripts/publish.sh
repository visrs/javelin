#!/usr/bin/env bash

unset CDPATH

cd "$(dirname "$0")/.." || exit 1

CRATES=(
  "javelin-codec"
  "."
)

publish_crate() {
  cd "$1" || exit 2
  cargo publish "${@:2}"
  cd - || exit 2
}

echo "Publishing crates"

for crate in "${CRATES[@]}"; do
  publish_crate "$crate" "$@"
done
