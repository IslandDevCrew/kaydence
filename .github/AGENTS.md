# .github/ — CI/CD

## Pipeline (workflows/ci.yml)
Matrix: `[macos-latest, windows-latest, ubuntu-latest]`. Jobs: rustfmt →
clippy -D warnings → cargo test → frontend typecheck/lint/test → privacy gates
→ ADR status → bench gate → (release tags) build, sign, notarize, checksum,
draft release.

## Rules
1. The OS matrix is permanent. A PR that skips Windows checks "temporarily"
   violates non-negotiable #5 — there is no temporarily.
2. `audit-network.sh` and `check-privacy-posture.sh` run on every PR; the
   network allowlist diff must be reviewed by the human operator (not
   auto-merged), and screen-capture primitives remain permanently banned.
3. Caches: cargo + model artifacts for the bench job (weights from the
   registry mirrors, checksum-verified — never committed).
4. Secrets: signing certs only. There are no telemetry/API keys because there
   is no telemetry/API.
5. Release artifacts: .dmg (notarized) + .msi (signed) + SHA256SUMS published
   alongside.
