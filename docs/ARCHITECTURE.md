# Architecture

## Minimal shape

Quatopsy starts as a Rust workspace with a pure analysis library (`quatopsy-core`), a CLI (`quatopsy`), a schema package (`quatopsy-schema`), and a static browser visualiser compiled to WebAssembly only where shared computations require it. An independent `quatopsy-oracle` crate exists only for conformance tests and is not linked into production verdicts. The browser consumes the same immutable JSON report produced by the CLI. No server, account, database, telemetry collector, plug-in runtime, or network dependency belongs in the first release.

## Components

1. `ingest`: bounded streaming parser for the V1 CSV and manifest profile.
2. `model`: validated samples, declared conventions, exact input identity, and result-state types.
3. `kernel`: normalisation checks, quotient-invariant distances, deterministic `S^3` lift, finite differences, and rule evaluation.
4. `repair`: opt-in candidate transforms that never overwrite input.
5. `report`: canonical versioned JSON and terminal rendering.
6. `cli`: public offline workflow, resource limits, exit codes, and atomic output.
7. `viewer`: local static UI for linked 3D, projected `S^3`, timeline, and evidence views.
8. `adapters`: later converters into the canonical input contract, outside the semantic core.

## Data flow

Bytes are snapshotted and hashed before parsing. Parsing either yields a fully declared canonical sequence or a refusal. The kernel evaluates obligations against immutable validated samples. Repair proposals derive from that same snapshot. Reports are written to a temporary sibling file, flushed, and atomically renamed without clobber unless explicitly requested.

## Core invariants

- A quaternion sample used for rotation analysis is finite and non-zero.
- Physical rotation comparisons are invariant under independent sign negation of either quaternion.
- Report intervals refer to stable source row and timestamp identities.
- A diagnostic failure, timeout, limit breach, unsupported convention, or partial parse cannot yield `pass`.
- Original inputs are read-only and repair outputs are separately named.
- CLI and viewer never reinterpret verdicts or recompute release-critical rules differently.
- Rule-set and report versions are explicit and cannot silently drift.

## Deterministic identity

`analysis_id = SHA-256(input_bytes || manifest_bytes || engine_version || rule_set_version || numeric_profile || enabled_rules || limits)`, encoded with length-delimited fields. Canonical JSON uses fixed field order, locale-independent decimal formatting, no non-finite numbers, and timestamps stored as integer nanoseconds after checked conversion.

## Numeric policy

The semantic core uses IEEE 754 binary64 with documented operation ordering (w, x, y, z). Transcendental operations use `libm`. Inputs are not silently clamped except for a narrowly bounded `acos` domain correction after a proven unit-domain calculation. Profile `quatopsy.numeric/1` treats `|‖q‖ − 1| > 1e-6` as off-unit, `0 < ‖q‖ < 1e-12` as near-zero refusal, and `|p · q| <= 1e-12` as a non-unique lift tie. Parallel rule execution may be introduced only if report order and results remain deterministic.

## Resource governance

The CLI defaults to 1 GiB input bytes, 10 million samples, 512 MiB working memory target, one analysis job, a bounded finding count per rule, and explicit wall-clock cancellation. Limits are configurable only within compiled safe maxima for the browser. Parsing is streaming; the validated quaternion series is contiguous; viewer geometry is downsampled with extrema and finding intervals retained.

## Trust boundaries

The mathematical kernel and canonical schema are the logical trusted computing base. Parsers, CLI orchestration, repair writer, and viewer are security-relevant but cannot redefine result semantics. External adapters, ROS/MCAP/SPICE readers, CI presentation, and future hosted integrations are outside the logical trust boundary and must emit canonical inputs with provenance.

## Compatibility

Report readers must ignore unknown optional fields but reject unsupported major schema versions. Rules are additive within a major version unless semantics change. Repairs name their exact algorithm. The canonical CSV profile is stable and adapters are versioned independently.

## Failure and recovery

Parsing and analysis are side-effect-free until output commit. Cancellation removes temporary files. Corrupt caches, if introduced later, are bypassable and never authoritative. A clean mode performs fresh analysis. Crashes retain inputs and may retain a redacted diagnostic log only with user consent.

## Minimality rationale

Rust provides explicit result types, bounded parsers, portable CLI distribution, and WebAssembly reuse. A service is unjustified because the first workflow is local, sensitive, deterministic, and batch-oriented. Three compiled adapters exist (`ids-jason1`, `ros-json`, `tubin-str`); a plug-in system remains out of scope until a fourth format needs a stable extension boundary.

