#!/usr/bin/env bash
# Build a local checksummed quatopsy binary. Does not sign or publish crates.
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
python3 - "$root/Cargo.toml" "$root/Cargo.lock" "$dest/PROVENANCE.txt" "$root" <<'PY'
import hashlib
import re
import subprocess
import sys
from pathlib import Path

cargo, lock, dest, repo = sys.argv[1:5]
match = re.search(r'^version = "([^"]+)"', Path(cargo).read_text(encoding="utf-8"), re.M)
if not match:
    raise SystemExit("workspace version missing")
git = subprocess.run(["git", "-C", repo, "rev-parse", "HEAD"], capture_output=True, text=True)
git_id = git.stdout.strip() if git.returncode == 0 else "unavailable"
rustc = subprocess.check_output(["rustc", "--version"], text=True).strip()
cargo_ver = subprocess.check_output(["cargo", "--version"], text=True).strip()
digest = hashlib.sha256(Path(lock).read_bytes()).hexdigest()
Path(dest).write_text(
    "\n".join(
        [
            "name: quatopsy",
            f"version: {match.group(1)}",
            f"git: {git_id}",
            f"rustc: {rustc}",
            f"cargo: {cargo_ver}",
            f"Cargo.lock: {digest}",
            "",
        ]
    ),
    encoding="utf-8",
)
PY
