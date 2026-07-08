# Competitive Analysis — Dictation Landscape

**Verified:** June 12, 2026 via web research (vendor pages, Product Hunt, Trustpilot
signals, GitHub, independent review sites). Vendor-published benchmarks are marked
*(vendor-claimed)*. Re-verify before citing externally; this market moves monthly.

## 1. The field at a glance

| Tool | Platforms | Price | Engine | Standout | Achilles heel |
|---|---|---|---|---|---|
| Wispr Flow | Mac/Win/iOS/Android | ~$144/yr | Cloud | Tone matching, self-correction collapse, command mode | Privacy trust collapse (screen capture reports, ~2.7 Trustpilot), ~800 MB idle RAM, Electron Windows freezes |
| Aqua Voice | Mac/Win | ~$96/yr | Cloud-only ("Avalon") | Streaming output, sub-second feel, 97.4% technical-term accuracy *(vendor-claimed)*, mid-flow voice edits | No offline at all (founders say infeasible at their latency) |
| Willow Voice | Mac/Win/iOS/Android | ~$144/yr | Cloud (limited offline) | Sub-200 ms claim *(vendor-claimed)*, style memory, filler removal | Latency collapses on weak wifi; benchmarks unverified |
| Superwhisper | Mac/iOS | $249.99 lifetime | Local Whisper + cloud modes | Most configurable "modes" system; file transcription | Price, intimidating setup, long-form output needs manual cleanup |
| VoiceInk | Mac | $39.99 one-time / OSS (GPL) | Local Whisper/Parakeet | Power Mode (per-app config), open source, cheap | Short-utterance truncation bugs (hotkey races), solo-dev risk, Mac-only |
| MacWhisper | Mac | ~€59 lifetime | Local whisper.cpp | User-set post-processing prompt; file transcription | Not real-time-dictation-first |
| Spokenly | Mac/iOS | Free local + BYOK | Local Parakeet/Whisper + BYOK cloud | Best architecture in category: free local, zero-markup BYOK, MCP server for coding agents | Mac-centric |
| **Handy** | **Mac/Win/Linux** | **Free, MIT** | **Local Whisper/Parakeet + Silero VAD** | **Tauri (12.6 MB Win installer), ~20k stars, explicitly built to be forked** | Raw output only, 2–5 s finalize latency, young-project rough edges |
| Glaido (Jack Roberts) | Mac (Win pending) | $20/mo | Cloud-assisted, local storage claim | Snippets, dictionary w/ **Wispr import**, Agentic Mode | Brand new (May 2026), priciest subscription, single-platform, no track record |
| OpenWhispr | Mac (cross-platform) | Free, OSS | Local + optional cloud | Agent Mode w/ custom system prompts | Less mature than Handy |
| OS built-ins (Apple/Win+H/Google) | Native | Free | Mixed local/cloud | Zero install; Google's raw accuracy on Pixel is strong | Raw output: transcribes your self-corrections verbatim; no custom vocab, fillers kept, no context awareness (~5.5% WER cited for Google Docs voice typing vs ~3.2% best-in-class) |
| Dragon | Win | $$$ | Local legacy | Medical/legal vocab, full voice control | Price, dated UX, heavyweight |

## 2. Praise patterns (what positive reviews converge on) → build these

- **Praise-1 — Edit tax eliminated.** Filler removal + self-correction collapsing
  ("5pm… actually 6" → "6pm") is *the* feature separating paid AI tools from
  built-ins. → PRD P0-5.
- **Praise-2 — Feels instant.** Streaming partials (Aqua) beat one-chunk paste;
  >1 s breaks flow; Handy's 2–5 s is its most-cited weakness. → P0-6, P1-1.
- **Praise-3 — One hotkey, every app, identical behavior.** → P0-1, P0-3.
- **Praise-4 — Technical-vocabulary accuracy + custom dictionary.** The single
  biggest driver of developer loyalty (Aqua). → P1-2.
- **Praise-5 — Never loses a recording.** Explicitly praised where present. → P0-4.
- **Praise-6 — Per-app context** (Slack casual / Gmail formal / editor code-aware;
  VoiceInk's Power Mode). → P1-4.

## 3. Complaint patterns (what negative reviews converge on) → avoid these

- **Complaint-1 — Privacy overreach.** Screen capture for "context," telemetry,
  forced login items, mandatory accounts. The #1 stated reason users flee to
  local tools. → P0-7, pitfall P6.
- **Complaint-2 — Resource hogging.** Electron bloat; ~800 MB idle RAM reports. → pitfall P3.
- **Complaint-3 — Cloud fragility.** Lag or death on weak connections. → ADR-0002.
- **Complaint-4 — Over-polishing.** Long-term users describing AI rewrite as
  losing their voice and self-downgrading to light cleanup. → ADR-0003.
- **Complaint-5 — Subscription fatigue.** $96–240/yr for a utility; lifetime
  pricing repeatedly cited as a differentiator. → PRD §6.
- **Complaint-6 — Windows second-class.** Freezes, late ports, Mac-first
  everything. The clearest open lane in the market. → non-negotiable #5.
- **Complaint-7 — Reliability edge cases.** Truncated short utterances, hotkey
  timing races, post-trial degradation patterns. → pitfall P1, P0-8.
- **Complaint-8 — Setup complexity.** Superwhisper's server-config feel. → P0-8.

## 4. The gap Kaydence occupies

Local-first **and** intelligent **and** cross-platform **and** lightweight.
Spokenly is closest in architecture but Mac-centric; Handy is closest in
platform reach but raw-only and slow to finalize; Wispr is closest in output
quality but failing on trust, footprint, and Windows. Nobody holds all four
corners. That is the product.

## 5. Standing intel tasks for agents

- Re-check this table at each phase boundary (tools ship monthly).
- Watch: Handy's release notes (we share dependencies), Wispr's privacy posture
  (their recovery would compress our positioning), Glaido's Windows launch
  (direct overlap with our lane), Apple/Microsoft built-in upgrades (floor rises).

---

## 6. June 2026 deep-dive — Cotypist mechanism + model landscape

### How Cotypist actually handles context (settles the screen-capture question)
- **Local model:** Qwen 2.5 1.5B primary on Apple Silicon (~100–200 ms budget);
  a ~3 GB Gemma option exists. Runtime is Apple MLX — **Mac-only** (why we choose
  llama.cpp/GGUF instead, for Windows parity).
- **Context is read locally, never sent:** surrounding text via **on-device text
  recognition / accessibility**, held **in memory only, nothing stored or
  transmitted**; clipboard awareness is opt-in and in-memory; password fields are
  OS-blocked. The open clone (Cotabby) confirms the pattern (llama.cpp; optional
  local screen-area read; never transmits text/screen/suggestions).
- **Takeaway:** "screen capture for context" ≠ "cloud screenshots." The former is
  safe and local (→ ADR-0006); the latter is the Wispr failure (permanently
  banned). This distinction is the basis for our `context/` module.

### Small-model landscape (for prediction)
- **Qwen 2.5 1.5B** (Apache 2.0): proven for this use case; best tooling. Mac primary.
- **Qwen 3 0.6B** (Apache 2.0): newest-gen, ultra-low latency; CPU/low-end floor.
- **Gemma 3 1B** (Gemma Terms — not Apache): strong on-device quality; Windows
  mid-range default per operator; **license review before commercial bundling**.
- Runtime: **llama.cpp/GGUF** (cross-platform) over MLX (Mac-only). Defaults
  locked from on-device bench (`scripts/bench-prediction.sh`).
