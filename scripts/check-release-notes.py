#!/usr/bin/env python3
"""Fail closed if curated release notes are missing, wrapped, or claim-violating."""
from __future__ import annotations

import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
notes_dir = root / ".github" / "release-notes"
cargo = (root / "Cargo.toml").read_text(encoding="utf-8")
match = re.search(r'^version = "([^"]+)"', cargo, re.M)
if not match:
    sys.exit("workspace version missing from Cargo.toml")
version = match.group(1)
path = notes_dir / f"v{version}.md"
if not path.is_file():
    sys.exit(f"missing curated notes {path.relative_to(root)}")

text = path.read_text(encoding="utf-8")
if "\u2014" in text:
    sys.exit(f"{path.name} contains Unicode U+2014")
lines = text.splitlines()
if not lines:
    sys.exit(f"{path.name} is empty")
title = lines[0].strip()
expected_title = re.compile(rf"^Quatopsy v{re.escape(version)}: .+$")
if not expected_title.match(title):
    sys.exit(f"title must match 'Quatopsy v{version}: <theme>', got {title!r}")
if title.startswith("#"):
    sys.exit("title must not be a Markdown heading")
if len(lines) < 3 or lines[1].strip():
    sys.exit("title must be followed by one blank line then the body")
body_lines = lines[2:]
body = "\n".join(body_lines)
if body.lstrip().startswith("#"):
    sys.exit("body must not start with a heading")
if re.search(r"(?i)^#+\s*release notes\b", body, re.M):
    sys.exit("body must not include a Release Notes heading")
if re.search(r"(?i)^##\s+\d{4}-\d{2}-\d{2}\b", body, re.M):
    sys.exit("body must not paste a changelog date heading")

fence = False
prev_prose = False
for number, line in enumerate(body_lines, start=3):
    stripped = line.strip()
    if stripped.startswith("```"):
        fence = not fence
        prev_prose = False
        continue
    if fence or not stripped:
        prev_prose = False
        continue
    is_list = stripped.startswith(("- ", "* ", "+ ")) or re.match(r"^\d+\.\s", stripped)
    if prev_prose and not is_list and not stripped.startswith("#"):
        sys.exit(f"{path.name}:{number}: hard-wrapped prose (keep each paragraph on one line)")
    prev_prose = not is_list and not stripped.startswith("#") and not stripped.startswith("|")

lower = body.lower()
forbidden = [
    r"\bnovel\b",
    r"flight-proven",
    r"\bcertified\b",
    r"production-ready",
    r"independently validated",
    r"expert consensus",
    r"flight approval",
]
for pattern in forbidden:
    for found in re.finditer(pattern, lower):
        start = max(0, found.start() - 80)
        window = lower[start : found.end() + 20]
        if re.search(
            r"(does not claim|do not claim|never|not claim|without|stays disabled|not signed)",
            window,
        ):
            continue
        if pattern == r"flight approval" and "not flight approval" in window:
            continue
        sys.exit(f"{path.name} contains prohibited claim phrase matching {pattern}")

bullets = [line for line in body_lines if line.strip().startswith("- ")]
if not 3 <= len(bullets) <= 5:
    sys.exit(f"body must contain 3 to 5 material-change bullets, found {len(bullets)}")
if "```bash" not in body:
    sys.exit("body must include one primary install path in a bash fence")
if "CHANGELOG.md" not in body:
    sys.exit("body must link the changelog")
if "advisory" not in lower:
    sys.exit("body must state the advisory boundary")
print(f"release-notes: {path.relative_to(root)}")
print(f"title: {title}")
