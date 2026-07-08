# Kaydence PRD — Product Requirements Document

**Status:** v1.0 draft · **Owner:** Jon Isaac (IDC) · **Working title:** Kaydence
**Companion docs:** `COMPETITIVE-ANALYSIS.md` (evidence), `ARCHITECTURE.md` (how), `ROADMAP.md` (when)

---

## 1. Problem

Dictation users today choose between two failure modes:

- **Cloud AI tools** (Wispr Flow, Aqua Voice, Willow): excellent output, but
  privacy overreach (screen capture, telemetry, forced login items), heavy
  idle footprint (~800 MB RAM reported), $96–240/yr subscriptions, degraded or
  dead offline, and Windows treated as a second-class port.
- **Local/built-in tools** (Handy, VoiceInk, Superwhisper, Apple/Windows/Google
  dictation): private and cheap, but raw output imposes an "edit tax" (fillers,
  verbatim self-corrections), multi-second finalize latency, Mac-only skew, and
  zero context awareness.

No shipping product is simultaneously: **local-first, cross-platform
(Mac+Win+Linux), low-latency, with a quality-controlled intelligence layer.**

## 2. Vision & positioning

One hotkey in any app on macOS, Windows, or Linux. Speech becomes clean text in
under a second. Nothing leaves the machine unless the user explicitly wires up a
cloud key. Cleanup intensity is the user's dial, not the vendor's opinion. Linux
is first-class (X11 + Wayland), not a post-traction port (ADR-0011).

**Positioning line:** *The dictation tool that respects your machine, your
privacy, and your voice.*

## 3. Users

- **Primary:** builders/operators who live in editors, terminals, email, and AI
  chat (Cursor, Claude Code, Gmail, Slack) and dictate heavily across mixed
  macOS / Windows / Linux fleets.
- **Secondary:** privacy-sensitive professionals (legal, health-adjacent, faith
  communities) who cannot send audio to third-party clouds.
- **Tertiary:** accessibility users needing reliable hands-free input.

## 4. Feature requirements

Every feature carries an ID (referenced in PRs) and an evidence tag pointing at
`COMPETITIVE-ANALYSIS.md` sections.

### P0 — MVP (cannot ship without)

| ID | Requirement | Acceptance criteria | Evidence |
|---|---|---|---|
| P0-1 | Global hotkey, push-to-talk + toggle modes | Configurable; works when any app has focus; min-capture 250 ms; tail buffer 300 ms | CA §Praise-3, Pitfall P1 |
| P0-2 | Local ASR, dual path | Parakeet V3 (CPU) and Whisper large-v3-turbo (GPU/Metal); model picker; auto-recommend by hardware | CA §Handy, §Spokenly |
| P0-3 | Universal text injection, Mac + Win + Linux | Native injection (AX API / SendInput+UIA / X11 XTest + Wayland virtual-keyboard-portal); clipboard fallback with snapshot-restore ≤200 ms; secure-field refusal | CA §Complaint-6, Pitfalls P2/P8; ADR-0011 |
| P0-4 | Write-ahead audio persistence + history | Audio on disk before ASR starts; SQLite history of audio+raw+clean; survives crash kill -9 mid-dictation | CA §Praise-5, Pitfall P7 |
| P0-5 | Cleanup dial: Raw / Light / Full | Light default = filler removal + self-correction collapse + punctuation via rules + small local model; Full = LLM rewrite, opt-in; per-invocation override hotkey | CA §Praise-1, §Complaint-4 |
| P0-6 | Latency budgets met | Table in root AGENTS.md §5, enforced by `scripts/bench.sh` in CI | CA §Praise-2 |
| P0-7 | Zero-trust privacy posture | No telemetry/accounts/screen capture; network audit script passes; privacy page in README states it in plain language | CA §Complaint-1 |
| P0-8 | 60-second first run | Install → model download → first successful dictation in ≤60 s on default settings, one settings screen | Pitfall P10 |

### P1 — Differentiators (the reason to switch)

| ID | Requirement | Acceptance criteria | Evidence |
|---|---|---|---|
| P1-1 | Streaming partial text | Partials render in HUD ≤300 ms behind speech; final replaces partials atomically | CA §Aqua |
| P1-2 | Custom dictionary + import | Add terms w/ optional pronunciation hints; import Wispr Flow & Glaido dictionary exports; terms boost ASR + cleanup | CA §Praise-4, §Glaido |
| P1-3 | Auto-learning corrections | When user edits injected text within 30 s, offer to learn the correction into the dictionary (explicit confirm, never silent) | CA §Willow style-memory |
| P1-4 | Per-app profiles | Detect frontmost app; apply tone/format/cleanup-level/custom prompt per app (e.g., Slack casual, Gmail formal, Cursor code-aware) | CA §Wispr, §VoiceInk Power Mode |
| P1-5 | BYOK cloud lanes | Optional: Groq/Deepgram ASR, any OpenAI-compatible LLM or local Ollama for cleanup; zero markup; per-lane kill switch; local fallback chain | CA §Spokenly |
| P1-6 | Snippets | Spoken trigger word expands to saved text | CA §Glaido |

