#!/usr/bin/env bash
# Optional Linux-like reproduction. Not part of the offline local CI gate.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
image="${QUATOPSY_LINUX_IMAGE:-rust:1.97.1-bookworm}"
if ! command -v docker >/dev/null 2>&1; then
  echo "linux-conformance: docker is not available; skip" >&2
  exit 0
fi
if ! docker image inspect "$image" >/dev/null 2>&1; then
  echo "linux-conformance: image $image is not present; skip" >&2
  exit 0
fi
docker run --rm \
  -e CARGO_HOME=/tmp/cargo-home \
  -e CARGO_TARGET_DIR=/tmp/cargo-target \
  -v "$root":/src:ro \
  "$image" \
  bash -lc 'cp -a /src /tmp/q && cd /tmp/q && rm -f rust-toolchain.toml && cargo test --locked -p quatopsy-core --test conformance --test oracle --test mutation --test fuzz'
