# ADR 0007: Static project website

## Status

Accepted for the public GitHub Pages site on 2026-08-30.

## Context

Quatopsy needs a product-oriented public entry point that explains the diagnostic problem, evidence flow, initial capabilities, installation path, and advisory boundary without creating a hosted analysis service or extending the product trust boundary.

The existing product is local-first Rust software. A static presentation site does not need an application framework, server runtime, client state, telemetry, or user data.

## Decision

The repository may ship a static GitHub Pages website under `website/`. It uses semantic HTML and generated CSS. Tailwind CSS is a pinned build-time compiler only. The deployed output must:

- contain no executable browser JavaScript, analytics, cookies, forms, remote fonts, or remote page resources;
- use repository-owned `quatopsy.brand/2` assets and evidence-bounded copy;
- keep original evidence, diagnostic verdicts, and advisory candidates distinct;
- state that physical hardware, hard real-time, certification, orbit determination, and flight approval remain unsupported;
- build through `scripts/build-site.sh` into ignored `target/site` output;
- pass deterministic structure, metadata, link, resource, payload, reduced-motion, and forced-colour checks;
- deploy only from reviewed `master` through the least-privilege GitHub Pages workflow.

JSON-LD structured metadata is permitted as inert `application/ld+json` data. Any hosted analysis, upload, account, form submission, telemetry, executable client application, or external content dependency requires a new decision and explicit owner approval.

## Consequences

The website remains outside verdict ownership and the product runtime. It adds one pinned npm build dependency graph but no browser runtime dependency. GitHub Pages availability does not affect local analysis, and a website outage cannot change a report or candidate.