### P2 — Expansion (post-traction)

| ID | Requirement | Evidence |
|---|---|---|
| P2-1 | Voice edit commands ("make this a list", "redo the last sentence") | CA §Aqua, §Wispr command mode |
| P2-2 | MCP server so coding agents (Claude Code, Cursor) receive voice input — *subsumed and deepened by Conductor P4-8* | CA §Spokenly |
| P2-3 | File transcription (drag a recording in) | CA §Superwhisper/MacWhisper |
| P2-4 | Agentic mode (manipulate selected text on screen via voice) | CA §Glaido |
| P2-5 | ~~Linux build~~ **PROMOTED to first-class (ADR-0011)** — no longer post-traction; X11 + Wayland injection in P1 | CA §Handy, §Talon |
| P2-6 | Mobile exploration (iOS/Android keyboard) | CA §Wispr/Willow platform spread |

### Explicit non-goals (v1)

- Meeting transcription/notetaking (Otter/Granola territory)
- Full computer voice control (Talon territory)
- Real-time translation
- Any feature requiring screen capture — permanently out, not just deferred

## 5. Non-functional requirements

- **Performance:** budgets in root `AGENTS.md` §5.
- **Footprint:** installer <60 MB (models downloaded separately, checksummed).
- **Reliability:** zero data loss on crash (P0-4 test is a hard gate);
  short-utterance suite (0.3–1.5 s clips) must pass at 100%.
- **Security/privacy:** no network egress by default; BYOK keys in OS keychain,
  never plaintext on disk; signed binaries + notarization (Mac) and code
  signing (Win).
- **Accessibility:** full keyboard operability of the settings UI; HUD readable
  at OS scaling 200%.

## 6. Licensing, pricing, distribution (DECIDED — ADR-0008 naming, **ADR-0010 pricing**)

Rule that generates the tiers: **features are one-time; only services with
recurring marginal cost may subscribe — and every service is self-hostable free.**

| Tier | Price | Contains |
|---|---|---|
| **Core** | **$0** · MIT | Local dictation (Raw/Light), global hotkey, history, single device |
| **Pro** | **$79 one-time** (launch $59), per major version | Whisper-Ahead, per-app profiles, snippets, dictionary import, context reading, analytics, **Voiceprint** |
| **Captain** | **$189 one-time** · lifetime-everything | Pro for all future majors + **Relay** + **Conductor** + priority support |
| **Harbor** | optional **~$6/mo or $60/yr** | Hosted services only (rendezvous relay, managed inference, encrypted backup) — every one self-hostable free; **no feature ever gated here** |

- **License:** open-core. MIT Core; Pro/Captain are one-time paid tiers. License
  verification is local/offline (no phone-home) to keep the zero-account posture.
- **Name:** "Kaydence" (`kaydence.io`) — formal trademark/domain pass remains a
  pre-launch gate (ADR-0008; prior art note: "Hearth" and "Sela" both died in
  trademark checks on a previous project — do this early).
- **Gemma 3 1B** (Windows-mid prediction default) carries Gemma Terms, not Apache:
  license review required before any *paid* build bundles it (ADR-0007/0010).

## 7. Success metrics

- Time-to-first-dictation ≤60 s for a new user (P0-8)
- ≥95% of dictations require zero manual edits at Light cleanup (sampled self-test corpus)
- p95 release→inject latency within budget on both reference machines (M-series Mac, mid-range Windows laptop with no dGPU)
- Crash-recovery test: 100% audio recovery
- Qualitative: daily-driver replacement of Wispr Flow for the primary user within Phase 2

## 8. Risks

| Risk | Mitigation |
|---|---|
| Windows native injection is genuinely hard (UIA quirks per app) | Earliest spike in Phase 1; matrix-test top 20 target apps; clipboard fallback always works |
| Local LLM cleanup too slow on CPU-only machines | Rule-engine-only Light mode as floor; Ollama optional; BYOK lane |
| Parakeet/Whisper licensing or model availability shifts | Model registry abstraction in `models/`; ≥2 interchangeable local models |
| Solo-maintainer bus factor (the VoiceInk critique) | This repo's AGENTS.md system *is* the mitigation — any competent agent or contributor can onboard from the docs |
| Scope creep toward Talon-style voice control | §4 non-goals; PRD-traceability rule in root charter §7.2 |

---

## 11. v2 addendum — Whisper-Ahead epic + locked decisions (2026-06-19)

### Locked decisions
- **Name:** Kaydence · `kaydence.io` (+ `getkaydence.com`). Cadence retired (ADR-0008).
- **Pricing:** free open-core (MIT) + one-time Pro **~$59–79**, no subscription (ADR-0008).
- **Prediction model:** llama.cpp/GGUF; Mac → Qwen 2.5 1.5B, Windows mid-range →
  Gemma 3 1B (license review), low-end floor → Qwen 3 0.6B; defaults locked from
  `scripts/bench-prediction.sh` (ADR-0007).
