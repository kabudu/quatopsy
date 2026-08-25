# Brand identity

## Productisation state

Owner productisation was approved on 2026-08-19. The owner declined a trademark filing as unnecessary for current needs. `Quatopsy` remains the product name. That decision is not trademark clearance, not a worldwide availability guarantee, and not public opening.

The canonical visual system is `quatopsy.brand/1`. Public opening, hosted CI, crates.io, signed binaries, and production support remain later distinct gates.

## Name and positioning

Name: `Quatopsy`, combining quaternion and examination. Pronunciation: `kwot-op-see`. Category: orientation-trajectory diagnostics. Purpose: make hidden rotation-path defects inspectable and reproducible. Tagline: `See where rotations go wrong.`

The name was searched on 2026-08-14 across general web results, GitHub repository names, npm, PyPI, and crates.io without an exact material collision. Fallback candidates retained for re-audit before any later public opening are `Quatlint` and `Attiscope`. Residual collision risk is accepted by the owner for private use without a trademark filing (R10).

## Audiences and language

Primary: spacecraft guidance, navigation, and control engineers. Secondary: robotics, simulation, and graphics engineers. Use `orientation trajectory`, `physical rotation`, `quaternion representation`, `finding`, `refusal`, and `repair candidate`. Avoid implying that quaternion components are four physical spatial dimensions.

## Brand platform

- Purpose: make orientation failures explainable before they become expensive motion.
- Promise: connect raw quaternion data to reproducible mathematical evidence.
- Principles: precise, inspectable, local-first, reversible, and claim-bounded.
- Personality: calm, technical, lucid, and candid.
- Anti-traits: mystical, aerospace-theatrical, alarmist, opaque, or falsely authoritative.

## Claim vocabulary

Permitted: `candidate`, `detects supported rule violations`, `proposes a repair`, `local-first`, and `deterministic under the documented profile`. Evidence-dependent: `reduces debugging time`, `prevents unwinding`, `portable`, and `differentiated`. Prohibited: `novel`, `safe`, `flight-proven`, `certified`, `production-ready`, `complete`, `optimal`, or `guarantees correct motion`.

## Two-layer model

Permanent identity: name, category, purpose, promise, principles, personality, mark, tagline, tokens, and lockups. These do not encode alpha, beta, evaluation, or production status.

Maturity overlay: private research, support posture, independent-validation status, and exact limitations. Overlay copy lives only in separately named templates such as `templates/overlay-private-research.svg`.

## Selected visual direction

Three directions were constructed and scored in `assets/brand/source/directions/` and `docs/DECISIONS/0005-product-brand.md`. The selected mark is the antipodal paired-point system: a circle, two opposite samples, and the chosen lift arc. It names the double cover without medical gore or a spacecraft silhouette. Minimum size is 16 px. Clear space is 8 units on the 32-unit grid.

## Visual system

- Lockups: horizontal, stacked, symbol, wordmark, monochrome, reversed, small-size, and light.
- Palettes: dark, light, monochrome, reversed, high-contrast, and forced-colour tokens in `assets/brand/tokens/`.
- Result states use colour plus a shape plus a text label: pass circle, findings diamond, refused triangle, error square.
- Typography: system UI and monospace stacks. IBM Plex Sans and IBM Plex Mono are recommended and are not redistributed.
- Diagrams and charts retain pass, findings, refused, and error as labelled states.
- Motion: non-essential animation disables under `prefers-reduced-motion`.
- Machine-readable tokens: `tokens.json` and `tokens.css`.

## Assets, provenance, and validation

Canonical assets live under `assets/brand/`. `python3 scripts/brandkit.py export` is the only writer. `python3 scripts/brandkit.py check` is wired into local CI. Each manifest entry records digests, dimensions, licence, creator, allowed use, and the export command. SVG sources forbid scripts, remote resources, and embedded rasters. Original marks are Apache-2.0.

## Governance

Roles: product, research claims, engineering, security, accessibility, brand/design, and legal. The owner currently holds each role and recorded productisation approval plus the trademark-filing decline separately from public opening. Brand sources version as `quatopsy.brand/1`, independent of the software protocol. Do not silently replace released manifests.
