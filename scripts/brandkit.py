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
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[1]
BRAND = ROOT / "assets" / "brand"
BRAND_VERSION = "quatopsy.brand/2"
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
            "bg": "#09070d",
            "surface": "#110d18",
            "ink": "#fff1d6",
            "muted": "#b9aebf",
            "accent": "#c778ff",
            "accent_secondary": "#ec48c6",
            "inspection": "#f5c96a",
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
            "accent": "#7133a5",
            "accent_secondary": "#9e1a77",
            "inspection": "#6e4b00",
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
        "wordmark": "Space Grotesk SemiBold, outlined in canonical lockups",
        "mono": 'ui-monospace, "SFMono-Regular", Menlo, Consolas, monospace',
        "recommended_open_ui": "Space Grotesk",
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
        "direction": "woven-lift",
        "clear_space_units": 6,
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


def antipodal_direction_svg(ink: str, accent: str, *, bg: str | None = None) -> str:
    g = antipodal_geometry()
    dots = (
        f'<circle cx="{g["x1"]:.3f}" cy="{g["y1"]:.3f}" r="{g["dot"]:.3f}" fill="{accent}"/>'
        f'<circle cx="{g["x2"]:.3f}" cy="{g["y2"]:.3f}" r="{g["dot"]:.3f}" fill="{accent}"/>'
    )
    ring = (
        f'<circle cx="{g["cx"]:.3f}" cy="{g["cy"]:.3f}" r="{g["r"]:.3f}" '
        f'fill="none" stroke="{ink}" stroke-width="{g["stroke"]:.3f}"/>'
    )
    chord = (
        f'<path d="M {g["x1"]:.3f} {g["y1"]:.3f} A {g["r"]:.3f} {g["r"]:.3f} 0 0 1 '
        f'{g["x2"]:.3f} {g["y2"]:.3f}" fill="none" stroke="{accent}" '
        f'stroke-width="{g["stroke"]:.3f}" stroke-linecap="round"/>'
    )
    return svg_wrap(ring + chord + dots, bg=bg)


RIBBON_A = "M4.7 14.0 C3.1 9.0 6.0 5.0 11.0 4.8 C16.0 4.6 20.9 7.8 23.8 12.2 C19.8 9.4 15.3 7.1 11.3 7.3 C7.7 7.5 6.2 10.2 7.0 13.8 C8.0 18.1 12.5 22.8 18.6 25.9 C12.2 24.3 6.4 19.9 4.7 14.0 Z"
RIBBON_B = "M20.2 5.8 C25.0 7.1 28.0 11.3 27.7 16.2 C27.3 21.4 22.4 24.5 16.3 25.8 C20.5 23.0 24.0 20.2 25.1 16.6 C26.2 13.0 24.3 8.9 20.2 5.8 Z M17.8 25.2 C21.1 26.2 23.8 25.4 26.3 23.1 C23.8 26.8 20.7 28.0 17.7 27.2 C14.7 26.4 12.3 25.7 10.4 24.3 C13.0 24.7 15.5 24.3 17.8 25.2 Z"
RIBBON_C = "M6.0 23.1 C9.7 18.1 15.7 14.0 21.7 14.1 C23.5 14.1 24.8 14.7 25.6 15.6 C21.1 15.4 16.8 16.6 13.0 18.5 C9.8 20.1 7.3 22.1 6.0 23.1 Z"


def symbol_body(*, mono: str | None = None, small: bool = False) -> str:
    if mono:
        return f'<path d="{RIBBON_A}" fill="{mono}"/><path d="{RIBBON_B}" fill="{mono}"/><path d="{RIBBON_C}" fill="{mono}"/>'
    if small:
        return (
            '<g transform="translate(-4,-4) scale(1.25)">'
            f'<path d="{RIBBON_A}" fill="#b342e8"/>'
            f'<path d="{RIBBON_B}" fill="#d52eb2"/>'
            '</g>'
        )
    return (
        '<defs>'
        '<linearGradient id="qa" gradientUnits="userSpaceOnUse" x1="5" y1="6" x2="20" y2="26"><stop stop-color="#8b5cf6"/><stop offset="0.48" stop-color="#b445eb"/><stop offset="1" stop-color="#d946ef"/></linearGradient>'
        '<linearGradient id="qb" gradientUnits="userSpaceOnUse" x1="12" y1="26" x2="27" y2="8"><stop stop-color="#58104f"/><stop offset="0.52" stop-color="#b21eaa"/><stop offset="1" stop-color="#ec48c6"/></linearGradient>'
        '<linearGradient id="qc" gradientUnits="userSpaceOnUse" x1="6" y1="22" x2="26" y2="15"><stop stop-color="#f5c96a"/><stop offset="0.45" stop-color="#fff1d6"/><stop offset="1" stop-color="#fff8e8"/></linearGradient>'
        '</defs>'
        f'<path d="{RIBBON_A}" fill="url(#qa)"/>'
        f'<path d="{RIBBON_B}" fill="url(#qb)"/>'
        f'<path d="{RIBBON_C}" fill="url(#qc)"/>'
    )


def symbol_svg(ink: str, accent: str, *, small: bool = False, bg: str | None = None) -> str:
    del accent
    mono = ink if ink in {"#111111", "#f4f4f4"} else None
    return svg_wrap(symbol_body(mono=mono, small=small), bg=bg)


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


