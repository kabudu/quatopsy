#!/usr/bin/env python3
"""Deterministic Quatopsy brand export and validation.

Canonical construction lives here. `export` writes `assets/brand/`. `check`
recomputes the tree and refuses drift, unsafe SVG, weak contrast, undeclared
licences, maturity wording on canonical lockups, and overlay leakage into tokens.
"""
from __future__ import annotations

import hashlib
import json
import math
import re
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BRAND = ROOT / "assets" / "brand"
BRAND_VERSION = "quatopsy.brand/1"
TAGLINE = "See where rotations go wrong."
CATEGORY = "Orientation-trajectory diagnostics"

# Dark-first forensic palette. Semantic colours are paired with labels and shapes.
TOKENS = {
    "brand_version": BRAND_VERSION,
    "name": "Quatopsy",
    "tagline": TAGLINE,
    "category": CATEGORY,
    "palette": {
        "dark": {
            "bg": "#05080b",
            "surface": "#0a1015",
            "ink": "#eef4f6",
            "muted": "#9babb4",
            "accent": "#62e6df",
            "pass": "#74e6a1",
            "findings": "#ff7770",
            "refused": "#ffc65c",
            "error": "#ff7770",
            "focus": "#fff3ad",
        },
        "light": {
            "bg": "#f4f7f8",
            "surface": "#ffffff",
            "ink": "#12181c",
            "muted": "#3d4c54",
            "accent": "#0b6f6a",
            "pass": "#0f6b3a",
            "findings": "#a31b16",
            "refused": "#7a4e00",
            "error": "#a31b16",
            "focus": "#5b4b00",
        },
        "mono": {"bg": "#ffffff", "ink": "#111111", "accent": "#111111"},
        "reversed": {"bg": "#111111", "ink": "#f4f4f4", "accent": "#f4f4f4"},
        "high_contrast": {
            "bg": "#000000",
            "ink": "#ffffff",
            "accent": "#ffff00",
            "pass": "#00ff00",
            "findings": "#ff0000",
            "refused": "#ffff00",
            "error": "#ff00ff",
        },
        "forced_colour": {
            "bg": "Canvas",
            "ink": "CanvasText",
            "accent": "Highlight",
            "pass": "LinkText",
            "findings": "Mark",
            "refused": "GrayText",
            "error": "Mark",
        },
    },
    "type": {
        "ui": 'system-ui, "Segoe UI", sans-serif',
        "mono": 'ui-monospace, "SFMono-Regular", Menlo, Consolas, monospace',
        "recommended_open_ui": "IBM Plex Sans",
        "recommended_open_mono": "IBM Plex Mono",
        "redistributed": False,
    },
    "result_shapes": {
        "pass": "circle",
        "findings": "diamond",
        "refused": "triangle",
        "error": "square",
    },
    "motion": {"reduced": "disable non-essential animation and autoplay"},
    "mark": {
        "direction": "antipodal-paired-point",
        "clear_space_units": 8,
        "min_size_px": 16,
        "viewbox": 32,
    },
}

MATURITY_RE = re.compile(
    r"\b(evaluation|alpha|beta|preview|release[ -]?candidate|production-ready|experimental)\b",
    re.I,
)
CANONICAL_SKIP_MATURITY = {
    "templates/overlay-private-research.svg",
    "source/directions/lifted-path.svg",
    "source/directions/antipodal-paired-point.svg",
    "source/directions/quotient-lens.svg",
}


def hex_rgb(value: str) -> tuple[int, int, int]:
    text = value.removeprefix("#")
    return int(text[0:2], 16), int(text[2:4], 16), int(text[4:6], 16)


def relative_luminance(rgb: tuple[int, int, int]) -> float:
    def channel(raw: int) -> float:
        c = raw / 255.0
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    r, g, b = rgb
    return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)


def contrast_ratio(fg: str, bg: str) -> float:
    a = relative_luminance(hex_rgb(fg))
    b = relative_luminance(hex_rgb(bg))
    lighter, darker = (a, b) if a >= b else (b, a)
    return (lighter + 0.05) / (darker + 0.05)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def svg_wrap(body: str, view: int = 32, bg: str | None = None) -> str:
    background = f'<rect width="{view}" height="{view}" fill="{bg}"/>' if bg else ""
    return (
        f'<?xml version="1.0" encoding="UTF-8"?>\n'
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {view} {view}" '
        f'role="img" aria-label="Quatopsy">\n'
        f"{background}{body}\n</svg>\n"
    )


