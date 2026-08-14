#!/usr/bin/env bash
# Build a local checksummed quatopsy binary. Does not sign, publish, or tag.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
dest="${1:-"$root/dist"}"
mkdir -p "$dest"
cargo build --release --locked --bin quatopsy
target_dir="${CARGO_TARGET_DIR:-$root/target}"
src="$target_dir/release/quatopsy"
if [[ ! -f "$src" ]]; then
  printf 'package-local: missing %s\n' "$src" >&2
  exit 1
fi
cp "$src" "$dest/quatopsy"
chmod +x "$dest/quatopsy"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$dest" && sha256sum quatopsy > SHA256SUMS)
else
  (cd "$dest" && shasum -a 256 quatopsy > SHA256SUMS)
fi
python3 - "$dest/SHA256SUMS" "$dest/quatopsy" <<'PY'
import hashlib
import sys
from pathlib import Path

sums = Path(sys.argv[1]).read_text(encoding="utf-8").split()
digest = sums[0]
payload = Path(sys.argv[2]).read_bytes()
got = hashlib.sha256(payload).hexdigest()
if got != digest:
    raise SystemExit(f"checksum mismatch: {got} != {digest}")
print(f"package: {sys.argv[2]}")
print(f"sha256: {got}")
PY
