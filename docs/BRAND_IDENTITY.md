# Brand identity

## Productisation state

Owner productisation was approved on 2026-08-19. `Quatopsy` is the approved product name, and the owner accepted it for public open-source use on 2026-08-30. Public opening remains a separate execution gate.

The canonical visual system is `quatopsy.brand/2`. Public opening, hosted CI, crates.io, signed binaries, and production support remain later distinct gates. `quatopsy.brand/1` is superseded but remains recoverable from repository history and its release commit.

## Name and positioning

Name: `Quatopsy`, combining quaternion and examination. Pronunciation: `kwot-op-see`. Category: orientation-trajectory diagnostics. Purpose: make hidden rotation-path defects inspectable and reproducible. Tagline: `See where rotations go wrong.`

The name was searched on 2026-08-14 across general web results, GitHub repository names, npm, PyPI, and crates.io without an exact material collision. The completed search and owner approval establish `Quatopsy` as the canonical public project identity.

## Audiences and language

Primary: spacecraft guidance, navigation, and control engineers. Secondary: robotics, simulation, and graphics engineers. Use `orientation trajectory`, `physical rotation`, `quaternion representation`, `finding`, `refusal`, and `repair candidate`. Avoid implying that quaternion components are four physical spatial dimensions.

## Brand platform

- Purpose: make orientation failures explainable before they become expensive motion.
- Promise: connect raw quaternion data to reproducible mathematical evidence.
- Principles: precise, inspectable, local-first, reversible, and claim-bounded.
- Personality: calm, technical, lucid, and candid.
- Anti-traits: mystical, aerospace-theatrical, alarmist, opaque, or falsely authoritative.

## Claim vocabulary

Permitted: `candidate`, `detects supported rule violations`, `proposes a repair`, `local-first`, `deterministic under the documented profile`, and `early-stage, production-quality research software for local advisory evaluation`. `Production-quality` describes engineering discipline within the declared scope and is distinct from production readiness. Evidence-dependent: `reduces debugging time`, `prevents unwinding`, `portable`, and `differentiated`. Prohibited: `novel`, `safe`, `flight-proven`, `certified`, `production-ready`, `complete`, `optimal`, or `guarantees correct motion`.

## Two-layer model

Permanent identity: name, category, purpose, promise, principles, personality, mark, tagline, tokens, and lockups. These do not encode alpha, beta, evaluation, or production status.

Maturity overlay: private research, support posture, independent-validation status, and exact limitations. Overlay copy lives only in separately named templates such as `templates/overlay-private-research.svg`.

## Selected visual direction

The initial three directions remain recorded in `assets/brand/source/directions/` and `docs/DECISIONS/0005-product-brand.md`. Owner review found the original antipodal paired-point mark too diagrammatic. ADR 0006 records the replacement selected after iterative visual exploration.

The selected woven-lift mark uses three tapered ribbon planes. Violet and cerise paths are rotational counterparts: related representations whose layered crossings imply the quaternion double cover. The ivory inspection ribbon passes through them as the forensic layer. The mark is expressive rather than a literal mathematical diagram; it must not be described as a proof or physical trajectory. Minimum size is 16 px using the dedicated two-ribbon small-size form. Clear space is 6 units on the 32-unit grid.

## Visual system

- Lockups: horizontal, stacked, symbol, wordmark, monochrome, reversed, small-size, and light.
- Primary palette: aubergine black, ultraviolet, orchid, indigo, cerise, warm ivory, and pale gold. Semantic result colours remain separate and must never be inferred from ribbon colour.
- Palettes: dark, light, monochrome, reversed, high-contrast, and forced-colour tokens in `assets/brand/tokens/`.
- Result states use colour plus a shape plus a text label: pass circle, findings diamond, refused triangle, error square.
- Wordmark typography: Space Grotesk SemiBold 600 with custom tracking. Canonical lockups contain deterministic vector outlines, not live text. The source font is not redistributed; its SHA-256 and SIL Open Font License provenance are recorded in `assets/brand/LICENSES/SPACE_GROTESK.md`.
- Interface typography: the existing system UI and monospace stacks remain dependency-free. Space Grotesk may be used for designed campaign headings when separately licensed and packaged. IBM Plex Mono remains the recommended open monospace.
- Diagrams and charts retain pass, findings, refused, and error as labelled states.
- The canonical repository architecture visuals are `templates/diagram-workflow.svg` and its narrow composition `templates/diagram-workflow-narrow.svg`. They use the product palette to distinguish recorded evidence, canonical semantics, candidate-only paths, verdict ownership, and local outputs without using colour as the sole label.
- Motion: non-essential animation disables under `prefers-reduced-motion`.
- Machine-readable tokens: `tokens.json` and `tokens.css`.

## Usage rules

- Use the full-colour mark on aubergine black or another near-black field. Use `quatopsy-lockup-light.svg` on light surfaces, and the mono or reversed variants when colour reproduction is uncertain.
- Use the dedicated `quatopsy-symbol-small.svg` below 32 px. It removes the ivory inspection ribbon and enlarges the two-path silhouette. Do not mechanically shrink the full mark below 32 px.
- Minimum symbol size is 16 px. Minimum horizontal lockup width is 156 px. Preserve clear space equal to 6 units on the mark's 32-unit construction grid.
- Do not rotate, reflect, add nodes, add an orbit, recolour individual ribbons outside approved variants, apply glow or shadow, or use the identity gradient to encode pass/fail state.
- Ultraviolet, orchid, indigo, and cerise are identity colours, not body-text colours. Use the documented ink, muted, focus, and semantic tokens for interface text and results.
- The plain tagline is `See where rotations go wrong.` Do not append maturity or safety language to a canonical lockup; use a separate overlay.

## Assets, provenance, and validation

Canonical assets live under `assets/brand/`. `python3 scripts/brandkit.py export` is the only writer. `python3 scripts/brandkit.py check` is wired into local CI. Each manifest entry records digests, dimensions, licence, creator, allowed use, and the export command. SVG sources forbid scripts, remote resources, and embedded rasters; local fragment gradients are permitted. Original marks are Apache-2.0. Result-state and brand-palette colour-vision simulation strips are generated under `exports/simulations/`; colour never carries result meaning alone.

## Governance

Roles: product, research claims, engineering, security, accessibility, and brand/design. The owner currently holds each role and recorded productisation approval, public-name approval, and the `/2` redesign approval separately from public opening. Brand sources version as `quatopsy.brand/2`, independent of the software protocol. Do not silently replace released manifests.