def antipodal_geometry(view: int = 32) -> dict[str, float]:
    cx = cy = view / 2
    radius = view * 0.34375
    a1 = math.radians(-50)
    a2 = math.radians(130)
    return {
        "cx": cx,
        "cy": cy,
        "r": radius,
        "x1": cx + radius * math.cos(a1),
        "y1": cy + radius * math.sin(a1),
        "x2": cx + radius * math.cos(a2),
        "y2": cy + radius * math.sin(a2),
        "dot": max(view * 0.07, 1.6),
        "stroke": max(view / 32 * 1.7, 1.15),
    }


def symbol_svg(ink: str, accent: str, *, small: bool = False, bg: str | None = None) -> str:
    g = antipodal_geometry()
    dots = (
        f'<circle cx="{g["x1"]:.3f}" cy="{g["y1"]:.3f}" r="{g["dot"]:.3f}" fill="{accent}"/>'
        f'<circle cx="{g["x2"]:.3f}" cy="{g["y2"]:.3f}" r="{g["dot"]:.3f}" fill="{accent}"/>'
    )
    ring = (
        f'<circle cx="{g["cx"]:.3f}" cy="{g["cy"]:.3f}" r="{g["r"]:.3f}" '
        f'fill="none" stroke="{ink}" stroke-width="{g["stroke"]:.3f}"/>'
    )
    chord = ""
    if not small:
        chord = (
            f'<path d="M {g["x1"]:.3f} {g["y1"]:.3f} A {g["r"]:.3f} {g["r"]:.3f} 0 0 1 '
            f'{g["x2"]:.3f} {g["y2"]:.3f}" fill="none" stroke="{accent}" '
            f'stroke-width="{g["stroke"]:.3f}" stroke-linecap="round"/>'
        )
    return svg_wrap(ring + chord + dots, bg=bg)


def direction_lifted_path() -> str:
    body = (
        '<rect width="32" height="32" fill="#05080b"/>'
        '<path d="M4 24 C10 8, 22 8, 28 24" fill="none" stroke="#62e6df" stroke-width="1.7"/>'
        '<path d="M18 11 l2.2-4.2" stroke="#ffc65c" stroke-width="1.5" stroke-linecap="round"/>'
        '<circle cx="18" cy="11" r="1.7" fill="#eef4f6"/>'
    )
    return svg_wrap(body)


def direction_quotient_lens() -> str:
    body = (
        '<rect width="32" height="32" fill="#05080b"/>'
        '<circle cx="13" cy="16" r="8.2" fill="none" stroke="#eef4f6" stroke-width="1.6"/>'
        '<circle cx="19" cy="16" r="8.2" fill="none" stroke="#62e6df" stroke-width="1.6"/>'
        '<path d="M16 9.2 V22.8" stroke="#ffc65c" stroke-width="1.4"/>'
    )
    return svg_wrap(body)


def wordmark_svg(ink: str, width: int = 180, height: int = 32) -> str:
    return (
        f'<?xml version="1.0" encoding="UTF-8"?>\n'
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" '
        f'role="img" aria-label="Quatopsy">\n'
        f'<text x="0" y="22" fill="{ink}" font-family="{TOKENS["type"]["ui"]}" '
        f'font-size="18" font-weight="650" letter-spacing="0.08em">Quatopsy</text>\n'
        f"</svg>\n"
    )


def lockup_horizontal_svg(ink: str, accent: str, bg: str | None = None) -> str:
    symbol = symbol_svg(ink, accent).split("\n", 2)[-1].rsplit("</svg>", 1)[0]
    background = f'<rect width="220" height="32" fill="{bg}"/>' if bg else ""
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 220 32" '
        'role="img" aria-label="Quatopsy">\n'
        f"{background}"
        f'<g transform="translate(0,0)">{symbol}</g>'
        f'<text x="40" y="22" fill="{ink}" font-family="{TOKENS["type"]["ui"]}" '
        f'font-size="18" font-weight="650" letter-spacing="0.08em">Quatopsy</text>\n'
        "</svg>\n"
    )