def wordmark_paths() -> str:
    # Space Grotesk SemiBold 600 outlines generated once from the upstream
    # font with SHA-256 acad6de1fc93436f5c0f1f4137751ef04f1aea3063e7036535970ffcfbd79f72.
    return (
        '<path d="M6.068251098632812 22.252Q3.7205991210937497 22.252 2.332348022460937 20.947Q0.9440969238281249 19.642 0.9440969238281249 17.196251098632814V14.203748901367188Q0.9440969238281249 11.758000000000001 2.332348022460937 10.453000000000001Q3.7205991210937497 9.148000000000001 6.068251098632812 9.148000000000001Q8.415903076171874 9.148000000000001 9.804154174804687 10.453000000000001Q11.1924052734375 11.758000000000001 11.1924052734375 14.203748901367188V17.196251098632814Q11.1924052734375 19.642 9.804154174804687 20.947Q8.415903076171874 22.252 6.068251098632812 22.252ZM6.068251098632812 20.31698681640625Q7.475845825195312 20.31698681640625 8.25321807861328 19.49259033203125Q9.030590332031249 18.66819384765625 9.030590332031249 17.26195153808594V14.138048461914064Q9.030590332031249 12.73180615234375 8.25321807861328 11.90740966796875Q7.475845825195312 11.083013183593751 6.068251098632812 11.083013183593751Q4.6786563720703125 11.083013183593751 3.8922841186523436 11.90740966796875Q3.1059118652343747 12.73180615234375 3.1059118652343747 14.138048461914064V17.26195153808594Q3.1059118652343747 18.66819384765625 3.8922841186523436 19.49259033203125Q4.6786563720703125 20.31698681640625 6.068251098632812 20.31698681640625ZM6.871052856445312 25.158105712890624Q6.042599121093749 25.158105712890624 5.529821594238281 24.645328186035158Q5.0170440673828125 24.13255065917969 5.0170440673828125 23.268096923828125V22.0H7.116308349609374V22.930145385742186Q7.116308349609374 23.47014538574219 7.620308349609375 23.47014538574219H8.71380615234375V25.158105712890624Z"/>'
        '<path d="M20.16740087890625 22.252Q18.654048461914062 22.252 17.56279736328125 21.700748901367184Q16.47154626464844 21.149497802734373 15.888121154785157 20.110671813964842Q15.304696044921874 19.07184582519531 15.304696044921874 17.625546264648438V9.4H17.466510986328124V17.684947143554687Q17.466510986328124 18.931440551757813 18.16445812988281 19.62421368408203Q18.8624052734375 20.31698681640625 20.16740087890625 20.31698681640625Q21.472396484375 20.31698681640625 22.17034362792969 19.62421368408203Q22.868290771484375 18.931440551757813 22.868290771484375 17.684947143554687V9.4H25.030105712890624V17.625546264648438Q25.030105712890624 19.07184582519531 24.448255493164062 20.110671813964842Q23.8664052734375 21.149497802734373 22.773579284667967 21.700748901367184Q21.680753295898437 22.252 20.16740087890625 22.252Z"/>'
        '<path d="M28.565497802734377 22.0 32.019237915039064 9.4H35.793863403320316L39.247603515625 22.0H37.01693835449219L36.263643188476564 19.127207031250002H31.54945812988281L30.79616296386719 22.0ZM32.066510986328126 17.13279296875H35.746590332031246L34.059101318359374 10.752259887695313H33.754Z"/>'
        '<path d="M45.81509252929688 22.0V11.364713623046876H42.04859912109375V9.4H51.74340087890625V11.364713623046876H47.976907470703125V22.0Z"/>'
        '<path d="M60.256251098632816 22.252Q57.90859912109375 22.252 56.52034802246094 20.947Q55.13209692382813 19.642 55.13209692382813 17.196251098632814V14.203748901367188Q55.13209692382813 11.758000000000001 56.52034802246094 10.453000000000001Q57.90859912109375 9.148000000000001 60.256251098632816 9.148000000000001Q62.603903076171875 9.148000000000001 63.99215417480469 10.453000000000001Q65.3804052734375 11.758000000000001 65.3804052734375 14.203748901367188V17.196251098632814Q65.3804052734375 19.642 63.99215417480469 20.947Q62.603903076171875 22.252 60.256251098632816 22.252ZM60.256251098632816 20.31698681640625Q61.66384582519532 20.31698681640625 62.44121807861329 19.49259033203125Q63.218590332031255 18.66819384765625 63.218590332031255 17.26195153808594V14.138048461914064Q63.218590332031255 12.73180615234375 62.44121807861329 11.90740966796875Q61.66384582519532 11.083013183593751 60.256251098632816 11.083013183593751Q58.86665637207032 11.083013183593751 58.080284118652344 11.90740966796875Q57.29391186523438 12.73180615234375 57.29391186523438 14.138048461914064V17.26195153808594Q57.29391186523438 18.66819384765625 58.080284118652344 19.49259033203125Q58.86665637207032 20.31698681640625 60.256251098632816 20.31698681640625Z"/>'
        '<path d="M69.57099560546875 22.0V9.4H74.66365197753908Q75.83905285644532 9.4 76.74490307617188 9.867550659179688Q77.65075329589844 10.335101318359376 78.16532818603517 11.186502197265625Q78.67990307617188 12.037903076171876 78.67990307617188 13.216453735351562V13.462154174804688Q78.67990307617188 14.625854614257813 78.14890307617188 15.486255493164062Q77.61790307617188 16.346656372070314 76.71205285644533 16.812632141113284Q75.80620263671875 17.27860791015625 74.66365197753908 17.27860791015625H71.732810546875V22.0ZM71.732810546875 15.313894287109376H74.44855065917969Q75.38274450683595 15.313894287109376 75.9504163208008 14.811246704101563Q76.51808813476563 14.30859912109375 76.51808813476563 13.429303955078126V13.249303955078126Q76.51808813476563 12.366859008789064 75.95356610107423 11.865786315917969Q75.38904406738283 11.364713623046876 74.44855065917969 11.364713623046876H71.732810546875Z"/>'
        '<path d="M86.75489868164063 22.252Q85.31579736328126 22.252 84.20924670410157 21.73787445068359Q83.10269604492188 21.223748901367188 82.47584582519532 20.25017401123047Q81.84899560546876 19.27659912109375 81.84899560546876 17.88340087890625V17.40774890136719H83.98111010742188V17.88340087890625Q83.98111010742188 19.124493408203126 84.7350814819336 19.73874011230469Q85.48905285644531 20.35298681640625 86.75489868164063 20.35298681640625Q88.03874450683594 20.35298681640625 88.68426654052735 19.824464782714845Q89.32978857421875 19.29594274902344 89.32978857421875 18.47379736328125Q89.32978857421875 17.910850219726562 89.020189453125 17.56322686767578Q88.71059033203126 17.215603515625 88.12784143066406 16.996004394531248Q87.54509252929688 16.7764052734375 86.72564318847657 16.590105712890626L86.18250219726563 16.465458129882812Q84.9337489013672 16.18060791015625 84.02722247314455 15.745907470703125Q83.12069604492189 15.31120703125 82.63042071533204 14.602231262207031Q82.14014538574219 13.893255493164062 82.14014538574219 12.763303955078126Q82.14014538574219 11.633352416992189 82.68599560546875 10.82537664794922Q83.23184582519532 10.017400878906251 84.21982159423828 9.582700439453127Q85.20779736328126 9.148000000000001 86.53889868164063 9.148000000000001Q87.87 9.148000000000001 88.91355065917969 9.602275329589844Q89.95710131835938 10.056550659179688 90.55695153808594 10.952951538085937Q91.1568017578125 11.849352416992188 91.1568017578125 13.198453735351563V13.792H89.02468725585938V13.198453735351563Q89.02468725585938 12.43120703125 88.71891412353516 11.960284118652345Q88.41314099121094 11.489361206054689 87.85334362792969 11.26818719482422Q87.29354626464844 11.047013183593752 86.53889868164063 11.047013183593752Q85.41795153808594 11.047013183593752 84.84510571289063 11.486660766601563Q84.27225988769531 11.926308349609375 84.27225988769531 12.703903076171876Q84.27225988769531 13.222299560546876 84.53415856933594 13.568348022460938Q84.79605725097657 13.914396484375 85.30883038330079 14.139845825195312Q85.821603515625 14.365295166015626 86.59650219726564 14.536744506835937L87.13964318847657 14.661392089843751Q88.42664758300782 14.939942749023437 89.39572467041016 15.378918518066406Q90.3648017578125 15.817894287109375 90.91335241699218 16.544870056152345Q91.46190307617188 17.271845825195314 91.46190307617188 18.414396484375Q91.46190307617188 19.556947143554687 90.88320263671875 20.418922912597658Q90.30450219726563 21.280898681640625 89.2476762084961 21.766449340820312Q88.19085021972657 22.252 86.75489868164063 22.252Z"/>'
        '<path d="M98.6215947265625 22.0V17.54860791015625L94.28719824218751 9.4H96.70956384277345L99.54995153808595 14.957489013671875H99.85505285644533L102.69544055175783 9.4H105.11780615234376L100.78340966796875 17.54860791015625V22.0Z"/>'
    )


