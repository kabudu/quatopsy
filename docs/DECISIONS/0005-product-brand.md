# ADR 0005: Product brand system

## Status

Accepted on 2026-08-19.

## Context

M5 shipped a restrained research identity because productisation was not approved. On 2026-08-19 the owner approved productisation and declined a trademark filing as unnecessary. Public opening remains a later gate. A full visual system was still required before that gate.

Three mark directions were required: a lifted-path diagnostic trace, an antipodal paired-point system, and a quotient-space inspection lens.

## Decision

Keep the product name `Quatopsy`. Record that this is not trademark clearance.

Select the antipodal paired-point mark. It states the representation-versus-physical mechanism, stays legible at 16 px, and avoids medical or spacecraft cliches. The owner assessed exported candidates against a five-point scale, where 5 is strongest. The small-size score was judged from each 16 px raster export; the other scores were judged from the source direction at 32 and 512 px.

| Direction | Mechanism fit | 16 px legibility | Distinctiveness | Misreading resistance | One-colour fitness | Total / 25 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Lifted path | 4 | 2 | 3 | 2 | 3 | 14 |
| Antipodal paired point | 5 | 5 | 4 | 4 | 5 | 23 |
| Quotient lens | 4 | 3 | 3 | 2 | 4 | 16 |

The lifted-path defect tick is easy to miss at small size and can read as a generic sparkline. The quotient lens can read as a Venn diagram. The selected paired-point direction preserves its ring-and-opposite-points structure in the dedicated small-size export. These are design judgements, not user-research results.

Ship `quatopsy.brand/1` as source SVGs, deterministic PNG exports, tokens, templates, licences, and `BRAND_ASSET_MANIFEST.json`. Canonical lockups stay maturity-neutral. Private-research wording lives only in overlay templates.

Wire the symbol into the local viewer as inline SVG so `img-src 'none'` CSP remains. The CLI about line carries the tagline. Local CI recomputes the asset tree and refuses drift.

## Consequences

Public opening, hosted CI, crates.io, signed binaries, and production support stay closed. Residual name-collision risk remains until a later rename or legal review if the owner later wants public distribution. Polished marks must not be read as flight approval; claim scans and overlay separation remain in CI.