def lockup_stacked_svg(ink: str, accent: str, bg: str | None = None) -> str:
    symbol = symbol_svg(ink, accent).split("\n", 2)[-1].rsplit("</svg>", 1)[0]
    background = f'<rect width="96" height="64" fill="{bg}"/>' if bg else ""
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 64" '
        'role="img" aria-label="Quatopsy">\n'
        f"{background}"
        f'<g transform="translate(32,0) scale(1)">{symbol}</g>'
        f'<text x="48" y="56" text-anchor="middle" fill="{ink}" '
        f'font-family="{TOKENS["type"]["ui"]}" font-size="12" font-weight="650" '
        f'letter-spacing="0.08em">Quatopsy</text>\n'
        "</svg>\n"
    )


def result_icon_svg(kind: str, fill: str) -> str:
    shapes = {
        "pass": '<circle cx="12" cy="12" r="8" fill="none" stroke="{0}" stroke-width="2"/>'
        '<path d="M8 12 l2.4 2.4 5.2-5.6" fill="none" stroke="{0}" stroke-width="2" '
        'stroke-linecap="round"/>',
        "findings": '<path d="M12 3 L21 12 L12 21 L3 12 Z" fill="none" stroke="{0}" stroke-width="2"/>',
        "refused": '<path d="M12 4 L20 19 H4 Z" fill="none" stroke="{0}" stroke-width="2"/>',
        "error": '<rect x="5" y="5" width="14" height="14" fill="none" stroke="{0}" stroke-width="2"/>',
    }
    label = kind.upper()
    body = (
        f'<title>Quatopsy {label}</title>'
        + shapes[kind].format(fill)
        + f'<text x="12" y="34" text-anchor="middle" fill="{fill}" font-size="6" '
        f'font-family="{TOKENS["type"]["ui"]}">{label}</text>'
    )
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 40" role="img" '
        f'aria-label="Quatopsy {label}">{body}</svg>\n'
    )


def workflow_diagram_svg() -> str:
    dark = TOKENS["palette"]["dark"]
    boxes = [
        (8, 18, "CSV"),
        (70, 18, "analyze"),
        (140, 18, "report"),
        (210, 18, "view"),
    ]
    parts = [f'<rect width="280" height="48" fill="{dark["bg"]}"/>']
    for x, y, label in boxes:
        parts.append(
            f'<rect x="{x}" y="{y}" width="52" height="20" fill="none" '
            f'stroke="{dark["accent"]}" stroke-width="1.2"/>'
            f'<text x="{x + 26}" y="{y + 14}" text-anchor="middle" fill="{dark["ink"]}" '
            f'font-size="7" font-family="{TOKENS["type"]["mono"]}">{label}</text>'
        )
    parts.append(
        f'<path d="M60 28 H70 M122 28 H140 M192 28 H210" stroke="{dark["muted"]}" '
        f'stroke-width="1"/>'
    )
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 280 48" '
        'role="img" aria-label="Quatopsy workflow">'
        + "".join(parts)
        + "</svg>\n"
    )


def chart_states_svg() -> str:
    dark = TOKENS["palette"]["dark"]
    rows = [
        ("pass", dark["pass"], "circle"),
        ("findings", dark["findings"], "diamond"),
        ("refused", dark["refused"], "triangle"),
        ("error", dark["error"], "square"),
    ]
    parts = [f'<rect width="160" height="80" fill="{dark["bg"]}"/>']
    y = 14
    for label, colour, shape in rows:
        if shape == "circle":
            mark = f'<circle cx="14" cy="{y}" r="5" fill="none" stroke="{colour}"/>'
        elif shape == "diamond":
            mark = f'<path d="M14 {y-6} L20 {y} L14 {y+6} L8 {y} Z" fill="none" stroke="{colour}"/>'
        elif shape == "triangle":
            mark = f'<path d="M14 {y-6} L20 {y+5} H8 Z" fill="none" stroke="{colour}"/>'
        else:
            mark = f'<rect x="9" y="{y-5}" width="10" height="10" fill="none" stroke="{colour}"/>'
        parts.append(
            mark
            + f'<text x="28" y="{y + 3}" fill="{dark["ink"]}" font-size="8" '
            f'font-family="{TOKENS["type"]["mono"]}">{label}</text>'
        )
        y += 16
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 80" '
        'role="img" aria-label="Quatopsy result states">'
        + "".join(parts)
        + "</svg>\n"
    )


