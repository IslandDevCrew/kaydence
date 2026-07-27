# ADR-0015: Model download-on-demand — the gated fetch, off by default

- **Status:** Accepted (surface built, off by default) — 2026-07-10. Shipping it enabled
  in the default first-run flow is a separate operator go (see Consequences).
- **Date:** 2026-07-10
- **PRD items affected:** P1-P0-2 (a fresh user acquires a model), P1-P0-8 (60-second
  first run: install → model download → first dictation). Completes the sub-decision
  ADR-0014 explicitly deferred.

## Context
Everything around model acquisition already exists and is dependency-free: the registry
produces a **validated `ModelDownloadPlan`** (checksum-placeholder-rejected, HTTPS-only
`sources`), `install_artifact_from_path()` **verifies sha256 before installing**, the
first-run `FirstRunModelDownloadPreflight` surfaces sources/size/license/blocked-reason,
and the whisper adapter transcribes the moment a verified artifact is present (ADR-0014,
proven live 2026-07-10). The **one** missing link is the actual fetch — the HTTP GET that
turns a plan into a local file. That is a **new network surface**, which the constitution
(non-negotiable #1: zero egress by default) treats as a critical decision path. ADR-0014
deferred it here on purpose.

## Decision
Add a checksum-gated model fetch behind an **off-by-default cargo feature `model-download`**,
using the minimal pure-Rust blocking client **`ureq`** (rustls TLS). It consumes the
existing `ModelDownloadPlan` and installs through the existing verify-before-install path:

1. **Zero egress by default.** The default build, CI, and every other module stay
   network-free; `model-download` opts the client + the one call site in. The
   `network-allowlist.json` entry `models/download` moves to `status: active` referencing
   this ADR so `audit-network.sh` stays honest (fail-closed on any un-allowlisted call).
2. **Pinned sources only.** Fetch only the HTTPS `sources` the registry pins (already
   validated: scheme-checked, no placeholders). No redirect to arbitrary hosts; no
   user-supplied URL in this ADR.
3. **Verify before use, fail closed.** Download to a temp path, then
   `install_artifact_from_path()` — which hashes the bytes and refuses to install on any
   sha256 mismatch. A corrupt or wrong file is discarded, never loaded. Try sources in
   registry order; a failure or mismatch falls through to the next, else a clear error.
4. **Explicit user action, no telemetry.** The fetch runs only on a deliberate
   user/operator action (first-run "download model" or an explicit CLI/selftest), never
   implicitly at startup, never in the background. No analytics, no phone-home; the only
   bytes on the wire are the GET to a pinned model mirror.
5. **License gate respected.** `license_review_required` models (e.g. Gemma) are not
   fetched by this surface until their review clears (ADR-0007/0010) — the plan already
   carries the flag; the fetch refuses them.

## Alternatives considered
- **reqwest/tokio client:** heavier (async runtime + hyper) for a blocking one-shot
  download. `ureq` is smaller and matches the synchronous install path. Rejected on weight.
- **Bundle a model / keep manual placement only:** violates "no weights in git" and
  leaves a fresh user unable to reach first dictation without shell steps — fails P1-P0-8.
- **On by default / implicit background fetch:** violates non-negotiable #1 and the
  "explicit action" principle. Rejected.

## Consequences
- **Easier:** with `model-download` on, a fresh user reaches a verified model (and thus
  real transcription) through one explicit action — the last gap in the P1-P0-8 chain.
- **Gated for ship:** enabling `model-download` in the **default** first-run build that
  users receive is a separate operator go — this ADR builds the surface and proves it,
  but leaves it dormant by default. Turning it on for release is an explicit decision,
  recorded when taken.
- **Maintained:** `audit-network.sh` now enforces the `models/download` allowlist entry;
  any second network call site still fails closed. Each mirror domain is pinned in the
  registry, reviewed here.
