# ADR-0017: Model download-on-demand — the gated fetch, off by default

- **Status:** Accepted by administrative renumber — 2026-08-04. This record
  preserves the operator-approved 2026-07-10 decision without changing its scope.
  Shipping the feature enabled in the default first-run build still requires the
  separate operator go defined below.
- **Date:** 2026-07-10 (authority ID repaired 2026-08-04)
- **Supersedes:** the duplicate-number record preserved byte-for-byte at
  `archive/0015-model-download-surface.md` (SHA-256
  `fedf8e8f4f26fddde8202ad94889b3852ff7d65bb78b142d988952ca87b92cec`).
- **PRD items affected:** P1-P0-2 (a fresh user acquires a model), P1-P0-8
  (60-second first run: install -> model download -> first dictation). Completes
  the sub-decision ADR-0014 explicitly deferred.

## Administrative repair

The original model-download record and the accepted quantized-ASR-default decision
were both assigned ADR-0015. That made critical-path citations ambiguous while the
ADR status gate still passed. ADR-0015 remains the immutable authority for the
quantized P1 ASR default. ADR-0017 is now the sole active authority for the existing
model-download surface. This renumber adds no dependency, network call, model,
feature enablement, or product behavior.

## Context

The registry produces a validated `ModelDownloadPlan` (placeholder checksums
rejected, HTTPS-only sources), and `install_artifact_from_path()` verifies SHA-256
before installation. First Run exposes source, size, license, and blocked-reason
preflight data. The missing link was the HTTP GET that turns a reviewed plan into a
verified local artifact. That is a network surface governed by non-negotiable #1.

## Decision

Keep the checksum-gated fetch behind the off-by-default Cargo feature
`model-download`, using the minimal blocking `ureq` client with Rustls:

1. **Zero egress by default.** Default builds and CI do not compile the client.
   The one reviewed call site stays in `network-allowlist.json` under ADR-0017.
2. **Pinned sources only.** Fetch only registry-pinned HTTPS sources; no
   user-supplied URL or arbitrary redirect is authorized.
3. **Verify before use, fail closed.** Download to a temporary path and install
   only through the existing SHA-256 verification seam. Bad bytes never load.
4. **Explicit user action only.** Never fetch at startup or in the background;
   no telemetry or phone-home behavior is authorized.
5. **License gate respected.** Models requiring license review remain blocked
   until that review clears.

## Alternatives retained from the accepted decision

- A heavier async HTTP stack was rejected for footprint and complexity.
- Bundling weights was rejected because models do not belong in git.
- Implicit or default-on download was rejected by the privacy constitution.

## Consequences

- An explicitly opted-in build can fetch and verify the reviewed first-run model.
- Enabling `model-download` in the build users receive remains a separate critical
  operator gate. This administrative repair does not provide that go.
- Any second network call site still fails the source audit.
- Historical evidence may say ADR-0015 because it predates this identity repair;
  live implementation and policy references must say ADR-0017.