def overlay_private_svg() -> str:
    dark = TOKENS["palette"]["dark"]
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 280 36" '
        'role="img" aria-label="Private research overlay">\n'
        f'<rect width="280" height="36" fill="{dark["surface"]}"/>'
        f'<text x="12" y="22" fill="{dark["muted"]}" font-size="11" '
        f'font-family="{TOKENS["type"]["ui"]}">Private research overlay. Not a maturity mark.</text>\n'
        "</svg>\n"
    )


def tokens_css() -> str:
    dark = TOKENS["palette"]["dark"]
    light = TOKENS["palette"]["light"]
    hc = TOKENS["palette"]["high_contrast"]
    lines = [":root {"]
    for key, value in dark.items():
        lines.append(f"  --qp-{key}: {value};")
    lines.append("}")
    lines.append("@media (prefers-color-scheme: light) { :root {")
    for key, value in light.items():
        lines.append(f"  --qp-{key}: {value};")
    lines.append("} }")
    lines.append("@media (prefers-contrast: more) { :root {")
    for key, value in hc.items():
        lines.append(f"  --qp-{key}: {value};")
    lines.append("} }")
    lines.append(
        "@media (prefers-reduced-motion: reduce) { *, *::before, *::after "
        "{ animation: none !important; transition: none !important; } }\n"
    )
    return "\n".join(lines)


def encode_png(width: int, height: int, pixels: bytes) -> bytes:
    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    raw = b""
    stride = width * 4
    for row in range(height):
        start = row * stride
        raw += b"\x00" + pixels[start : start + stride]
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def coverage(dist: float, width: float) -> float:
    return max(0.0, min(1.0, 0.5 - dist / width))


def mix(bg: tuple[int, int, int], fg: tuple[int, int, int], a: float) -> tuple[int, int, int]:
    return tuple(int(b * (1 - a) + f * a) for b, f in zip(bg, fg))  # type: ignore[return-value]


def raster_symbol(size: int, ink: str, accent: str, bg: str, *, small: bool = False) -> bytes:
    g = antipodal_geometry(32)
    scale = size / 32.0
    background = hex_rgb(bg)
    ink_rgb = hex_rgb(ink)
    accent_rgb = hex_rgb(accent)
    pixels = bytearray(size * size * 4)
    half = g["stroke"] / 2
    a1 = math.atan2(g["y1"] - g["cy"], g["x1"] - g["cx"])
    a2 = math.atan2(g["y2"] - g["cy"], g["x2"] - g["cx"])
    span = (a2 - a1) % (2 * math.pi)

    def plot(x: int, y: int, colour: tuple[int, int, int], alpha: float) -> None:
        if alpha <= 0 or x < 0 or y < 0 or x >= size or y >= size:
            return
        i = (y * size + x) * 4
        blended = mix((pixels[i], pixels[i + 1], pixels[i + 2]), colour, min(1.0, alpha))
        pixels[i : i + 3] = bytes(blended)
        pixels[i + 3] = 255

    for y in range(size):
        for x in range(size):
            i = (y * size + x) * 4
            pixels[i : i + 3] = bytes(background)
            pixels[i + 3] = 255
            px, py = (x + 0.5) / scale, (y + 0.5) / scale
            radial = math.hypot(px - g["cx"], py - g["cy"])
            d_ring = abs(radial - g["r"])
            ring_a = coverage(d_ring, 0.55) if d_ring <= half + 0.8 else 0.0
            on_arc = False
            if ring_a > 0 and not small:
                ang = math.atan2(py - g["cy"], px - g["cx"])
                delta = (ang - a1) % (2 * math.pi)
                on_arc = delta <= span
            if ring_a > 0:
                plot(x, y, accent_rgb if on_arc else ink_rgb, ring_a)
            for dx, dy in ((g["x1"], g["y1"]), (g["x2"], g["y2"])):
                d_dot = math.hypot(px - dx, py - dy)
                plot(x, y, accent_rgb, coverage(d_dot - g["dot"], 0.45))
    return bytes(pixels)