def wordmark_svg(ink: str, width: int = 108, height: int = 32) -> str:
    return (
        f'<?xml version="1.0" encoding="UTF-8"?>\n'
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" '
        f'role="img" aria-label="Quatopsy">\n'
        f'<g fill="{ink}">{wordmark_paths()}</g>\n'
        f"</svg>\n"
    )


def lockup_horizontal_svg(ink: str, accent: str, bg: str | None = None) -> str:
    symbol = symbol_svg(ink, accent).split("\n", 2)[-1].rsplit("</svg>", 1)[0]
    background = f'<rect width="156" height="32" fill="{bg}"/>' if bg else ""
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 156 32" '
        'role="img" aria-label="Quatopsy">\n'
        f"{background}"
        f'<g transform="translate(0,0)">{symbol}</g>'
        f'<g transform="translate(40,0)" fill="{ink}">{wordmark_paths()}</g>\n'
        "</svg>\n"
    )


def lockup_stacked_svg(ink: str, accent: str, bg: str | None = None) -> str:
    symbol = symbol_svg(ink, accent).split("\n", 2)[-1].rsplit("</svg>", 1)[0]
    background = f'<rect width="108" height="64" fill="{bg}"/>' if bg else ""
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 108 64" '
        'role="img" aria-label="Quatopsy">\n'
        f"{background}"
        f'<g transform="translate(38,0)">{symbol}</g>'
        f'<g transform="translate(1,32)" fill="{ink}">{wordmark_paths()}</g>\n'
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
        f'font-family=\'{TOKENS["type"]["ui"]}\'>{label}</text>'
    )
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 40" role="img" '
        f'aria-label="Quatopsy {label}">{body}</svg>\n'
    )


