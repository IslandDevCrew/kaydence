# context/ — Local Context Reading (opt-in, in-memory)

Read ADR-0006 before editing. This module is governed first and foremost by
non-negotiable #1: **the ban is on transmission and persistence, not awareness.**

## Owns
Two tiers of local context acquisition that improve prediction quality:
- **Tier 1 — Accessibility text read (primary, lightweight):** read surrounding
  text in the focused field/document via AX (macOS) / UI Automation (Windows) —
  the same APIs `inject/` uses. Cheap, precise, cross-platform.
- **Tier 2 — On-device OCR (optional, heavier):** for apps that don't expose
  text, recognize the region near the cursor with local Vision (macOS) / Windows
  OCR. Off by default.

## Does not own
Prediction generation (prediction/ consumes our output), injection, ASR.

## Invariants — absolute
1. **Opt-in, default OFF**, both tiers. First-run explains plainly; the app is
   fully functional if declined.
2. **In memory only.** Context is built, handed to `prediction/`, and dropped.
   **Never written to disk. Never transmitted. Never logged verbatim** (logs may
   record that context was used and its token length — never its content).
3. **Secure fields blocked.** Never read password/secure inputs (OS enforces;
   we double-check and refuse).
4. **No cloud screen capture, ever** — Tier 2 is local OCR, not a screenshot
   pipeline; nothing leaves the process. This is the bright line vs. Wispr (P6).
5. **Bounded.** Cap context window size and acquisition frequency to protect the
   prediction latency budget; Tier 2 runs at most once per paused-prediction.
6. **Per-app honor.** Respect the same secure/blocked-app policy as `inject/`;
   a user can disable context per app in `profiles/`.

## Tests
Tier-1 read returns surrounding text and is dropped (no persistence — assert no
disk write, no log content), secure-field refusal, Tier-2 OCR gated off by
default, frequency/size caps, per-app disable.