def raster_lifted(size: int) -> bytes:
    dark = TOKENS["palette"]["dark"]
    bg, accent, ink, tick = hex_rgb(dark["bg"]), hex_rgb(dark["accent"]), hex_rgb(dark["ink"]), hex_rgb(dark["refused"])
    pixels = bytearray(size * size * 4)
    scale = size / 32.0

    def bezier(t: float) -> tuple[float, float]:
        u = 1 - t
        x = u * u * 4 + 2 * u * t * 16 + t * t * 28
        y = u * u * 24 + 2 * u * t * 8 + t * t * 24
        return x, y

    for y in range(size):
        for x in range(size):
            i = (y * size + x) * 4
            pixels[i : i + 3] = bytes(bg)
            pixels[i + 3] = 255
            px, py = (x + 0.5) / scale, (y + 0.5) / scale
            d = min(math.hypot(px - bx, py - by) for bx, by in (bezier(t / 80) for t in range(81)))
            a = coverage(d - 0.85, 0.5)
            if a > 0:
                blended = mix((pixels[i], pixels[i + 1], pixels[i + 2]), accent, a)
                pixels[i : i + 3] = bytes(blended)
            d_tick = min(
                math.hypot(px - (18 + 0.2 * s), py - (11 - 0.42 * s)) for s in range(12)
            )
            a_tick = coverage(d_tick - 0.7, 0.45)
            if a_tick > 0:
                blended = mix((pixels[i], pixels[i + 1], pixels[i + 2]), tick, a_tick)
                pixels[i : i + 3] = bytes(blended)
            a_dot = coverage(math.hypot(px - 18, py - 11) - 1.7, 0.45)
            if a_dot > 0:
                blended = mix((pixels[i], pixels[i + 1], pixels[i + 2]), ink, a_dot)
                pixels[i : i + 3] = bytes(blended)
    return bytes(pixels)


def raster_lens(size: int) -> bytes:
    dark = TOKENS["palette"]["dark"]
    bg, ink, accent, tick = (
        hex_rgb(dark["bg"]),
        hex_rgb(dark["ink"]),
        hex_rgb(dark["accent"]),
        hex_rgb(dark["refused"]),
    )
    pixels = bytearray(size * size * 4)
    scale = size / 32.0
    for y in range(size):
        for x in range(size):
            i = (y * size + x) * 4
            pixels[i : i + 3] = bytes(bg)
            pixels[i + 3] = 255
            px, py = (x + 0.5) / scale, (y + 0.5) / scale
            for cx, colour in ((13, ink), (19, accent)):
                a = coverage(abs(math.hypot(px - cx, py - 16) - 8.2) - 0.8, 0.5)
                if a > 0:
                    blended = mix((pixels[i], pixels[i + 1], pixels[i + 2]), colour, a)
                    pixels[i : i + 3] = bytes(blended)
            if 9.2 <= py <= 22.8:
                a = coverage(abs(px - 16) - 0.7, 0.45)
                if a > 0:
                    blended = mix((pixels[i], pixels[i + 1], pixels[i + 2]), tick, a)
                    pixels[i : i + 3] = bytes(blended)
    return bytes(pixels)


def raster_social(width: int, height: int) -> bytes:
    dark = TOKENS["palette"]["dark"]
    bg = hex_rgb(dark["bg"])
    pixels = bytearray(width * height * 4)
    mark_size = 280
    mark = raster_symbol(mark_size, dark["ink"], dark["accent"], dark["bg"])
    for i in range(0, len(pixels), 4):
        pixels[i : i + 3] = bytes(bg)
        pixels[i + 3] = 255
    ox, oy = (width - mark_size) // 2, (height - mark_size) // 2
    for y in range(mark_size):
        for x in range(mark_size):
            s = (y * mark_size + x) * 4
            d = ((oy + y) * width + (ox + x)) * 4
            pixels[d : d + 4] = mark[s : s + 4]
    return bytes(pixels)