- **Privacy refinement:** opt-in, in-memory, never-transmitted local context
  reading allowed; ban is on transmission + persistence (ADR-0006).
- **Color:** blue prediction → amber-gold merge pulse → settles to text; themeable.

### P2 — Whisper-Ahead epic (predictive completion)
Depends on P1-1 (streaming partials) + the local-LLM lane being solid. Lands as a
dedicated milestone after core dictation is a daily driver.

| ID | Requirement | Acceptance criteria | Evidence |
|---|---|---|---|
| P2-7 | Local prediction engine | llama.cpp/GGUF; per-platform default model; ≤400 ms guess; yields to dictation | CA §Cotypist, ADR-0007 |
| P2-8 | Surface A — HUD dual-line | Blue lane below tracking the mic; Streaming + Paused-Merge (5 s); only committed text injects | ADR-0005, P11 |
| P2-9 | Surface B — inline fallback | Non-overlay apps: streaming off; inline ghost after 10 s pause; Tab accepts | ADR-0005, P12 |
| P2-10 | Merge + accept | Say-or-select; Tab=word, Shift-Tab=phrase; ~0.5 s gold pulse → text color | ADR-0005 |
| P2-11 | Local prediction analytics | `prediction_events`; words merged, acceptance rate, by app; local-only, exportable, deletable | ADR-0005 |
| P2-12 | Opt-in local context | Tier 1 accessibility read + Tier 2 on-device OCR; in-memory only; secure fields blocked | ADR-0006 |
| P2-13 | Timing ladder | 5 s HUD / 10 s inline / 20 s auto-stop; all configurable | ADR-0005 |

---

## 12. v3 addendum — Flagship Trio epic (2026-06-21, committed 2026-07-07)

The three features that define the v3 separation. Each requires the local-first
architecture (a cloud incumbent must abandon its business model to copy any of
them). Committed as epics **P4-1..P4-9** under **ADR-0009** (Relay crypto) and
priced per **ADR-0010** (Voiceprint → Pro; Relay + Conductor → Captain). Sequenced
Voiceprint → Relay → Conductor. Architecture: `ARCHITECTURE.md` §9.

### Flagship 01 — Voiceprint (on-device personalization)

| ID | Requirement | Acceptance criteria | Evidence |
|---|---|---|---|
| P4-1 | Correction auto-learn into dictionary | Every learn is explicit-confirm, never silent; ties to P1-3 | CA §Willow style-memory (cloud) |
| P4-2 | Local LoRA fine-tune of the prediction model | Trains on merged/edited history; idle/charging schedule; opt-in; kill switch + one-tap reset; training budget capped | CA §on-device personalization empty lane |
| P4-3 | Acoustic biasing for proper nouns | Recognition-stage biasing from the user's terms | CA §Praise-4 |
| P4-4 | Artifact ownership | Voiceprint is exportable, inspectable, deletable; carried E2E via Relay | Non-negotiable #1 |
| P4-5 | **Gate: measured acceptance-rate lift** | Lift vs. pre-Voiceprint baseline, or the feature does not advertise | ladder: Personalization lift |

### Flagship 02 — Relay (cross-device continuity, no cloud)

| ID | Requirement | Acceptance criteria | Evidence |
|---|---|---|---|
| P4-6 | Pairing + trust group | mDNS discovery + SAS short-code; signed device roster; key rotation + device revocation | ADR-0009 |
| P4-7 | E2E sync + cross-device dictation | Noise_XX transport; CRDT sync of dictionary/profiles/Voiceprint/analytics; optional self-hostable rendezvous off-network | ADR-0009 |
| P4-2g | **Gate: LAN latency + zero cloud** | ≤500 ms added on LAN, E2E-encrypted, zero cloud egress (audit-network) | ladder: Cross-device continuity |

### Flagship 03 — Conductor (voice control plane)

| ID | Requirement | Acceptance criteria | Evidence |
|---|---|---|---|
| P4-8 | MCP server + agent targets | Built-in MCP server; targets (Claude Code, Cursor, terminals, custom); per-agent profiles + command grammars; spoken command palette; subsumes P2-2 | CA §Spokenly, §Willow-to-agents |
| P4-9 | Fleet mode + confirm gate | Fleet mode via Relay; **destructive actions (delete/send/pay/deploy) never auto-execute — explicit confirm each**; confirm-gate suite 100% | Zapier 2026 voice-as-interface roundup |

**Non-goal reminder:** nothing here relaxes non-negotiable #1. Voiceprint trains
locally; Relay is E2E with a blind self-hostable relay; Conductor's confirm gate
is absolute. A single auto-executed destructive command is a release blocker.