def workflow_diagram_svg() -> str:
    return '''<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 720" role="img" aria-labelledby="title desc">
<title id="title">Quatopsy local-first system architecture</title>
<desc id="desc">Recorded orientation evidence enters adapters and a canonical schema. The analysis kernel alone owns diagnostic verdicts. Repairs, reproducers, viewers, plans, controls, and a digest-bound investigation bundle remain separate local outputs with no physical command path.</desc>
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#09070d"/><stop offset="0.55" stop-color="#100a18"/><stop offset="1" stop-color="#160b20"/></linearGradient>
  <linearGradient id="violet" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#8b5cf6"/><stop offset="0.48" stop-color="#c778ff"/><stop offset="1" stop-color="#d946ef"/></linearGradient>
  <linearGradient id="cerise" x1="0" y1="1" x2="1" y2="0"><stop stop-color="#58104f"/><stop offset="0.55" stop-color="#b21eaa"/><stop offset="1" stop-color="#ec48c6"/></linearGradient>
  <linearGradient id="gold" x1="0" y1="0" x2="1" y2="0"><stop stop-color="#f5c96a"/><stop offset="0.55" stop-color="#fff1d6"/><stop offset="1" stop-color="#fff8e8"/></linearGradient>
  <radialGradient id="halo"><stop stop-color="#c778ff" stop-opacity="0.18"/><stop offset="1" stop-color="#c778ff" stop-opacity="0"/></radialGradient>
  <filter id="glow" x="-60%" y="-60%" width="220%" height="220%"><feGaussianBlur stdDeviation="8"/></filter>
  <marker id="arrow" markerWidth="10" markerHeight="10" refX="8" refY="5" orient="auto"><path d="M0 0 L10 5 L0 10 Z" fill="#8f8298"/></marker>
</defs>
<rect width="1200" height="720" rx="28" fill="url(#bg)"/>
<path d="M0 82 H1200 M0 640 H1200" stroke="#3a2b43" stroke-width="1"/>
<g opacity="0.18" stroke="#8b6d98" stroke-width="1"><path d="M80 118 H1120 M80 180 H1120 M80 242 H1120 M80 304 H1120 M80 366 H1120 M80 428 H1120 M80 490 H1120 M80 552 H1120"/><path d="M140 104 V610 M300 104 V610 M460 104 V610 M620 104 V610 M780 104 V610 M940 104 V610 M1100 104 V610"/></g>
<circle cx="600" cy="335" r="210" fill="url(#halo)"/>
<g transform="translate(38 21) scale(1.15)"><path d="M4.7 14 C3.1 9 6 5 11 4.8 C16 4.6 20.9 7.8 23.8 12.2 C19.8 9.4 15.3 7.1 11.3 7.3 C7.7 7.5 6.2 10.2 7 13.8 C8 18.1 12.5 22.8 18.6 25.9 C12.2 24.3 6.4 19.9 4.7 14 Z" fill="url(#violet)"/><path d="M20.2 5.8 C25 7.1 28 11.3 27.7 16.2 C27.3 21.4 22.4 24.5 16.3 25.8 C20.5 23 24 20.2 25.1 16.6 C26.2 13 24.3 8.9 20.2 5.8 Z M17.8 25.2 C21.1 26.2 23.8 25.4 26.3 23.1 C23.8 26.8 20.7 28 17.7 27.2 C14.7 26.4 12.3 25.7 10.4 24.3 C13 24.7 15.5 24.3 17.8 25.2 Z" fill="url(#cerise)"/><path d="M6 23.1 C9.7 18.1 15.7 14 21.7 14.1 C23.5 14.1 24.8 14.7 25.6 15.6 C21.1 15.4 16.8 16.6 13 18.5 C9.8 20.1 7.3 22.1 6 23.1 Z" fill="url(#gold)"/></g>
<text x="86" y="46" fill="#fff1d6" font-size="25" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif" letter-spacing="2">QUATOPSY</text>
<text x="245" y="45" fill="#b9aebf" font-size="14" font-family="ui-monospace, monospace" letter-spacing="1.2">LOCAL-FIRST ORIENTATION FORENSICS</text>
<text x="1150" y="44" text-anchor="end" fill="#f5c96a" font-size="12" font-family="ui-monospace, monospace">NO NETWORK / NO ACTUATOR I/O</text>

<text x="74" y="130" fill="#c778ff" font-size="12" font-weight="700" font-family="ui-monospace, monospace" letter-spacing="2">01 / RECORDED EVIDENCE</text>
<rect x="64" y="151" width="252" height="142" rx="18" fill="#110f17" stroke="#55415f" stroke-width="1.5"/>
<text x="88" y="184" fill="#fff1d6" font-size="19" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Orientation sources</text>
<g fill="#b9aebf" font-size="14" font-family="ui-monospace, monospace"><text x="88" y="218">CSV + declared manifest</text><text x="88" y="246">ROS / MCAP / SPICE CK</text><text x="88" y="274">TUBIN / IDS exports</text></g>

<text x="430" y="130" fill="#c778ff" font-size="12" font-weight="700" font-family="ui-monospace, monospace" letter-spacing="2">02 / CANONICAL BOUNDARY</text>
<rect x="420" y="151" width="300" height="142" rx="18" fill="#17101f" stroke="url(#violet)" stroke-width="2"/>
<text x="444" y="184" fill="#fff1d6" font-size="19" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Declared trajectory</text>
<text x="444" y="214" fill="#f5c96a" font-size="13" font-family="ui-monospace, monospace">quatopsy.manifest/1</text>
<g fill="#b9aebf" font-size="14" font-family="ui-monospace, monospace"><text x="444" y="245">frames + convention + units</text><text x="444" y="273">immutable bytes + bounded ingest</text></g>

<text x="824" y="130" fill="#c778ff" font-size="12" font-weight="700" font-family="ui-monospace, monospace" letter-spacing="2">03 / VERDICT OWNER</text>
<rect x="814" y="151" width="322" height="142" rx="18" fill="#17101f" stroke="url(#gold)" stroke-width="2"/>
<circle cx="1095" cy="188" r="44" fill="#c778ff" opacity="0.13" filter="url(#glow)"/>
<text x="838" y="184" fill="#fff1d6" font-size="19" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Conformance kernel</text>
<text x="838" y="214" fill="#f5c96a" font-size="13" font-family="ui-monospace, monospace">quatopsy.report/1</text>
<g fill="#b9aebf" font-size="14" font-family="ui-monospace, monospace"><text x="838" y="245">deterministic rules + oracles</text><text x="838" y="273">pass / findings / refused / error</text></g>

<path d="M316 222 H405" fill="none" stroke="#8f8298" stroke-width="2" marker-end="url(#arrow)"/><path d="M720 222 H799" fill="none" stroke="#8f8298" stroke-width="2" marker-end="url(#arrow)"/>
<text x="360" y="210" text-anchor="middle" fill="#8f8298" font-size="11" font-family="ui-monospace, monospace">adapt</text><text x="760" y="210" text-anchor="middle" fill="#8f8298" font-size="11" font-family="ui-monospace, monospace">analyse</text>

<text x="74" y="355" fill="#ec48c6" font-size="12" font-weight="700" font-family="ui-monospace, monospace" letter-spacing="2">OPTIONAL CANDIDATE PLANE</text>
<rect x="64" y="376" width="312" height="156" rx="18" fill="#110f17" stroke="#55415f" stroke-width="1.5"/>
<text x="88" y="410" fill="#fff1d6" font-size="18" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Plan + control</text>
<g fill="#b9aebf" font-size="14" font-family="ui-monospace, monospace"><text x="88" y="442">torque-limited candidate</text><text x="88" y="470">SIL / host PIL / loopback HIL</text><text x="88" y="498">MEKF / UKF / guidance / wheels</text></g>
<rect x="418" y="376" width="300" height="156" rx="18" fill="#110f17" stroke="#55415f" stroke-width="1.5" stroke-dasharray="6 6"/>
<text x="442" y="410" fill="#fff1d6" font-size="18" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Generated trajectory</text>
<text x="442" y="442" fill="#ec48c6" font-size="13" font-family="ui-monospace, monospace">NO SELF-ASSIGNED RESULT</text>
<g fill="#b9aebf" font-size="14" font-family="ui-monospace, monospace"><text x="442" y="474">separately named output</text><text x="442" y="502">returns through canonical analysis</text></g>
<path d="M376 454 H403" fill="none" stroke="#8f8298" stroke-width="2" marker-end="url(#arrow)"/><path d="M718 422 C780 422 762 300 880 300" fill="none" stroke="#ec48c6" stroke-width="2" stroke-dasharray="6 6" marker-end="url(#arrow)"/>

<text x="824" y="355" fill="#f5c96a" font-size="12" font-weight="700" font-family="ui-monospace, monospace" letter-spacing="2">04 / LOCAL EVIDENCE PRODUCTS</text>
<rect x="814" y="376" width="322" height="156" rx="18" fill="#110f17" stroke="#55415f" stroke-width="1.5"/>
<g font-family="ui-monospace, monospace" font-size="13"><rect x="838" y="402" width="120" height="34" rx="9" fill="#21182a" stroke="#725980"/><text x="898" y="424" text-anchor="middle" fill="#fff1d6">static viewer</text><rect x="972" y="402" width="140" height="34" rx="9" fill="#21182a" stroke="#725980"/><text x="1042" y="424" text-anchor="middle" fill="#fff1d6">repro slices</text><rect x="838" y="450" width="120" height="34" rx="9" fill="#21182a" stroke="#725980"/><text x="898" y="472" text-anchor="middle" fill="#fff1d6">repairs</text><rect x="972" y="450" width="140" height="34" rx="9" fill="#21182a" stroke="#725980"/><text x="1042" y="472" text-anchor="middle" fill="#fff1d6">evidence.json</text></g>
<text x="975" y="512" text-anchor="middle" fill="#b9aebf" font-size="12" font-family="ui-monospace, monospace">digest-bound / offline / reviewable</text>
<path d="M975 293 V361" fill="none" stroke="#8f8298" stroke-width="2" marker-end="url(#arrow)"/>

<rect x="64" y="574" width="1072" height="46" rx="12" fill="#130f19" stroke="#3f3048"/>
<circle cx="92" cy="597" r="7" fill="#fff1d6"/><text x="108" y="602" fill="#b9aebf" font-size="12" font-family="ui-monospace, monospace">observed bytes</text>
<path d="M244 597 H272" stroke="url(#violet)" stroke-width="5" stroke-linecap="round"/><text x="286" y="602" fill="#b9aebf" font-size="12" font-family="ui-monospace, monospace">canonical semantics</text>
<path d="M472 597 H500" stroke="#ec48c6" stroke-width="3" stroke-dasharray="5 4"/><text x="514" y="602" fill="#b9aebf" font-size="12" font-family="ui-monospace, monospace">candidate only</text>
<path d="M660 597 H688" stroke="url(#gold)" stroke-width="5" stroke-linecap="round"/><text x="702" y="602" fill="#b9aebf" font-size="12" font-family="ui-monospace, monospace">verdict boundary</text>
<text x="1110" y="602" text-anchor="end" fill="#f5c96a" font-size="12" font-family="ui-monospace, monospace">ADVISORY, NOT FLIGHT APPROVAL</text>
<text x="64" y="674" fill="#8f8298" font-size="12" font-family="ui-monospace, monospace">SOURCE INPUTS REMAIN READ-ONLY</text>
<text x="1136" y="674" text-anchor="end" fill="#8f8298" font-size="12" font-family="ui-monospace, monospace">PHYSICAL HARDWARE / HARD REAL-TIME / ORBIT DETERMINATION: REFUSED</text>
</svg>
'''


