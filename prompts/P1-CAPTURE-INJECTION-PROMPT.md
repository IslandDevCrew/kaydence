# P1 Build Prompt — Global Hotkey Capture + Universal Text Injection (Fable-Safe)

The definitive build prompt for **P1-P0-1 (global hotkey → capture)** and **P1-P0-3
(universal injection on macOS, Windows, Linux)** — the two highest-risk platform tasks
in the MVP. Written to the highest engineering + security bar; if any single run is too
large for one Fable session, split at the phase markers (each is independently
shippable) and run them in sequence.

This is authorized product engineering on the **owned** monorepo. It is not a bypass of
any OS security control — every capability here is a documented, user-granted OS input
API used with explicit consent, and the prompt's security core is about *respecting*
those controls (secure-field refusal, least privilege, no persistence), not evading them.

> **How to run:** paste the ROLE + CONTEXT + the task section you're building into a
> Fable session (or use `/goal` A from `VISUAL-BUILD-PROMPT-PACK.md` first for the
> standing rules). Everything outside a fenced ```prompt``` block is guidance to you,
> not text for the model.

---

## Scope guardrails (read before running)

- **Owned repo:** `/Users/IDC2.5/Kaydence/kaydence`, remote `IslandDevCrew/kaydence`,
  default branch `main`, 3-OS Actions CI live.
