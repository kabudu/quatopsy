#!/usr/bin/env python3
"""Validate the public repository entry points without network access."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "README.md"
SETTINGS = ROOT / ".github" / "repository-settings.json"
ARCHITECTURE = ROOT / "assets" / "brand" / "templates" / "diagram-workflow.svg"
ARCHITECTURE_NARROW = ROOT / "assets" / "brand" / "templates" / "diagram-workflow-narrow.svg"

REQUIRED = [
    README,
    ROOT / "CRATE_README.md",
    ROOT / "CONTRIBUTING.md",
    ROOT / "CODE_OF_CONDUCT.md",
    ROOT / "SECURITY.md",
    ROOT / "SUPPORT.md",
    ROOT / "LICENSE",
    ROOT / "NOTICE",
    ROOT / ".github" / "PULL_REQUEST_TEMPLATE.md",
    ROOT / ".github" / "ISSUE_TEMPLATE" / "config.yml",
    ROOT / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml",
    ROOT / ".github" / "ISSUE_TEMPLATE" / "feature_request.yml",
    ROOT / ".github" / "ISSUE_TEMPLATE" / "adapter_request.yml",
    ROOT / ".github" / "dependabot.yml",
    ROOT / ".github" / "workflows" / "ci.yml",
    ROOT / ".github" / "workflows" / "prepare-release.yml",
    ROOT / ".github" / "workflows" / "release.yml",
    ROOT / "CHANGELOG.md",
    ROOT / "scripts" / "release.py",
    ROOT / "scripts" / "publish-crates.sh",
    SETTINGS,
    ARCHITECTURE,
    ARCHITECTURE_NARROW,
]


def fail(message: str) -> None:
    raise SystemExit(f"community-check: {message}")


for path in REQUIRED:
    if not path.is_file() or path.stat().st_size == 0:
        fail(f"missing or empty {path.relative_to(ROOT)}")

readme = README.read_text(encoding="utf-8")
required_readme = [
    "assets/brand/templates/release-lockup.svg",
    "See where rotations go wrong.",
    "## Quick start",
    "## What Quatopsy finds",
    "## How it fits together",
    "assets/brand/templates/diagram-workflow.svg",
    "assets/brand/templates/diagram-workflow-narrow.svg",
    "Quatopsy is advisory research software.",
    "early-stage, production-quality research software for local advisory evaluation",
    "CONTRIBUTING.md",
    "SECURITY.md",
]
for item in required_readme:
    if item not in readme:
        fail(f"README is missing required product entry point {item!r}")

local_targets: set[str] = set()
for match in re.finditer(r"\[[^\]]+\]\(([^)]+)\)", readme):
    local_targets.add(match.group(1))
for match in re.finditer(r'''(?:src|href)=["']([^"']+)["']''', readme):
    local_targets.add(match.group(1))
for raw in sorted(local_targets):
    target = raw.split("#", 1)[0]
    if not target or re.match(r"^[a-z][a-z0-9+.-]*:", target):
        continue
    if target.startswith("#"):
        continue
    if not (ROOT / target).exists():
        fail(f"README local link does not exist: {raw}")

settings = json.loads(SETTINGS.read_text(encoding="utf-8"))
if settings.get("schema") != "quatopsy.repository-settings/2":
    fail("repository settings schema is unsupported")
description = settings.get("description")
if not isinstance(description, str) or not 40 <= len(description) <= 160:
    fail("repository description must contain 40 to 160 characters")
topics = settings.get("topics")
if not isinstance(topics, list) or not 5 <= len(topics) <= 20:
    fail("repository topics must contain 5 to 20 entries")
if topics != sorted(set(topics)):
    fail("repository topics must be unique and sorted")
for topic in topics:
    if not isinstance(topic, str) or not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,49}", topic):
        fail(f"invalid repository topic {topic!r}")
if settings.get("visibility") != "public":
    fail("repository settings must require public visibility")
if settings.get("hosted_ci") != "enabled":
    fail("repository settings must require hosted CI")
if settings.get("release_registry") != "crates.io":
    fail("repository settings must bind Cargo releases to crates.io")

for architecture in [ARCHITECTURE, ARCHITECTURE_NARROW]:
    svg = architecture.read_text(encoding="utf-8")
    try:
        root = ElementTree.fromstring(svg)
    except ElementTree.ParseError as error:
        fail(f"{architecture.name} is invalid XML: {error}")
    if root.tag != "{http://www.w3.org/2000/svg}svg":
        fail(f"{architecture.name} root is not SVG")
    if root.get("role") != "img" or root.get("aria-labelledby") != "title desc":
        fail(f"{architecture.name} is missing its accessible image contract")
    for element in root.iter():
        tag = element.tag.rsplit("}", 1)[-1]
        if tag in {"script", "style", "foreignObject", "iframe", "animate", "set"}:
            fail(f"{architecture.name} contains forbidden element {tag}")
        for name, value in element.attrib.items():
            plain_name = name.rsplit("}", 1)[-1].lower()
            lowered = value.lower()
            if plain_name.startswith("on") or "javascript:" in lowered or "data:" in lowered:
                fail(f"{architecture.name} contains unsafe attribute {plain_name}")
            if "url(" in lowered and not re.fullmatch(r"url\(#[a-z0-9_-]+\)", lowered):
                fail(f"{architecture.name} contains non-local resource reference {value!r}")

notice = (ROOT / "NOTICE").read_text(encoding="utf-8")
if "not cleared for public productisation" in notice:
    fail("NOTICE still contains superseded productisation copy")

project_authored_docs = [
    *ROOT.glob("*.md"),
    *ROOT.joinpath("docs").rglob("*.md"),
    *ROOT.joinpath(".github").rglob("*.md"),
    ROOT / "NOTICE",
]
obsolete_name_caveat = "trade" + "mark"
for path in project_authored_docs:
    if obsolete_name_caveat in path.read_text(encoding="utf-8").casefold():
        fail(f"obsolete name-clearance caveat in {path.relative_to(ROOT)}")

print(f"community-check: {len(REQUIRED)} required files")
print(f"community-check: {len(local_targets)} README links and assets")
print(f"community-check: {len(topics)} repository topics")
