# ADR 0006: Woven-lift brand revision

## Status

Accepted on 2026-08-25.

## Context

The owner found the `quatopsy.brand/1` antipodal paired-point mark mathematically relevant but visually boring and diagrammatic. Iterations based on a ruptured orbit became either too busy, too flat, too atomic, or too literal. Further owner-supplied references established the desired visual grammar: few elements, broad tapered ribbon planes, controlled asymmetry, layered crossings, elegant colour transitions, and recognisable character.

## Decision

Supersede the canonical identity with `quatopsy.brand/2` and select the owner-approved woven-lift composition. It contains two related violet and cerise ribbon paths plus a warm-ivory inspection path. The form suggests representation duality and forensic inspection without claiming to be a mathematical diagram.

Use Space Grotesk SemiBold 600 for the wordmark. Canonical SVG lockups contain fixed outlines derived from upstream font SHA-256 `acad6de1fc93436f5c0f1f4137751ef04f1aea3063e7036535970ffcfbd79f72`; no font binary or runtime font request is required. UI typography remains on local system stacks.

Keep the result-state palette independent from the identity ribbons. Retain monochrome, reversed, high-contrast, forced-colour, and dedicated 16 px forms. Apply the new mark to the viewer as inline SVG so its fail-closed content security policy remains unchanged.

## Consequences

`quatopsy.brand/1` remains available through repository history and PR 15, but `/2` is canonical. The richer identity is still not evidence of safety, production maturity, public opening, or independent validation. Gradient artwork must degrade to the monochrome form where gradients, colour, or adequate contrast cannot be guaranteed.
