#!/usr/bin/env python3
"""Prepare and validate lockstep Cargo releases from CHANGELOG.md."""
from __future__ import annotations

import argparse
import datetime as dt
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARGO = ROOT / "Cargo.toml"
CHANGELOG = ROOT / "CHANGELOG.md"
REPOSITORY = "https://github.com/kabudu/quatopsy"
SEMVER = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
RELEASE_HEADING = re.compile(r"^## \[([^]]+)] - ([0-9]{4}-[0-9]{2}-[0-9]{2})$", re.M)
INTERNAL_DEPENDENCY = re.compile(
    r'^(quatopsy-(?:adapt|core|oracle|plan|nav|guidance|control|schema)) = '
    r'\{ version = "=([^"]+)", path = "([^"]+)" \}$',
    re.M,
)


def fail(message: str) -> None:
    raise SystemExit(f"release: {message}")


def cargo_version(text: str) -> str:
    workspace = text.split("[workspace.package]", 1)
    if len(workspace) != 2:
        fail("Cargo.toml has no [workspace.package] section")
    match = re.search(r'^version = "([^"]+)"$', workspace[1], re.M)
    if not match:
        fail("workspace package version is missing")
    return match.group(1)


def version_tuple(value: str) -> tuple[int, int, int]:
    match = SEMVER.fullmatch(value)
    if not match:
        fail(f"version must be stable SemVer major.minor.patch, got {value!r}")
    return tuple(int(part) for part in match.groups())


def changelog_section(text: str, version: str) -> str:
    match = re.search(
        rf"^## \[{re.escape(version)}](?: - [0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}})?\n\n(.*?)(?=^## \[|\Z)",
        text,
        re.M | re.S,
    )
    if not match:
        fail(f"CHANGELOG.md has no section for {version}")
    return match.group(1).strip()


def check(expect_version: str | None, release: bool) -> str:
    cargo = CARGO.read_text(encoding="utf-8")
    changelog = CHANGELOG.read_text(encoding="utf-8")
    version = cargo_version(cargo)
    version_tuple(version)
    if expect_version and version != expect_version:
        fail(f"expected version {expect_version}, found {version}")
    required_intro = (
        "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), "
        "and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)."
    )
    if not changelog.startswith("# Changelog\n\n") or required_intro not in changelog:
        fail("CHANGELOG.md does not declare Keep a Changelog 1.1.0 and Semantic Versioning")
    if changelog.count("## [Unreleased]") != 1:
        fail("CHANGELOG.md must contain one [Unreleased] section")
    releases = RELEASE_HEADING.findall(changelog)
    if not releases:
        fail("CHANGELOG.md has no dated releases")
    if releases[0][0] != version:
        fail(f"newest changelog release {releases[0][0]} does not match Cargo {version}")
    previous: tuple[int, int, int] | None = None
    for release_version, date in releases:
        parsed = version_tuple(release_version)
        if previous is not None and parsed >= previous:
            fail("changelog releases are not in descending SemVer order")
        previous = parsed
        try:
            dt.date.fromisoformat(date)
        except ValueError as error:
            fail(f"invalid release date {date}: {error}")
        section = changelog_section(changelog, release_version)
        if not section or "### " not in section:
            fail(f"release {release_version} has no categorized changes")
    unreleased = changelog_section(changelog, "Unreleased")
    if release and unreleased:
        fail("a release commit must have an empty [Unreleased] section")
    expected_unreleased = f"[Unreleased]: {REPOSITORY}/compare/v{version}...HEAD"
    if expected_unreleased not in changelog:
        fail(f"missing changelog link {expected_unreleased!r}")
    for release_version, _ in releases:
        if not re.search(rf"^\[{re.escape(release_version)}]: https://", changelog, re.M):
            fail(f"missing changelog link for release {release_version}")
    dependencies = INTERNAL_DEPENDENCY.findall(cargo)
    if len(dependencies) != 8:
        fail(f"expected eight versioned internal dependencies, found {len(dependencies)}")
    for name, dependency_version, path in dependencies:
        if dependency_version != version:
            fail(f"{name} requires {dependency_version}, expected {version}")
        if not (ROOT / path / "Cargo.toml").is_file():
            fail(f"{name} path does not contain Cargo.toml: {path}")
    print(f"release: changelog and Cargo workspace agree on {version}")
    return version


def prepare(version: str, date: str) -> None:
    new_tuple = version_tuple(version)
    try:
        dt.date.fromisoformat(date)
    except ValueError as error:
        fail(f"invalid ISO release date {date}: {error}")
    cargo = CARGO.read_text(encoding="utf-8")
    changelog = CHANGELOG.read_text(encoding="utf-8")
    current = cargo_version(cargo)
    if new_tuple <= version_tuple(current):
        fail(f"new version {version} must be greater than {current}")
    unreleased = changelog_section(changelog, "Unreleased")
    if not unreleased:
        fail("cannot prepare a release from an empty [Unreleased] section")
    marker = f"## [{current}] -"
    marker_index = changelog.find(marker)
    if marker_index < 0:
        fail(f"current release heading for {current} is missing")
    prefix = changelog[:marker_index]
    expected = f"## [Unreleased]\n\n{unreleased}\n\n"
    if not prefix.endswith(expected):
        fail("[Unreleased] content is not immediately before the current release")
    prefix = prefix[: -len(expected)]
    changelog = (
        prefix
        + f"## [Unreleased]\n\n## [{version}] - {date}\n\n{unreleased}\n\n"
        + changelog[marker_index:]
    )
    old_link = f"[Unreleased]: {REPOSITORY}/compare/v{current}...HEAD"
    new_links = (
        f"[Unreleased]: {REPOSITORY}/compare/v{version}...HEAD\n"
        f"[{version}]: {REPOSITORY}/compare/v{current}...v{version}"
    )
    if old_link not in changelog:
        fail(f"missing current [Unreleased] link for {current}")
    changelog = changelog.replace(old_link, new_links, 1)
    cargo = cargo.replace(f'version = "{current}"', f'version = "{version}"', 1)
    cargo = cargo.replace(f'version = "={current}"', f'version = "={version}"')
    CARGO.write_text(cargo, encoding="utf-8")
    CHANGELOG.write_text(changelog, encoding="utf-8")
    print(f"release: prepared {version} dated {date}; run cargo update --workspace")


def notes(version: str) -> None:
    changelog = CHANGELOG.read_text(encoding="utf-8")
    section = changelog_section(changelog, version)
    print(section)
    print()
    print("Quatopsy remains local advisory software. See SECURITY.md and docs/CLAIMS.md for the supported boundary.")


def main() -> None:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    check_parser = subparsers.add_parser("check")
    check_parser.add_argument("--expect-version")
    check_parser.add_argument("--release", action="store_true")
    prepare_parser = subparsers.add_parser("prepare")
    prepare_parser.add_argument("version")
    prepare_parser.add_argument("--date", default=dt.date.today().isoformat())
    notes_parser = subparsers.add_parser("notes")
    notes_parser.add_argument("version")
    args = parser.parse_args()
    if args.command == "check":
        check(args.expect_version, args.release)
    elif args.command == "prepare":
        prepare(args.version, args.date)
    elif args.command == "notes":
        notes(args.version)


if __name__ == "__main__":
    main()