def workflow_diagram_narrow_svg() -> str:
    return '''<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 720 1180" role="img" aria-labelledby="title desc">
<title id="title">Quatopsy local-first system architecture for narrow displays</title>
<desc id="desc">Recorded orientation sources become a declared canonical trajectory. The conformance kernel alone owns verdicts. Local viewer, repair, reproducer, and evidence outputs remain separate. Optional plan and control candidates return through canonical analysis, with no network or physical command path.</desc>
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#09070d"/><stop offset="0.55" stop-color="#100a18"/><stop offset="1" stop-color="#160b20"/></linearGradient>
  <linearGradient id="violet" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#8b5cf6"/><stop offset="0.48" stop-color="#c778ff"/><stop offset="1" stop-color="#d946ef"/></linearGradient>
  <linearGradient id="cerise" x1="0" y1="1" x2="1" y2="0"><stop stop-color="#58104f"/><stop offset="0.55" stop-color="#b21eaa"/><stop offset="1" stop-color="#ec48c6"/></linearGradient>
  <linearGradient id="gold" x1="0" y1="0" x2="1" y2="0"><stop stop-color="#f5c96a"/><stop offset="0.55" stop-color="#fff1d6"/><stop offset="1" stop-color="#fff8e8"/></linearGradient>
  <marker id="arrow" markerWidth="10" markerHeight="10" refX="8" refY="5" orient="auto"><path d="M0 0 L10 5 L0 10 Z" fill="#8f8298"/></marker>
</defs>
<rect width="720" height="1180" rx="28" fill="url(#bg)"/>
<g opacity="0.16" stroke="#8b6d98"><path d="M40 120 H680 M40 220 H680 M40 320 H680 M40 420 H680 M40 520 H680 M40 620 H680 M40 720 H680 M40 820 H680 M40 920 H680 M40 1020 H680"/><path d="M120 100 V1060 M240 100 V1060 M360 100 V1060 M480 100 V1060 M600 100 V1060"/></g>
<g transform="translate(38 28) scale(1.25)"><path d="M4.7 14 C3.1 9 6 5 11 4.8 C16 4.6 20.9 7.8 23.8 12.2 C19.8 9.4 15.3 7.1 11.3 7.3 C7.7 7.5 6.2 10.2 7 13.8 C8 18.1 12.5 22.8 18.6 25.9 C12.2 24.3 6.4 19.9 4.7 14 Z" fill="url(#violet)"/><path d="M20.2 5.8 C25 7.1 28 11.3 27.7 16.2 C27.3 21.4 22.4 24.5 16.3 25.8 C20.5 23 24 20.2 25.1 16.6 C26.2 13 24.3 8.9 20.2 5.8 Z M17.8 25.2 C21.1 26.2 23.8 25.4 26.3 23.1 C23.8 26.8 20.7 28 17.7 27.2 C14.7 26.4 12.3 25.7 10.4 24.3 C13 24.7 15.5 24.3 17.8 25.2 Z" fill="url(#cerise)"/><path d="M6 23.1 C9.7 18.1 15.7 14 21.7 14.1 C23.5 14.1 24.8 14.7 25.6 15.6 C21.1 15.4 16.8 16.6 13 18.5 C9.8 20.1 7.3 22.1 6 23.1 Z" fill="url(#gold)"/></g>
<text x="92" y="59" fill="#fff1d6" font-size="28" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif" letter-spacing="2">QUATOPSY</text>
<text x="680" y="56" text-anchor="end" fill="#f5c96a" font-size="13" font-family="ui-monospace, monospace">LOCAL / OFFLINE / ADVISORY</text>

<text x="64" y="130" fill="#c778ff" font-size="14" font-weight="700" font-family="ui-monospace, monospace" letter-spacing="2">01 / RECORDED EVIDENCE</text>
<rect x="52" y="150" width="616" height="126" rx="18" fill="#110f17" stroke="#55415f" stroke-width="2"/>
<text x="80" y="190" fill="#fff1d6" font-size="23" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Orientation sources</text>
<text x="80" y="230" fill="#b9aebf" font-size="17" font-family="ui-monospace, monospace">CSV + ROS + MCAP + SPICE CK + TUBIN + IDS</text>
<text x="80" y="255" fill="#8f8298" font-size="15" font-family="ui-monospace, monospace">recorded bytes / declared format / read-only</text>
<path d="M360 276 V316" stroke="#8f8298" stroke-width="2" marker-end="url(#arrow)"/>

<text x="64" y="342" fill="#c778ff" font-size="14" font-weight="700" font-family="ui-monospace, monospace" letter-spacing="2">02 / CANONICAL BOUNDARY</text>
<rect x="52" y="362" width="616" height="126" rx="18" fill="#17101f" stroke="url(#violet)" stroke-width="3"/>
<text x="80" y="402" fill="#fff1d6" font-size="23" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Declared trajectory</text>
<text x="80" y="440" fill="#f5c96a" font-size="16" font-family="ui-monospace, monospace">quatopsy.manifest/1</text>
<text x="80" y="466" fill="#b9aebf" font-size="16" font-family="ui-monospace, monospace">frames + convention + units + bounded ingest</text>
<path d="M360 488 V528" stroke="#8f8298" stroke-width="2" marker-end="url(#arrow)"/>

<text x="64" y="554" fill="#f5c96a" font-size="14" font-weight="700" font-family="ui-monospace, monospace" letter-spacing="2">03 / SOLE VERDICT OWNER</text>
<rect x="52" y="574" width="616" height="136" rx="18" fill="#17101f" stroke="url(#gold)" stroke-width="3"/>
<text x="80" y="614" fill="#fff1d6" font-size="23" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Conformance kernel</text>
<text x="80" y="652" fill="#f5c96a" font-size="16" font-family="ui-monospace, monospace">quatopsy.report/1</text>
<text x="80" y="680" fill="#b9aebf" font-size="16" font-family="ui-monospace, monospace">pass / findings / refused / error</text>

<text x="64" y="765" fill="#ec48c6" font-size="14" font-weight="700" font-family="ui-monospace, monospace" letter-spacing="2">OPTIONAL CANDIDATES</text>
<rect x="52" y="785" width="292" height="136" rx="18" fill="#110f17" stroke="#55415f" stroke-width="2"/>
<text x="76" y="825" fill="#fff1d6" font-size="21" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Plan + control</text>
<text x="76" y="860" fill="#b9aebf" font-size="15" font-family="ui-monospace, monospace">software candidates</text>
<text x="76" y="890" fill="#ec48c6" font-size="14" font-family="ui-monospace, monospace">NO RESULT FIELD</text>
<rect x="376" y="785" width="292" height="136" rx="18" fill="#110f17" stroke="#55415f" stroke-width="2"/>
<text x="400" y="825" fill="#fff1d6" font-size="21" font-weight="700" font-family="ui-sans-serif, system-ui, sans-serif">Local products</text>
<text x="400" y="860" fill="#b9aebf" font-size="15" font-family="ui-monospace, monospace">viewer / repair / repro</text>
<text x="400" y="890" fill="#b9aebf" font-size="15" font-family="ui-monospace, monospace">digest-bound evidence</text>
<path d="M198 785 C198 735 260 735 300 710" fill="none" stroke="#ec48c6" stroke-width="2" stroke-dasharray="7 6" marker-end="url(#arrow)"/>
<path d="M522 710 V770" stroke="#8f8298" stroke-width="2" marker-end="url(#arrow)"/>

<rect x="52" y="968" width="616" height="98" rx="16" fill="#130f19" stroke="#3f3048"/>
<text x="80" y="1004" fill="#f5c96a" font-size="16" font-family="ui-monospace, monospace">ADVISORY, NOT FLIGHT APPROVAL</text>
<text x="80" y="1038" fill="#b9aebf" font-size="15" font-family="ui-monospace, monospace">NO NETWORK / NO PHYSICAL ACTUATOR I/O</text>
<text x="52" y="1125" fill="#8f8298" font-size="13" font-family="ui-monospace, monospace">SOURCE INPUTS REMAIN READ-ONLY</text>
<text x="668" y="1125" text-anchor="end" fill="#8f8298" font-size="13" font-family="ui-monospace, monospace">CANDIDATES RETURN THROUGH ANALYSIS</text>
</svg>
'''


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
            f'font-family=\'{TOKENS["type"]["mono"]}\'>{label}</text>'
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
        f'font-family=\'{TOKENS["type"]["ui"]}\'>Private research overlay. Not a maturity mark.</text>\n'
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
    forced = TOKENS["palette"]["forced_colour"]
    lines.append("@media (forced-colors: active) { :root {")
    for key, value in forced.items():
        lines.append(f"  --qp-{key}: {value};")
    lines.append("} }")
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