def simulate(rgb: tuple[int, int, int], mode: str) -> tuple[int, int, int]:
    r, g, b = [c / 255.0 for c in rgb]
    # Brettel-inspired coarse matrices for artefact checks, not clinical accuracy.
    matrices = {
        "protan": ((0.152, 1.053, -0.205), (0.115, 0.786, 0.099), (-0.004, -0.048, 1.052)),
        "deutan": ((0.367, 0.861, -0.228), (0.280, 0.673, 0.047), (-0.012, 0.043, 0.969)),
        "tritan": ((1.256, -0.077, -0.179), (-0.078, 0.931, 0.148), (0.005, 0.048, 0.947)),
        "gray": (
            (0.2126, 0.7152, 0.0722),
            (0.2126, 0.7152, 0.0722),
            (0.2126, 0.7152, 0.0722),
        ),
    }
    m = matrices[mode]
    out = [m[i][0] * r + m[i][1] * g + m[i][2] * b for i in range(3)]
    return tuple(max(0, min(255, int(v * 255))) for v in out)  # type: ignore[return-value]


def simulation_strip() -> bytes:
    dark = TOKENS["palette"]["dark"]
    width, height = 240, 48
    pixels = bytearray(width * height * 4)
    colours = [
        hex_rgb(dark["pass"]),
        hex_rgb(dark["findings"]),
        hex_rgb(dark["refused"]),
        hex_rgb(dark["error"]),
    ]
    modes = ["protan", "deutan", "tritan", "gray"]
    for row, mode in enumerate(modes):
        for col, colour in enumerate(colours):
            sim = simulate(colour, mode)
            for y in range(12):
                for x in range(60):
                    i = ((row * 12 + y) * width + col * 60 + x) * 4
                    pixels[i : i + 3] = bytes(sim)
                    pixels[i + 3] = 255
    return bytes(pixels)


def svg_is_safe(text: str) -> list[str]:
    errors = []
    lowered = re.sub(r'xmlns="http://www.w3.org/2000/svg"', "", text).lower()
    for needle in (
        "<script",
        "javascript:",
        "onload=",
        "onclick=",
        "http://",
        "https://",
        "<foreignobject",
        "<iframe",
        "xlink:href",
        "data:image",
    ):
        if needle in lowered:
            errors.append(f"unsafe SVG fragment {needle}")
    if "<?xml" not in text:
        errors.append("SVG missing XML declaration")
    return errors


