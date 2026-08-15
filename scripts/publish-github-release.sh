#!/usr/bin/env bash
# Publish a private GitHub Release from curated notes. Never opens the
# repository, never publishes crates, and never stores credentials.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ "${QUATOPSY_RELEASE_AUTHORIZE:-}" != "1" ]]; then
  printf 'publish-github-release: refusing without QUATOPSY_RELEASE_AUTHORIZE=1\n' >&2
  exit 1
fi

version="$(python3 - "$root/Cargo.toml" <<'PY'
import re, sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding="utf-8")
match = re.search(r'^version = "([^"]+)"', text, re.M)
if not match:
    raise SystemExit("workspace version missing")
print(match.group(1))
PY
)"
tag="v${version}"
notes="$root/.github/release-notes/${tag}.md"
[[ -f "$notes" ]] || { printf 'missing %s\n' "$notes" >&2; exit 1; }
python3 "$root/scripts/check-release-notes.py"

title="$(python3 - "$notes" <<'PY'
from pathlib import Path
import sys
print(Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()[0])
PY
)"
body="$(python3 - "$notes" <<'PY'
from pathlib import Path
import sys
lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
print("\n".join(lines[2:]))
PY
)"

visibility="$(gh repo view --json isPrivate --jq '.isPrivate')"
if [[ "$visibility" != "true" ]]; then
  printf 'publish-github-release: repository is not private; public opening is a distinct gate\n' >&2
  exit 1
fi

dest="${QUATOPSY_DIST:-"$root/dist"}"
bash "$root/scripts/package-local.sh" "$dest"

tmp="$(mktemp)"
printf '%s\n' "$body" > "$tmp"
gh release create "$tag" \
  --title "$title" \
  --notes-file "$tmp" \
  --target "${QUATOPSY_RELEASE_TARGET:-master}" \
  "$dest/quatopsy" \
  "$dest/SHA256SUMS" \
  "$dest/PROVENANCE.txt"
rm -f "$tmp"
printf 'published %s\n' "$tag"
