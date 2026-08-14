#!/usr/bin/env bash
# Authoritative local CI for Quatopsy. Non-interactive, fail-fast, and
# offline-capable after Cargo dependency bootstrap.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ -t 1 ]]; then
  export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
else
  export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-never}"
fi

log() {
  printf '%s\n' "ci-local: $*"
}

fail() {
  printf '%s\n' "ci-local: error: $*" >&2
  exit 1
}

[[ -f Cargo.toml ]] || fail "workspace Cargo.toml is missing"
[[ -f AGENTS.md ]] || fail "AGENTS.md is missing"
[[ -x scripts/ci-local.sh ]] || fail "scripts/ci-local.sh must be executable"

log "scan tracked text for Unicode U+2014"
python3 - "$root" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
forbidden = "\u2014"
ignored = {".git", "target", ".venv", "node_modules"}
errors = []
for path in root.rglob("*"):
    if not path.is_file():
        continue
    if ignored.intersection(path.relative_to(root).parts):
        continue
    try:
        text = path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        continue
    for number, line in enumerate(text.splitlines(), start=1):
        if forbidden in line:
            errors.append(f"{path.relative_to(root)}:{number}")
if errors:
    print("forbidden Unicode U+2014:", file=sys.stderr)
    for item in errors:
        print(f"  {item}", file=sys.stderr)
    sys.exit(1)
PY

log "scan tracked text for incomplete markers"
python3 - "$root" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
parts = ["TO", "DO"]
fix = ["FIX", "ME"]
repl = ["REPLACE_", "WITH"]
marker = re.compile(
    r"\b(?:%s|%s|%s)\b" % ("".join(parts), "".join(fix), "".join(repl))
)
ignored = {".git", "target", ".venv", "node_modules"}
errors = []
for path in root.rglob("*"):
    if not path.is_file():
        continue
    relative = path.relative_to(root)
    if ignored.intersection(relative.parts):
        continue
    try:
        text = path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        continue
    for number, line in enumerate(text.splitlines(), start=1):
        if marker.search(line):
            errors.append(f"{relative}:{number}: {line.strip()}")
if errors:
    print("forbidden incomplete markers:", file=sys.stderr)
    for item in errors:
        print(f"  {item}", file=sys.stderr)
    sys.exit(1)
PY

if [[ -d .git ]]; then
  log "diff hygiene"
  git diff --check
  git diff --cached --check
fi

log "rustfmt"
cargo fmt --all -- --check

log "clippy"
cargo clippy --workspace --all-targets --locked -- -D warnings

log "tests"
cargo test --workspace --locked

log "CLI smoke"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cargo run --locked --bin quatopsy -- analyze \
  --input fixtures/conformance/clean_slew/input.csv \
  --manifest fixtures/conformance/clean_slew/manifest.json \
  --report "$tmp/report.json"
python3 - "$tmp/report.json" <<'PY'
import json
import sys
from pathlib import Path

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
if report.get("result") != "pass":
    raise SystemExit(f"clean slew expected pass, got {report.get('result')!r}")
if report.get("schema") != "quatopsy.report/1":
    raise SystemExit(f"unexpected schema {report.get('schema')!r}")
PY

log "passed"
