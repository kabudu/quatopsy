# ADR 0005: Product brand system

## Status

Accepted on 2026-08-19.

## Context

M5 shipped a restrained research identity because productisation was not approved. On 2026-08-19 the owner approved productisation and declined a trademark filing as unnecessary. Public opening remains a later gate. A full visual system was still required before that gate.

Three mark directions were required: a lifted-path diagnostic trace, an antipodal paired-point system, and a quotient-space inspection lens.

## Decision

Keep the product name `Quatopsy`. Record that this is not trademark clearance.

Select the antipodal paired-point mark. It states the representation-versus-physical mechanism, stays legible at 16 px, and avoids medical or spacecraft cliches. Score notes:

- Lifted-path: readable as a generic chart sparkline; the defect tick is easy to miss at small size.
- Antipodal paired-point: two samples on one circle plus a chosen lift. Highest mechanism fit.
- Quotient lens: overlapping circles read as a Venn diagram or generic comparison mark.

Ship `quatopsy.brand/1` as source SVGs, deterministic PNG exports, tokens, templates, licences, and `BRAND_ASSET_MANIFEST.json`. Canonical lockups stay maturity-neutral. Private-research wording lives only in overlay templates.

Wire the symbol into the local viewer as inline SVG so `img-src 'none'` CSP remains. The CLI about line carries the tagline. Local CI recomputes the asset tree and refuses drift.

## Consequences

Public opening, hosted CI, crates.io, signed binaries, and production support stay closed. Residual name-collision risk remains until a later rename or legal review if the owner later wants public distribution. Polished marks must not be read as flight approval; claim scans and overlay separation remain in CI.