def inside_polygon(x: float, y: float, points: tuple[tuple[float, float], ...]) -> bool:
    inside = False
    previous = points[-1]
    for current in points:
        x1, y1 = previous
        x2, y2 = current
        if (y1 > y) != (y2 > y):
            cross = (x2 - x1) * (y - y1) / (y2 - y1) + x1
            if x < cross:
                inside = not inside
        previous = current
    return inside


def flatten_cubic_path(path: str, steps: int = 32) -> tuple[tuple[tuple[float, float], ...], ...]:
    tokens = re.findall(r"[MCZ]|-?\d+(?:\.\d+)?", path)
    polygons: list[tuple[tuple[float, float], ...]] = []
    points: list[tuple[float, float]] = []
    cursor = (0.0, 0.0)
    i = 0
    while i < len(tokens):
        command = tokens[i]
        i += 1
        if command == "M":
            if points:
                polygons.append(tuple(points))
            cursor = (float(tokens[i]), float(tokens[i + 1]))
            points = [cursor]
            i += 2
        elif command == "C":
            control1 = (float(tokens[i]), float(tokens[i + 1]))
            control2 = (float(tokens[i + 2]), float(tokens[i + 3]))
            end = (float(tokens[i + 4]), float(tokens[i + 5]))
            i += 6
            start = cursor
            for step in range(1, steps + 1):
                t = step / steps
                u = 1 - t
                points.append(
                    (
                        u**3 * start[0] + 3 * u * u * t * control1[0] + 3 * u * t * t * control2[0] + t**3 * end[0],
                        u**3 * start[1] + 3 * u * u * t * control1[1] + 3 * u * t * t * control2[1] + t**3 * end[1],
                    )
                )
            cursor = end
        elif command == "Z":
            if points:
                polygons.append(tuple(points))
                points = []
        else:
            raise ValueError(f"unsupported ribbon path token {command}")
    if points:
        polygons.append(tuple(points))
    return tuple(polygons)


def flatten_outline_path(path: str, steps: int = 16) -> tuple[tuple[tuple[float, float], ...], ...]:
    tokens = re.findall(r"[MLHVQCZ]|-?\d+(?:\.\d+)?", path)
    polygons: list[tuple[tuple[float, float], ...]] = []
    points: list[tuple[float, float]] = []
    cursor = (0.0, 0.0)
    command = ""
    i = 0
    while i < len(tokens):
        if tokens[i] in {"M", "L", "H", "V", "Q", "C", "Z"}:
            command = tokens[i]
            i += 1
            if command == "Z":
                if points:
                    polygons.append(tuple(points))
                    points = []
                continue
        if command == "M":
            if points:
                polygons.append(tuple(points))
            cursor = (float(tokens[i]), float(tokens[i + 1]))
            points = [cursor]
            i += 2
            command = "L"
        elif command == "L":
            cursor = (float(tokens[i]), float(tokens[i + 1]))
            points.append(cursor)
            i += 2
        elif command == "H":
            cursor = (float(tokens[i]), cursor[1])
            points.append(cursor)
            i += 1
        elif command == "V":
            cursor = (cursor[0], float(tokens[i]))
            points.append(cursor)
            i += 1
        elif command in {"Q", "C"}:
            start = cursor
            if command == "Q":
                control = (float(tokens[i]), float(tokens[i + 1]))
                end = (float(tokens[i + 2]), float(tokens[i + 3]))
                i += 4
                for step in range(1, steps + 1):
                    t = step / steps
                    u = 1 - t
                    points.append((u * u * start[0] + 2 * u * t * control[0] + t * t * end[0], u * u * start[1] + 2 * u * t * control[1] + t * t * end[1]))
            else:
                control1 = (float(tokens[i]), float(tokens[i + 1]))
                control2 = (float(tokens[i + 2]), float(tokens[i + 3]))
                end = (float(tokens[i + 4]), float(tokens[i + 5]))
                i += 6
                for step in range(1, steps + 1):
                    t = step / steps
                    u = 1 - t
                    points.append((u**3 * start[0] + 3 * u * u * t * control1[0] + 3 * u * t * t * control2[0] + t**3 * end[0], u**3 * start[1] + 3 * u * u * t * control1[1] + 3 * u * t * t * control2[1] + t**3 * end[1]))
            cursor = end
        else:
            raise ValueError(f"unsupported wordmark path token {tokens[i]}")
    if points:
        polygons.append(tuple(points))
    return tuple(polygons)