def tree() -> dict[str, bytes]:
    dark = TOKENS["palette"]["dark"]
    light = TOKENS["palette"]["light"]
    mono = TOKENS["palette"]["mono"]
    rev = TOKENS["palette"]["reversed"]
    files: dict[str, bytes] = {}

    def put(rel: str, text: str | bytes) -> None:
        files[rel] = text.encode("utf-8") if isinstance(text, str) else text

    put("tokens/tokens.json", json.dumps(TOKENS, indent=2) + "\n")
    put("tokens/tokens.css", tokens_css())
    put("source/quatopsy-symbol.svg", symbol_svg(dark["ink"], dark["accent"]))
    put("source/quatopsy-symbol-small.svg", symbol_svg(dark["ink"], dark["accent"], small=True))
    put("source/quatopsy-symbol-mono.svg", symbol_svg(mono["ink"], mono["accent"]))
    put(
        "source/quatopsy-symbol-reversed.svg",
        symbol_svg(rev["ink"], rev["accent"], bg=rev["bg"]),
    )
    put("source/quatopsy-wordmark.svg", wordmark_svg(dark["ink"]))
    put(
        "source/quatopsy-lockup-horizontal.svg",
        lockup_horizontal_svg(dark["ink"], dark["accent"]),
    )
    put(
        "source/quatopsy-lockup-horizontal-mono.svg",
        lockup_horizontal_svg(mono["ink"], mono["accent"]),
    )
    put(
        "source/quatopsy-lockup-horizontal-reversed.svg",
        lockup_horizontal_svg(rev["ink"], rev["accent"], rev["bg"]),
    )
    put(
        "source/quatopsy-lockup-stacked.svg",
        lockup_stacked_svg(dark["ink"], dark["accent"]),
    )
    put("source/quatopsy-lockup-light.svg", lockup_horizontal_svg(light["ink"], light["accent"]))
    put("source/directions/lifted-path.svg", direction_lifted_path())
    put(
        "source/directions/antipodal-paired-point.svg",
        symbol_svg(dark["ink"], dark["accent"], bg=dark["bg"]),
    )
    put("source/directions/quotient-lens.svg", direction_quotient_lens())
    put("source/icons/result-pass.svg", result_icon_svg("pass", dark["pass"]))
    put("source/icons/result-findings.svg", result_icon_svg("findings", dark["findings"]))
    put("source/icons/result-refused.svg", result_icon_svg("refused", dark["refused"]))
    put("source/icons/result-error.svg", result_icon_svg("error", dark["error"]))
    put("templates/diagram-workflow.svg", workflow_diagram_svg())
    put("templates/chart-states.svg", chart_states_svg())
    put(
        "templates/release-lockup.svg",
        lockup_horizontal_svg(dark["ink"], dark["accent"], dark["bg"]),
    )
    put("templates/overlay-private-research.svg", overlay_private_svg())
    put(
        "exports/plain-lockup.txt",
        f"Quatopsy\n{TAGLINE}\n{CATEGORY}\n",
    )
    put(
        "LICENSES/README.md",
        (
            "Original Quatopsy marks, tokens, diagrams, and rasters are Apache-2.0.\n"
            "Typography uses the system UI and monospace stacks. IBM Plex Sans and "
            "IBM Plex Mono are recommended open fonts and are not redistributed here.\n"
        ),
    )

    rasters = {
        "exports/favicon-16.png": (16, True),
        "exports/favicon-32.png": (32, False),
        "exports/apple-touch-180.png": (180, False),
        "exports/avatar-512.png": (512, False),
        "exports/app-icon-512.png": (512, False),
    }
    for rel, (size, small) in rasters.items():
        pixels = raster_symbol(size, dark["ink"], dark["accent"], dark["bg"], small=small)
        put(rel, encode_png(size, size, pixels))
    put("exports/social-og-1200x630.png", encode_png(1200, 630, raster_social(1200, 630)))
    put("exports/simulations/result-cvd.png", encode_png(240, 48, simulation_strip()))
    put("exports/study-lifted-path.png", encode_png(512, 512, raster_lifted(512)))
    put("exports/study-quotient-lens.png", encode_png(512, 512, raster_lens(512)))
    return files


def manifest_for(files: dict[str, bytes]) -> bytes:
    entries = []
    for rel in sorted(files):
        data = files[rel]
        entry = {
            "path": rel,
            "sha256": sha256(data),
            "bytes": len(data),
            "licence": "Apache-2.0",
            "creator": "scripts/brandkit.py",
            "tool": "scripts/brandkit.py",
            "allowed_use": "product identity, documentation, viewer, and release presentation",
            "export_command": "python3 scripts/brandkit.py export",
        }
        if rel.endswith(".png"):
            if "favicon-16" in rel:
                entry["dimensions"] = [16, 16]
            elif "favicon-32" in rel:
                entry["dimensions"] = [32, 32]
            elif "apple-touch" in rel:
                entry["dimensions"] = [180, 180]
            elif "512" in rel:
                entry["dimensions"] = [512, 512]
            elif "social" in rel:
                entry["dimensions"] = [1200, 630]
            elif "cvd" in rel:
                entry["dimensions"] = [240, 48]
            elif "study-" in rel:
                entry["dimensions"] = [512, 512]
            entry["colour_space"] = "sRGB"
        entries.append(entry)
    doc = {
        "schema": "quatopsy.brand-manifest/1",
        "brand_version": BRAND_VERSION,
        "direction": "antipodal-paired-point",
        "entries": entries,
    }
    return (json.dumps(doc, indent=2) + "\n").encode("utf-8")


