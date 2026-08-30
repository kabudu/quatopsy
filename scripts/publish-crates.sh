#!/usr/bin/env bash
# Package or publish the lockstep Quatopsy workspace in dependency order.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

mode="${1:---inspect}"
if [[ "$mode" != "--inspect" && "$mode" != "--dry-run" && "$mode" != "--verify" && "$mode" != "--publish" ]]; then
  printf 'usage: %s [--inspect|--dry-run|--verify|--publish]\n' "$0" >&2
  exit 64
fi

if [[ "$mode" == "--publish" ]]; then
  [[ "${QUATOPSY_RELEASE_AUTHORIZE:-}" == "1" ]] || {
    printf 'publish-crates: refusing without QUATOPSY_RELEASE_AUTHORIZE=1\n' >&2
    exit 1
  }
  [[ -n "${CARGO_REGISTRY_TOKEN:-}" ]] || {
    printf 'publish-crates: CARGO_REGISTRY_TOKEN is required\n' >&2
    exit 1
  }
fi

version="$(python3 - <<'PY'
import re
from pathlib import Path
text = Path("Cargo.toml").read_text(encoding="utf-8").split("[workspace.package]", 1)[1]
match = re.search(r'^version = "([^"]+)"$', text, re.M)
if not match:
    raise SystemExit("workspace version missing")
print(match.group(1))
PY
)"
release_args=(check --expect-version "$version")
if [[ "$mode" == "--verify" || "$mode" == "--publish" ]]; then
  release_args+=(--release)
fi
python3 scripts/release.py "${release_args[@]}"

packages=(
  quatopsy-schema
  quatopsy-oracle
  quatopsy-nav
  quatopsy-guidance
  quatopsy-adapt
  quatopsy-core
  quatopsy-plan
  quatopsy-control
  quatopsy
)
registry_independent=(quatopsy-schema quatopsy-oracle quatopsy-nav)

if [[ "$mode" == "--verify" || "$mode" == "--publish" ]]; then
  [[ -z "$(git status --porcelain)" ]] || {
    printf 'publish-crates: release checkout must be clean\n' >&2
    exit 1
  }
fi

remote_checksum() {
  local package="$1"
  local output="$2"
  local code
  code="$(curl --silent --show-error \
    --user-agent "quatopsy-release/${version}" \
    --output "$output" \
    --write-out '%{http_code}' \
    "https://crates.io/api/v1/crates/${package}/${version}")"
  if [[ "$code" == "200" ]]; then
    python3 - "$output" <<'PY'
import json
import sys
from pathlib import Path
data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
checksum = data.get("version", {}).get("checksum")
if not isinstance(checksum, str) or len(checksum) != 64:
    raise SystemExit("crates.io response has no valid checksum")
print(checksum)
PY
    return 0
  fi
  [[ "$code" == "404" ]] || {
    printf 'publish-crates: crates.io returned HTTP %s for %s %s\n' \
      "$code" "$package" "$version" >&2
    return 2
  }
  return 1
}

file_checksum() {
  python3 - "$1" <<'PY'
import hashlib
import sys
from pathlib import Path
print(hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest())
PY
}

retry_after_seconds() {
  python3 - "$1" <<'PY'
import datetime as dt
import email.utils
import re
import sys
from pathlib import Path

text = Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace")
match = re.search(r"Please try again after (.+? GMT)", text)
if not match:
    raise SystemExit(1)
deadline = email.utils.parsedate_to_datetime(match.group(1))
now = dt.datetime.now(dt.timezone.utc)
seconds = max(1, int((deadline - now).total_seconds()) + 5)
print(seconds)
PY
}

bounded_sleep() {
  local remaining="$1"
  while (( remaining > 0 )); do
    local chunk=60
    (( remaining < chunk )) && chunk="$remaining"
    printf 'publish-crates: rate-limit backoff, %s seconds remaining\n' "$remaining"
    sleep "$chunk"
    remaining=$((remaining - chunk))
  done
}