def raster_symbol(size: int, ink: str, accent: str, bg: str, *, small: bool = False) -> bytes:
    del ink, accent
    scale = size / 32.0
    background = hex_rgb(bg)
    a0, a1 = hex_rgb("#8b5cf6"), hex_rgb("#d946ef")
    b0, b1 = hex_rgb("#58104f"), hex_rgb("#ec48c6")
    c0, c1 = hex_rgb("#f5c96a"), hex_rgb("#fff8e8")
    supersample = 4
    canvas_size = size * supersample
    canvas = bytearray(bytes(background) * (canvas_size * canvas_size))
    shapes = [(flatten_cubic_path(RIBBON_A, 96), a0, a1), (flatten_cubic_path(RIBBON_B, 96), b0, b1)]
    if not small or size >= 32:
        shapes.append((flatten_cubic_path(RIBBON_C, 96), c0, c1))

    def canvas_point(point: tuple[float, float]) -> tuple[float, float]:
        x, y = point
        if small:
            x, y = 1.25 * x - 4, 1.25 * y - 4
        return x * scale * supersample, y * scale * supersample

    # Scan-convert at 4x resolution and box-filter down. This keeps the raster
    # silhouette faithful to the canonical cubic paths without platform font or
    # graphics-library dependencies.
    for polygons, start, end in shapes:
        for polygon in polygons:
            transformed = tuple(canvas_point(point) for point in polygon)
            min_y = max(0, math.floor(min(point[1] for point in transformed)))
            max_y = min(canvas_size, math.ceil(max(point[1] for point in transformed)))
            for y in range(min_y, max_y):
                sample_y = y + 0.5
                intersections: list[float] = []
                previous = transformed[-1]
                for current in transformed:
                    x1, y1 = previous
                    x2, y2 = current
                    if (y1 > sample_y) != (y2 > sample_y):
                        intersections.append(x1 + (sample_y - y1) * (x2 - x1) / (y2 - y1))
                    previous = current
                intersections.sort()
                for left, right in zip(intersections[0::2], intersections[1::2]):
                    first = max(0, math.ceil(left - 0.5))
                    last = min(canvas_size, math.ceil(right - 0.5))
                    for x in range(first, last):
                        px = (x + 0.5) / (scale * supersample)
                        py = (y + 0.5) / (scale * supersample)
                        if small:
                            px, py = (px + 4) / 1.25, (py + 4) / 1.25
                        t = max(0.0, min(1.0, (px + py - 8) / 44))
                        colour = mix(start, end, t)
                        i = (y * canvas_size + x) * 3
                        canvas[i : i + 3] = bytes(colour)

    pixels = bytearray(size * size * 4)
    for y in range(size):
        for x in range(size):
            channels = [0, 0, 0]
            for sy in range(y * supersample, (y + 1) * supersample):
                for sx in range(x * supersample, (x + 1) * supersample):
                    i = (sy * canvas_size + sx) * 3
                    for channel in range(3):
                        channels[channel] += canvas[i + channel]
            colour = tuple(round(value / supersample**2) for value in channels)
            i = (y * size + x) * 4
            pixels[i : i + 4] = bytes((*colour, 255))
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
    wordmark_colour = hex_rgb(dark["ink"])
    pixels = bytearray(width * height * 4)
    mark_size = 360
    mark = raster_symbol(mark_size, dark["ink"], dark["accent"], dark["bg"])
    for i in range(0, len(pixels), 4):
        pixels[i : i + 3] = bytes(bg)
        pixels[i + 3] = 255
    ox, oy = 84, (height - mark_size) // 2
    for y in range(mark_size):
        for x in range(mark_size):
            s = (y * mark_size + x) * 4
            d = ((oy + y) * width + (ox + x)) * 4
            pixels[d : d + 4] = mark[s : s + 4]

    # Rasterise the canonical outlined wordmark rather than depending on an
    # installed font. Even-odd filling preserves counters such as Q, A and O.
    glyphs = [
        flatten_outline_path(path)
        for path in re.findall(r'd="([^"]+)"', wordmark_paths())
    ]
    word_scale = 5.55
    word_x = 520.0
    word_y = 226.0
    x0, x1 = int(word_x), min(width, int(word_x + 106 * word_scale + 1))
    y0, y1 = int(word_y + 8 * word_scale), min(height, int(word_y + 26 * word_scale + 1))
    samples = ((0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75))
    for y in range(y0, y1):
        for x in range(x0, x1):
            covered = 0
            for dx, dy in samples:
                sx = (x + dx - word_x) / word_scale
                sy = (y + dy - word_y) / word_scale
                if any(
                    sum(inside_polygon(sx, sy, contour) for contour in glyph) % 2
                    for glyph in glyphs
                ):
                    covered += 1
            if covered:
                i = (y * width + x) * 4
                pixels[i : i + 3] = bytes(mix(bg, wordmark_colour, covered / len(samples)))
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


def simulation_strip(colours: list[tuple[int, int, int]]) -> bytes:
    width, height = 240, 48
    pixels = bytearray(width * height * 4)
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
    errors: list[str] = []
    if "<?xml" not in text:
        errors.append("SVG missing XML declaration")
    try:
        root = ElementTree.fromstring(text)
    except ElementTree.ParseError as exc:
        return errors + [f"invalid XML: {exc}"]
    svg_namespace = "{http://www.w3.org/2000/svg}"
    if root.tag != f"{svg_namespace}svg":
        errors.append("root element is not SVG")
    forbidden_elements = {
        "script",
        "style",
        "foreignObject",
        "iframe",
        "audio",
        "video",
        "animate",
        "animateMotion",
        "animateTransform",
        "set",
    }
    for element in root.iter():
        local_tag = element.tag.rsplit("}", 1)[-1]
        if local_tag in forbidden_elements:
            errors.append(f"unsafe SVG element {local_tag}")
        for raw_name, raw_value in element.attrib.items():
            name = raw_name.rsplit("}", 1)[-1].lower()
            value = raw_value.strip().lower()
            if name.startswith("on"):
                errors.append(f"unsafe SVG event attribute {name}")
            if name in {"href", "src"}:
                errors.append(f"SVG resource reference {name} is forbidden")
            if ("url(" in value and not re.fullmatch(r"url\(#[a-z0-9_-]+\)", value)) or "javascript:" in value or "data:" in value:
                errors.append(f"unsafe SVG attribute value in {name}")
    return errors