- **Governing docs (read, don't re-derive):** `AGENTS.md` (non-negotiables), `docs/PRD.md`
  (P0-1, P0-3), `docs/ARCHITECTURE.md` §4 SessionEvent + §5 platform injection, ADR-0003
  (cleanup dial), ADR-0004 (Handy study: adopt TranscriptionCoordinator, native-first
  injection), ADR-0006 (privacy = transmission+persistence), ADR-0011 (Linux first-class).
- **Design:** build UI to screen-family boards 01 (Cockpit) + 03 (Injection) +
  07 (Privacy) — `docs/design/README.md`. Option 1 mark in the reserved slot; OS accent
  is the only free-tier per-OS variation.
- **Non-negotiables that bind this work:** #1 zero-trust privacy (no telemetry/network),
  #2 never lose audio, #5 Windows+Linux first-class, #8 universal injection incl. secure-
  field refusal. Never weaken one to pass a gate — that is itself a blocking defect.
- **Critical decision paths that HALT for operator go + ADR:** any new dependency or
  network surface; all platform-native code; changes to the SessionEvent contract. Both
  tasks here are platform code — expect an ADR per platform decision, and stop before
  merging one silently.
- **Evidence-first:** every gate result saved under `ops/mission/evidence/`. A green
  Judge with a red Audit does not merge.

---

## The prompt

```prompt
ROLE
You are a senior cross-platform systems engineer implementing the Kaydence MVP input
layer in an owned Tauri 2 + Rust monorepo (React/TypeScript presentation; all logic in
Rust). You value correctness, memory safety, least privilege, and platform-native
behavior over cleverness. You produce small, reviewable, well-tested increments and you
stop at declared decision points instead of guessing.

CONTEXT (authorized, owned system)
- Repo: local /Users/IDC2.5/Kaydence/kaydence, remote IslandDevCrew/kaydence, branch main,
  3-OS GitHub Actions CI (macOS + Windows + Ubuntu) live.
- Read first and conform to: AGENTS.md (non-negotiables), docs/PRD.md (P0-1, P0-3),
  docs/ARCHITECTURE.md (§4 SessionEvent, §5 platform injection), docs/decisions/ADR-0003,
  ADR-0004 (Handy adopt/redesign notes), ADR-0006 (privacy line), ADR-0011 (Linux).
- The typed SessionEvent contract already exists (apps/desktop/src-tauri/src/events.rs)
  with module stubs (todo!()). Build behind that contract; do not change its shape
  without an ADR + operator go.
- Standing gates you must keep green: cargo fmt --check; cargo clippy -D warnings;
  cargo test; scripts/check-frontend.sh; scripts/audit-network.sh (any input/platform
  change is network-adjacent — run it); pnpm install --frozen-lockfile.

OBJECTIVE
Implement two MVP capabilities to production quality on all three OSes:
  1. P1-P0-1 — Global hotkey → audio capture pipeline.
  2. P1-P0-3 — Universal injection of finalized text into the focused app.

═══════════════════════════════════════════════════════════════════════════
PHASE 1 — P1-P0-1: Global hotkey + capture
═══════════════════════════════════════════════════════════════════════════
Requirements (PRD P0-1, Pitfall P1):
- Global hotkey with BOTH push-to-talk (hold) and toggle modes; user-rebindable;
  sensible per-OS defaults. Register via the vetted Tauri global-shortcut plugin
  (adding it is a dependency decision → audit-network allowlist entry + short ADR note).
- Adopt Handy's TranscriptionCoordinator pattern (ADR-0004): a SINGLE owner thread with a
  ~30ms debounce so rapid hotkey presses cannot start overlapping capture sessions
  (Pitfall P1 — the hotkey race). Model capture state as an explicit state machine
  (Idle → Arming → Capturing → Finalizing → Idle); illegal transitions are unrepresentable.
- Min-capture 250ms (ignore accidental taps) and a 300ms tail buffer after release so
  trailing audio is never clipped.
- Capture audio via cpal; emit the typed SessionEvent stages (§4). Audio must reach the
  write-ahead log (P1-P0-4, already merged) BEFORE any downstream processing — never hold
  the only copy in memory (non-negotiable #2).
- HUD reflects capture state per screen-family board 01 (Cockpit): recording indicator,
  waveform, elapsed time, engine, target app, local-only status.

Deliverables:
- Rust: hotkey module + capture coordinator (state machine, single-thread ownership,
  debounce) wired to cpal and the WAL, behind the SessionEvent contract.
- Frontend: Cockpit hotkey/recording UI to board 01 (Option 1 mark; OS accent only).
- Tests: state-machine transition tests incl. the rapid-press race (must not double-start);
  min-capture + tail-buffer boundary tests; a "capture writes to WAL before ASR" test.

═══════════════════════════════════════════════════════════════════════════
PHASE 2 — P1-P0-3: Universal injection  (CRITICAL PATH — highest platform risk)
═══════════════════════════════════════════════════════════════════════════
Sequence deliberately — do the RISKIEST backend FIRST so a dead end surfaces early:

  2a. LINUX Wayland spike FIRST (highest risk; ROADMAP week-2 mandate). Prove text
      injection via the input-method / virtual-keyboard portal path on a mainstream
      Wayland compositor; document exactly what is and isn't possible and which portals
      are required. If Wayland injection is not achievable for a target surface, that is
      a finding to report with the fallback plan — not something to force.
  2b. Linux X11 path (XTEST / XSendEvent as appropriate).
  2c. macOS: Accessibility (AX) API insertion; requires the Accessibility permission —
      detect, request with a plain-language rationale, and degrade gracefully if denied.
  2d. Windows: UI Automation TextPattern where supported, else SendInput; handle the
      known per-app UIA quirks (Pitfall P8).

Cross-cutting injection requirements:
- Define ONE trait (e.g. `TextInjector`) with per-OS implementations behind cfg gates;
  callers are platform-agnostic. Keep each platform impl in its own module per the
  architecture doc; no platform types leak across the trait boundary.
- Clipboard fallback ONLY when native insertion is unavailable: snapshot the user's
  existing clipboard, set text, paste, and RESTORE the original within <=200ms. Never
  leave the user's clipboard changed.
- SECURE-FIELD REFUSAL (non-negotiable #8 + #1): before injecting, detect password /
  secure input fields (macOS AXSecureTextField / secure event input; Windows
  IsPassword / protected UIA; Linux best-effort equivalents). If the focused field is
  secure, REFUSE to inject and surface a clear reason. This is a hard invariant with its
  own test.
- Injection is best-effort and observable: on failure, emit a typed error event and a
  user-visible message; never silently drop finalized text (that would risk the user's
  words). Finalized text also remains in history (WAL/SQLite) regardless of injection
  outcome.
- No network, no telemetry, no persistence of injected content beyond the existing local
  history. audit-network.sh must stay green.

Deliverables:
- Rust: TextInjector trait + 4 platform impls (Wayland, X11, macOS AX, Windows UIA/SendInput)
  + clipboard-fallback helper (snapshot/restore) + secure-field detector per OS.
- Frontend: Injection settings + status to board 03 (paths shown per OS; permission panels
  native; accent-only variation).
- Tests: secure-field refusal test PER OS (the load-bearing safety test); clipboard
  snapshot-restore round-trip (original restored, timing budget); trait-level injection
  contract tests with a mock target; a "finalized text survives an injection failure" test.

SECURITY & QUALITY BAR (applies to both phases)
- Least privilege: request each OS permission only when first needed, with a plain
  rationale; function (degraded) if denied; never assume a permission is granted.
- Validate at boundaries: treat focused-window/app metadata as untrusted input.
- No secrets, no network calls, no telemetry, no analytics — anywhere in this layer.
- Errors handled explicitly and surfaced to the user in UI-facing paths; nothing
  swallowed. Detailed context logged locally only.
- Immutable-by-default data flow; the capture state machine is the one owner of mutable
  capture state.
- Files stay focused (<800 lines) and functions small (<50); one module per platform.

WORKING PROCEDURE
- One task = one branch (mission/p1-p0-1-hotkey, mission/p1-p0-3-injection) = one PR,
  <=400 changed lines; split further if larger.
- Follow prompts/BUILD-LOOP.md (ORIENT→PLAN→BUILD→JUDGE→AUDIT→GATE) inside each task.
- For EACH new dependency and EACH platform-native decision: add the audit-network
  allowlist entry and write a short ADR (docs/decisions/), then HALT for operator go
  before merging — these are declared critical decision paths.
- Run the full gate set locally (macOS leg) and push so the 3-OS CI covers Win+Linux;
  save gate output under ops/mission/evidence/. Design-review the built screens against
  boards 01/03/07 for the P1-G4 fidelity gate.
- End each task: update ops/mission/state.json, run node ops/mission/render-sotu.mjs,
  append ops/mission/journal.md, commit state+journal+HTML.

STOP CONDITIONS (report loudly, do not work around)
- Wayland injection proves infeasible for a required surface → report with the fallback.
- Any decision would weaken a non-negotiable (e.g. injecting into a secure field to make
  a test pass) → stop; that is a blocking defect, not a trade-off.
- A gate can only be verified on hardware this host lacks (windowed tauri dev on Win/Linux)
  → mark waived-pending-infra with the exact machine needed.
- A critical decision path is reached (new dep/network surface, platform code, event
  contract change) → HALT for operator go + ADR.

DELIVER
For each task: the built code, files changed, tests written + their results, gate
evidence paths, the ADR(s) drafted, the CI run URL, and an honest list of anything
that could not close on this host and exactly what's needed to close it.
```

---

## Why this prompt is shaped this way (notes for the operator)

- **Risk-first ordering.** Phase 2 does the Wayland spike before the "easy" desktops, so
  the single highest platform risk in the whole MVP fails fast if it's going to fail —
  matching the ROADMAP week-2 mandate rather than discovering it last.
- **Security is the point, not a footnote.** Secure-field refusal, clipboard
  snapshot-restore, least-privilege permission prompts, and "never silently drop
  finalized text" are written as hard invariants each with their own test — because this
  layer touches the user's keystrokes and focused apps, which is exactly where a voice
  tool must be most trustworthy.
- **It respects the mission's own rules.** Every new dependency and platform decision is
  routed through the audit-network gate + an ADR + an operator HALT, so the prompt can't
  quietly expand the trust surface — it stays inside the governance the repo already
  enforces.
- **Deliberately strong.** It asks for full 3-OS parity, a trait-based injector, and a
  complete test matrix in one spec. If a given Fable run can't do all of it at once,
  split at the PHASE markers — each phase is independently shippable and independently
  gated. Revise freely before running; this is a source artifact, not a one-shot.
