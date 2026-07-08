# ADR-0011: Linux is first-class — the 3-OS matrix (macOS + Windows + Linux)

- **Status:** Accepted
- **Date:** 2026-07-07
- **PRD items affected:** promotes P2-5 (Linux build) out of "expansion" into the
  core platform mandate; amends non-negotiable #5; adds P0 CI matrix leg

## Context
The v2 charter made non-negotiable #5 "Windows is first-class" (a 2-OS mandate:
macOS + Windows), and listed Linux as post-traction expansion (PRD P2-5). The v3
market re-verification found the Linux desktop **structurally underserved**: only
Handy (young, raw-only) and Talon (X11-only, **no Wayland**) exist there. That is
a verified-empty lane, and for the mixed-fleet users Relay targets (Flagship 01),
"clean, seamless, consistent across all three" is itself a headline no polished
competitor offers. The v3 plan therefore makes Linux first-class. Because this
reinterprets a non-negotiable, it needs an ADR (operator ratified 2026-07-07).

## Decision
**Non-negotiable #5 becomes: "Windows and Linux are first-class."** Nothing ships,
merges, or is "done" if it works on macOS only, or on macOS + Windows only. The CI
matrix runs **macOS + Windows + Linux (Ubuntu)** on every PR. Detail:

- **Linux injection targets both X11 and Wayland.** X11 via XTest; Wayland via the
  virtual-keyboard / input-method protocols and the desktop portal APIs (the half
  of the Linux desktop Talon skips). Injection stays behind the `inject/` trait
  with a per-display-server implementation, same contract as mac/win.
- **Packaging:** AppImage + `.deb` (alongside notarized `.dmg` and signed `.msi`),
  from the same Tauri codebase.
- **Reference posture:** the two *tuning* reference machines for latency/prediction
  defaults remain the M-series Mac and the mid-range Windows laptop (ADR-0007);
  Linux must pass functional + CI gates but is not a third bench-tuning target for
  prediction defaults in v3.
- **Wayland is the highest platform risk** and gets an early spike in P1 (mirrors
  the Windows-UIA spike), not a late port.

## Alternatives considered
- **Keep Linux as post-traction expansion (v2 P2-5).** Cheaper near-term, but
  concedes the one verified-empty desktop lane and undercuts Relay's mixed-fleet
  pitch. Rejected per v3.
- **Linux X11 only (the Talon compromise).** Leaves modern Wayland desktops
  unserved — the exact gap the plan calls out. Rejected; Wayland is in scope.
- **Linux "best effort," excluded from the merge gate.** A second-class platform
  by another name; violates the spirit of #5. Rejected — Linux is in the CI gate.

## Consequences
- Root `AGENTS.md` non-negotiable #5, §1, and repo-map CI note updated to 3-OS;
  `.github/workflows/ci.yml` matrix gains `ubuntu-latest`.
- `inject/` (and app-detection in `profiles/`) gains X11 + Wayland implementations;
  `ARCHITECTURE.md` §5 documents them.
- Every phase's Definition of Done and latency/footprint budgets now span three
  OSes; "3-OS CI green" is a standing gate.
- **Environment note (2026-07-07):** the current build host is macOS-only with no
  git remote, so the Windows and Linux CI legs cannot execute yet. Per operator
  ("local-only for now"), Windows/Linux gates are **waived-pending-infra** and
  tracked as mission blockers until a remote with Actions (or the reference
  machines) is available. The mandate stands; only its *verification* is deferred.