def png_dimensions(data: bytes) -> tuple[int, int] | None:
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        return None
    return struct.unpack(">II", data[16:24])


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
        antipodal_direction_svg(dark["ink"], dark["accent"], bg=dark["bg"]),
    )
    put("source/directions/quotient-lens.svg", direction_quotient_lens())
    put("source/directions/woven-lift.svg", symbol_svg(dark["ink"], dark["accent"], bg=dark["bg"]))
    put("source/icons/result-pass.svg", result_icon_svg("pass", dark["pass"]))
    put("source/icons/result-findings.svg", result_icon_svg("findings", dark["findings"]))
    put("source/icons/result-refused.svg", result_icon_svg("refused", dark["refused"]))
    put("source/icons/result-error.svg", result_icon_svg("error", dark["error"]))
    put("templates/diagram-workflow.svg", workflow_diagram_svg())
    put("templates/diagram-workflow-narrow.svg", workflow_diagram_narrow_svg())
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
            "The canonical wordmark contains Space Grotesk SemiBold outlines generated from "
            "upstream font SHA-256 acad6de1fc93436f5c0f1f4137751ef04f1aea3063e7036535970ffcfbd79f72. "
            "No font binary is redistributed. See SPACE_GROTESK.md for provenance and OFL terms.\n"
            "UI typography uses system stacks; IBM Plex Mono remains the recommended open monospace.\n"
        ),
    )
    put(
        "LICENSES/SPACE_GROTESK.md",
        (
            "# Space Grotesk provenance\n\n"
            "The canonical Quatopsy wordmark was outlined from Space Grotesk SemiBold 600. "
            "The font binary is not redistributed.\n\n"
            "Upstream: https://github.com/floriankarsten/space-grotesk\n\n"
            "Source font SHA-256: `acad6de1fc93436f5c0f1f4137751ef04f1aea3063e7036535970ffcfbd79f72`\n\n"
            "Upstream licence: SIL Open Font License 1.1. The OFL permits documents created "
            "using the font to use another licence; the generated outline paths are distributed "
            "with the original Quatopsy artwork under Apache-2.0.\n"
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
    put(
        "exports/simulations/result-cvd.png",
        encode_png(
            240,
            48,
            simulation_strip(
                [hex_rgb(dark["pass"]), hex_rgb(dark["findings"]), hex_rgb(dark["refused"]), hex_rgb(dark["error"])]
            ),
        ),
    )
    put(
        "exports/simulations/brand-cvd.png",
        encode_png(
            240,
            48,
            simulation_strip([hex_rgb("#7c4dff"), hex_rgb("#d946ef"), hex_rgb("#c026d3"), hex_rgb("#f5c96a")]),
        ),
    )
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
        "direction": "woven-lift",
        "entries": entries,
    }
    return (json.dumps(doc, indent=2) + "\n").encode("utf-8")


def export(dest: Path = BRAND) -> None:
    files = tree()
    files["BRAND_ASSET_MANIFEST.json"] = manifest_for(files)
    for rel, data in files.items():
        path = dest / rel
        if path.is_symlink() or any(parent.is_symlink() for parent in path.parents if parent != dest.parent):
            raise RuntimeError(f"refusing brand export through symlink: {path}")
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
    actual: dict[str, bytes] = {}
    for path in BRAND.rglob("*"):
        if path.is_symlink():
            errors.append(f"brand tree contains symlink: {path.relative_to(BRAND)}")
        elif path.is_file():
            actual[str(path.relative_to(BRAND))] = path.read_bytes()
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
    manifest = json.loads(expected["BRAND_ASSET_MANIFEST.json"])
    for entry in manifest["entries"]:
        if "dimensions" in entry:
            dimensions = png_dimensions(expected[entry["path"]])
            if dimensions is None or list(dimensions) != entry["dimensions"]:
                errors.append(f'{entry["path"]}: manifest dimensions do not match PNG')
    svg_attack_cases = (
        '<svg xmlns="http://www.w3.org/2000/svg"><script/></svg>',
        '<svg xmlns="http://www.w3.org/2000/svg"><path onmouseover="x()"/></svg>',
        '<svg xmlns="http://www.w3.org/2000/svg"><image href="payload.png"/></svg>',
        '<svg xmlns="http://www.w3.org/2000/svg"><path style="fill:url(https://example.invalid/x)"/></svg>',
        '<svg xmlns="http://www.w3.org/2000/svg"><style>@import url(https://example.invalid/x)</style></svg>',
    )
    for attack in svg_attack_cases:
        if not svg_is_safe('<?xml version="1.0"?>' + attack):
            errors.append("SVG validator accepted a built-in attack case")
    dark = TOKENS["palette"]["dark"]
    light = TOKENS["palette"]["light"]
    for name, fg, bg in (
        ("dark ink", dark["ink"], dark["bg"]),
        ("dark muted", dark["muted"], dark["bg"]),
        ("dark accent", dark["accent"], dark["bg"]),
        ("dark secondary accent", dark["accent_secondary"], dark["bg"]),
        ("dark inspection", dark["inspection"], dark["bg"]),
        ("dark pass", dark["pass"], dark["bg"]),
        ("dark findings", dark["findings"], dark["bg"]),
        ("dark refused", dark["refused"], dark["bg"]),
        ("light ink", light["ink"], light["bg"]),
        ("light accent", light["accent"], light["bg"]),
        ("light secondary accent", light["accent_secondary"], light["bg"]),
        ("light inspection", light["inspection"], light["bg"]),
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
    viewer_root = ROOT / "crates" / "quatopsy-cli" / "viewer"
    viewer_css = (viewer_root / "viewer.css").read_text(encoding="utf-8")
    for token in (dark["bg"], dark["ink"], dark["accent"]):
        if token not in viewer_css:
            errors.append(f"viewer.css missing token {token}")
    viewer_html = (viewer_root / "index.html").read_text(encoding="utf-8")
    if 'aria-label="Quatopsy"' not in viewer_html:
        errors.append("viewer mark missing accessible name")
    if f'data-brand-version="{BRAND_VERSION}"' not in viewer_html:
        errors.append("viewer mark missing canonical brand version")
    if RIBBON_A not in viewer_html or RIBBON_B not in viewer_html or RIBBON_C not in viewer_html:
        errors.append("viewer mark geometry drifted from canonical ribbon paths")
    if "<script" in viewer_html.lower() and "viewer.js" not in viewer_html:
        errors.append("unexpected viewer script")
    if TAGLINE not in (ROOT / "crates" / "quatopsy-cli" / "src" / "main.rs").read_text(
        encoding="utf-8"
    ):
        errors.append("CLI about text missing tagline")
    wordmark = expected["source/quatopsy-wordmark.svg"]
    if b"<text" in wordmark or b"font-family" in wordmark:
        errors.append("canonical wordmark must contain outlines, not live font text")
    if manifest.get("brand_version") != BRAND_VERSION or manifest.get("direction") != "woven-lift":
        errors.append("brand manifest identity metadata is stale")
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