def export(dest: Path = BRAND) -> None:
    files = tree()
    files["BRAND_ASSET_MANIFEST.json"] = manifest_for(files)
    if dest.exists():
        for path in dest.rglob("*"):
            if path.is_file():
                path.unlink()
    for rel, data in files.items():
        path = dest / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    print(f"brandkit: wrote {len(files)} files under {dest.relative_to(ROOT)}")


def check() -> int:
    expected = tree()
    expected["BRAND_ASSET_MANIFEST.json"] = manifest_for(
        {k: v for k, v in expected.items() if k != "BRAND_ASSET_MANIFEST.json"}
    )
    errors: list[str] = []
    if not BRAND.exists():
        errors.append("assets/brand is missing")
        print("\n".join(errors), file=sys.stderr)
        return 1
    actual = {
        str(path.relative_to(BRAND)): path.read_bytes()
        for path in BRAND.rglob("*")
        if path.is_file()
    }
    if set(actual) != set(expected):
        extra = sorted(set(actual) - set(expected))
        missing = sorted(set(expected) - set(actual))
        if extra:
            errors.append("unexpected brand files: " + ", ".join(extra))
        if missing:
            errors.append("missing brand files: " + ", ".join(missing))
    for rel, data in expected.items():
        if actual.get(rel) != data:
            errors.append(f"drift: {rel}")
        if rel.endswith(".svg"):
            errors.extend(f"{rel}: {item}" for item in svg_is_safe(data.decode("utf-8")))
        if rel.endswith(".png") and len(data) > 900_000:
            errors.append(f"{rel}: raster exceeds 900kB")
        if rel not in CANONICAL_SKIP_MATURITY and rel.endswith((".svg", ".json", ".css", ".txt", ".md")):
            text = data.decode("utf-8", errors="replace")
            if MATURITY_RE.search(text) and "overlay" not in rel:
                errors.append(f"{rel}: maturity term in canonical brand surface")
    dark = TOKENS["palette"]["dark"]
    light = TOKENS["palette"]["light"]
    for name, fg, bg in (
        ("dark ink", dark["ink"], dark["bg"]),
        ("dark muted", dark["muted"], dark["bg"]),
        ("dark accent", dark["accent"], dark["bg"]),
        ("dark pass", dark["pass"], dark["bg"]),
        ("dark findings", dark["findings"], dark["bg"]),
        ("dark refused", dark["refused"], dark["bg"]),
        ("light ink", light["ink"], light["bg"]),
        ("light accent", light["accent"], light["bg"]),
        ("light findings", light["findings"], light["bg"]),
    ):
        ratio = contrast_ratio(fg, bg)
        if ratio < 4.5:
            errors.append(f"contrast {name} {ratio:.2f} < 4.5")
    symbol = expected["source/quatopsy-symbol.svg"]
    overlay = expected["templates/overlay-private-research.svg"]
    if b"Private research overlay" not in overlay:
        errors.append("overlay template missing overlay copy")
    if b"Private research overlay" in symbol:
        errors.append("canonical symbol contains overlay copy")
    viewer_css = (ROOT / "viewer" / "viewer.css").read_text(encoding="utf-8")
    for token in (dark["bg"], dark["ink"], dark["accent"]):
        if token not in viewer_css:
            errors.append(f"viewer.css missing token {token}")
    viewer_html = (ROOT / "viewer" / "index.html").read_text(encoding="utf-8")
    if 'aria-label="Quatopsy"' not in viewer_html:
        errors.append("viewer mark missing accessible name")
    if "<script" in viewer_html.lower() and "viewer.js" not in viewer_html:
        errors.append("unexpected viewer script")
    if TAGLINE not in (ROOT / "crates" / "quatopsy-cli" / "src" / "main.rs").read_text(
        encoding="utf-8"
    ):
        errors.append("CLI about text missing tagline")
    if errors:
        print("brandkit check failed:", file=sys.stderr)
        for item in errors:
            print(f"  {item}", file=sys.stderr)
        return 1
    print(f"brandkit: {len(expected)} files, contrast and SVG checks passed")
    return 0


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in {"export", "check"}:
        print("usage: python3 scripts/brandkit.py export|check", file=sys.stderr)
        return 2
    if sys.argv[1] == "export":
        export()
        return 0
    return check()


if __name__ == "__main__":
    raise SystemExit(main())
