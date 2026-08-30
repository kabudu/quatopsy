#!/usr/bin/env python3
"""Validate the static Quatopsy website without executing browser content."""
from __future__ import annotations

import json
import re
import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlparse


class SiteParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: list[str] = []
        self.headings: list[int] = []
        self.h1 = 0
        self.images: list[dict[str, str | None]] = []
        self.links: list[tuple[str | None, str]] = []
        self.resources: list[str] = []
        self.scripts: list[dict[str, str | None]] = []
        self.meta: list[dict[str, str | None]] = []
        self.canonicals: list[str] = []
        self._anchor_href: str | None = None
        self._anchor_text: list[str] = []
        self._script_type: str | None = None
        self._script_text: list[str] = []
        self.json_ld: list[str] = []
        self.title_text: list[str] = []
        self._in_title = False

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = dict(attrs)
        if values.get("id"):
            self.ids.append(values["id"] or "")
        if re.fullmatch(r"h[1-6]", tag):
            level = int(tag[1])
            self.headings.append(level)
            self.h1 += level == 1
        if tag == "img":
            self.images.append(values)
            if values.get("src"):
                self.resources.append(values["src"] or "")
        elif tag == "source" and values.get("srcset"):
            self.resources.append(values["srcset"] or "")
        elif tag == "link":
            href = values.get("href")
            if href:
                if values.get("rel") == "canonical":
                    self.canonicals.append(href)
                else:
                    self.resources.append(href)
        elif tag == "a":
            self._anchor_href = values.get("href")
            self._anchor_text = []
        elif tag == "script":
            self.scripts.append(values)
            self._script_type = values.get("type")
            self._script_text = []
        elif tag == "meta":
            self.meta.append(values)
        elif tag == "title":
            self._in_title = True

    def handle_data(self, data: str) -> None:
        if self._anchor_href is not None:
            self._anchor_text.append(data)
        if self._script_type == "application/ld+json":
            self._script_text.append(data)
        if self._in_title:
            self.title_text.append(data)

    def handle_endtag(self, tag: str) -> None:
        if tag == "a" and self._anchor_href is not None:
            self.links.append((self._anchor_href, "".join(self._anchor_text).strip()))
            self._anchor_href = None
            self._anchor_text = []
        elif tag == "script":
            if self._script_type == "application/ld+json":
                self.json_ld.append("".join(self._script_text))
            self._script_type = None
            self._script_text = []
        elif tag == "title":
            self._in_title = False


def fail(errors: list[str]) -> None:
    print("site-check failed:", file=sys.stderr)
    for error in errors:
        print(f"  {error}", file=sys.stderr)
    raise SystemExit(1)


root = Path(sys.argv[1]).resolve() if len(sys.argv) == 2 else Path("target/site").resolve()
errors: list[str] = []
required = ["index.html", "styles.css", "robots.txt", "sitemap.xml", ".nojekyll"]
for relative in required:
    path = root / relative
    if not path.is_file() or path.is_symlink():
        errors.append(f"missing regular file {relative}")

html_path = root / "index.html"
if errors:
    fail(errors)
html = html_path.read_text(encoding="utf-8")
parser = SiteParser()
parser.feed(html)

if parser.h1 != 1:
    errors.append(f"expected exactly one h1, found {parser.h1}")
if not parser.headings or any(next_level > level + 1 for level, next_level in zip(parser.headings, parser.headings[1:])):
    errors.append("heading order skips a level")
if len(parser.ids) != len(set(parser.ids)):
    errors.append("duplicate element id")
if parser.canonicals != ["https://kabudu.github.io/quatopsy/"]:
    errors.append("canonical URL is missing or incorrect")
title = "".join(parser.title_text).strip()
if not 20 <= len(title) <= 60:
    errors.append(f"title length is {len(title)}, expected 20 to 60")

meta_keys = {(item.get("name") or item.get("property")): item.get("content") for item in parser.meta}
for key in ("description", "og:title", "og:description", "og:image", "twitter:card"):
    if not meta_keys.get(key):
        errors.append(f"missing metadata {key}")
description = meta_keys.get("description") or ""
if not 70 <= len(description) <= 170:
    errors.append(f"description length is {len(description)}, expected 70 to 170")

if len(parser.json_ld) != 1:
    errors.append("expected one JSON-LD document")
else:
    try:
        structured = json.loads(parser.json_ld[0])
        if structured.get("@type") != "SoftwareApplication" or structured.get("name") != "Quatopsy":
            errors.append("JSON-LD software identity is incorrect")
    except json.JSONDecodeError as error:
        errors.append(f"invalid JSON-LD: {error}")
for script in parser.scripts:
    if script.get("type") != "application/ld+json" or script.get("src"):
        errors.append("executable or remote script is forbidden")

for image in parser.images:
    src = image.get("src") or ""
    if image.get("alt") is None:
        errors.append(f"image lacks alt attribute: {src}")
    if not image.get("width") or not image.get("height"):
        errors.append(f"image lacks intrinsic dimensions: {src}")

for href, text in parser.links:
    if not href:
        errors.append("anchor lacks href")
        continue
    if not text and "aria-label" not in html:
        errors.append(f"anchor has no accessible text: {href}")
    if href.startswith("#") and href[1:] not in parser.ids:
        errors.append(f"fragment target does not exist: {href}")

for raw in parser.resources:
    parsed = urlparse(raw)
    if parsed.scheme or parsed.netloc or raw.startswith("//"):
        errors.append(f"remote page resource is forbidden: {raw}")
        continue
    target = raw.split("?", 1)[0].split("#", 1)[0]
    if target and not (root / target).is_file():
        errors.append(f"local resource does not exist: {raw}")

css = (root / "styles.css").read_text(encoding="utf-8")
for required_css in ("prefers-reduced-motion", "forced-colors", "focus-visible"):
    if required_css not in css:
        errors.append(f"compiled CSS lacks {required_css}")
if re.search(r"url\(\s*['\"]?(?:https?:)?//", css, re.I) or "@font-face" in css:
    errors.append("compiled CSS contains a remote resource")

total_bytes = sum(path.stat().st_size for path in root.rglob("*") if path.is_file())
if total_bytes > 1_800_000:
    errors.append(f"site payload is {total_bytes} bytes, exceeds 1800000")
for path in root.rglob("*"):
    if path.is_symlink():
        errors.append(f"site contains symlink: {path.relative_to(root)}")

if errors:
    fail(errors)
print(f"site-check: {len(parser.links)} links, {len(parser.images)} images, {total_bytes} bytes")