for package in "${packages[@]}"; do
  if [[ "$mode" == "--inspect" ]]; then
    printf 'publish-crates: inspecting payload %s %s\n' "$package" "$version"
    payload="$(cargo package --list --allow-dirty -p "$package")"
    grep -qx 'Cargo.toml' <<<"$payload"
    grep -Eq '^(CRATE_)?README\.md$' <<<"$payload"
    grep -q '^src/' <<<"$payload"
    continue
  fi

  if [[ "$mode" == "--dry-run" ]]; then
    is_independent=false
    for independent in "${registry_independent[@]}"; do
      [[ "$package" == "$independent" ]] && is_independent=true
    done
    if [[ "$is_independent" == "true" ]]; then
      printf 'publish-crates: dry-running registry-independent %s %s\n' \
        "$package" "$version"
      cargo publish --locked --dry-run --no-verify --allow-dirty -p "$package"
    else
      printf 'publish-crates: inspecting dependent payload %s %s\n' \
        "$package" "$version"
      payload="$(cargo package --list --allow-dirty -p "$package")"
      grep -qx 'Cargo.toml' <<<"$payload"
      grep -Eq '^(CRATE_)?README\.md$' <<<"$payload"
      grep -q '^src/' <<<"$payload"
    fi
    continue
  fi

  printf 'publish-crates: packaging %s %s\n' "$package" "$version"
  cargo package --locked --no-verify -p "$package"
  crate="target/package/${package}-${version}.crate"
  [[ -f "$crate" ]] || {
    printf 'publish-crates: expected package missing: %s\n' "$crate" >&2
    exit 1
  }
  local_checksum="$(file_checksum "$crate")"

  if [[ "$mode" == "--verify" ]]; then
    response="$(mktemp)"
    if ! existing="$(remote_checksum "$package" "$response")"; then
      rm -f "$response"
      printf 'publish-crates: %s %s is not available for verification\n' \
        "$package" "$version" >&2
      exit 1
    fi
    rm -f "$response"
    [[ "$existing" == "$local_checksum" ]] || {
      printf 'publish-crates: published %s %s checksum differs from local package\n' \
        "$package" "$version" >&2
      exit 1
    }
    printf 'publish-crates: verified %s %s\n' "$package" "$version"
    continue
  fi

  response="$(mktemp)"
  if existing="$(remote_checksum "$package" "$response")"; then
    rm -f "$response"
    [[ "$existing" == "$local_checksum" ]] || {
      printf 'publish-crates: published %s %s checksum differs from local package\n' \
        "$package" "$version" >&2
      exit 1
    }
    printf 'publish-crates: %s %s already published identically\n' "$package" "$version"
    continue
  fi
  rm -f "$response"

  verified=false
  rate_limit_waited=0
  for attempt in 1 2 3 4 5 6 7 8 9 10; do
    printf 'publish-crates: publishing %s %s (attempt %s)\n' \
      "$package" "$version" "$attempt"
    publish_log="$(mktemp)"
    set +e
    cargo publish --locked -p "$package" 2>&1 | tee "$publish_log"
    publish_status="${PIPESTATUS[0]}"
    set -e
    for _ in 1 2 3 4 5 6; do
      response="$(mktemp)"
      if existing="$(remote_checksum "$package" "$response")"; then
        rm -f "$response"
        [[ "$existing" == "$local_checksum" ]] || {
          printf 'publish-crates: crates.io checksum mismatch for %s %s\n' \
            "$package" "$version" >&2
          exit 1
        }
        verified=true
        break
      fi
      rm -f "$response"
      sleep 10
    done
    if [[ "$verified" == "true" ]]; then
      rm -f "$publish_log"
      break
    fi
    if (( publish_status != 0 )); then
      if backoff="$(retry_after_seconds "$publish_log")"; then
        rm -f "$publish_log"
        if (( rate_limit_waited + backoff > 1800 )); then
          printf 'publish-crates: refusing cumulative crates.io backoff longer than 1800 seconds\n' >&2
          exit 1
        fi
        bounded_sleep "$backoff"
        rate_limit_waited=$((rate_limit_waited + backoff))
        continue
      fi
      printf 'publish-crates: cargo publish failed without a recognised retry deadline\n' >&2
      rm -f "$publish_log"
      exit "$publish_status"
    fi
    rm -f "$publish_log"
  done
  [[ "$verified" == "true" ]] || {
    printf 'publish-crates: could not verify %s %s on crates.io\n' \
      "$package" "$version" >&2
    exit 1
  }
done

printf 'publish-crates: %s complete for %s\n' "$mode" "$version"
