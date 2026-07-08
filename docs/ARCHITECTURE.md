# Kaydence Architecture

**Status:** v1.0 draft · Read root `AGENTS.md` first. ADRs in `docs/decisions/` record why.

## 1. Stack

- **Shell:** Tauri 2.x — Rust core + system webview. Chosen over Electron for
  footprint and over pure-native for single-codebase Mac/Win parity (ADR-0001).
- **Backend (the product):** Rust, in `apps/desktop/src-tauri/`. All capture,
  ASR, cleanup, injection, persistence.
- **Frontend (presentation only):** React + TypeScript in `apps/desktop/src/`.
  Settings, HUD overlay, history browser. No business logic.
- **ASR:** whisper.cpp bindings (GPU/Metal path) + ONNX Runtime for Parakeet V3
  (CPU path) + Silero VAD. Abstracted behind the `AsrEngine` trait (ADR-0002).
- **Cleanup:** staged pipeline — deterministic rule engine, then optional LLM
  (local Ollama or BYOK OpenAI-compatible endpoint) (ADR-0003).
- **Persistence:** SQLite (rusqlite) at the OS app-data dir; audio WAV/FLAC
  files alongside, WAL-style write-before-process.
- **Reference codebase:** Handy (MIT, github.com/cjpais/Handy) — we study its
  Tauri+Whisper+Parakeet+VAD plumbing and may vendor patterns with attribution
  (ADR-0004). We are not a fork; we are a superset with a cleanup layer.

## 2. Process & thread model

```
┌────────────────────────── Tauri process ──────────────────────────┐
│  Main thread: Tauri runtime, window/tray, IPC                     │
│  RT audio thread (cpal callback): ring buffer only — NEVER blocks │
│  Capture task: drains ring buffer → WAL file → VAD → ASR feed     │
│  ASR worker (dedicated thread/pool): model inference, partials    │
│  Cleanup task (tokio): rules → optional LLM call                  │
│  Injection (main/UI thread as required per-OS)                    │
└───────────────────────────────────────────────────────────────────┘
```

Rules:
- The cpal callback copies samples into a lock-free ring buffer and returns.
  Any allocation, lock, log, or syscall in that callback is a defect.
- Model load happens at startup or model-switch, never on the hot path. Keep
  the selected model resident (within the 250 MB idle budget — quantized).
- One dictation = one `SessionId` (ULID) threading through every stage and log line.

## 3. Pipeline stages & ownership

| Stage | Module | Contract |
|---|---|---|
| Capture | `audio/` | Mic → ring buffer → WAL file; emits `AudioChunk` |
| Gate | `audio/` (VAD submodule) | Silero VAD; emits speech-only chunks |
| Recognize | `engine/` | Chunks → `Partial(text)` stream → `RawFinal(text)` |
| Clean | `cleanup/` | `RawFinal` + dial + profile → `CleanFinal(text)` |
| Personalize | `dictionary/` | Term boosting (pre-ASR hints) + post-pass replacement + snippets |
| Target | `profiles/` | Frontmost-app detection → `AppProfile` (tone, dial override, prompt) |
| Deliver | `inject/` | `CleanFinal` → focused field; fallback clipboard w/ restore |
| Remember | `history/` | Persist session row: audio path, raw, clean, app, timings |
| Configure | `settings/` | Single source of config truth; typed; hot-reload |

## 4. Event contract (the only coupling allowed between stages)

Rust enum (single source of truth; frontend mirrors via generated TS types):

```rust
enum SessionEvent {
    Started { id: SessionId, target_app: AppRef, at: Instant },
    AudioPersisted { id: SessionId, wal_path: PathBuf },
    Partial { id: SessionId, text: String, t_lag_ms: u32 },
    RawFinal { id: SessionId, text: String },
    CleanFinal { id: SessionId, text: String, dial: CleanupDial },
    Injected { id: SessionId, method: InjectMethod },
    Held { id: SessionId, reason: HoldReason },      // e.g., focus changed, secure field
    Failed { id: SessionId, stage: Stage, error: String }, // text preserved in history regardless
}
```

Every stage consumes upstream events and emits its own; no stage imports
another stage's internals. Integration tests in `tests/integration/` assert
event sequences for: normal flow, crash-mid-session, focus-change, secure
field, short utterance (<1 s), cleanup-LLM timeout.

## 5. Platform abstraction

`inject/`, `hotkeys/`, and parts of `audio/` and `profiles/` are the only
modules allowed `#[cfg(target_os)]` blocks, each behind a trait:

- macOS: Accessibility API insertion; CGEvent fallback; AX frontmost-app;
  global monitor for hotkeys (needs Input Monitoring + Accessibility perms —
  first-run flow must request them gracefully).
- Windows: UI Automation `ValuePattern`/`TextPattern` insertion; `SendInput`
  fallback; `GetForegroundWindow` + process name for profiles; RegisterHotKey
  / Raw Input for hotkeys.

The clipboard fallback is shared: snapshot → set → paste keystroke → restore,
bounded at 200 ms, with the snapshot kept until restore confirms.

## 6. Cleanup stage design (the differentiator — treat as core IP)

1. **Rule engine (always available, ~0 ms):** filler-token removal ("um",
   "uh", "you know" — locale-aware list), repeated-word dedupe, basic
   punctuation/capitalization, spoken-command tokens ("new line", "bullet").
2. **Self-correction collapse:** detect retraction markers ("actually",
   "no wait", "scratch that", restart-of-clause) → keep the corrected span.
   Rules catch the easy 80%; the LLM pass catches the rest in Light mode when
   a local model is available.
3. **LLM pass:** strict prompt contract — *edit, don't author*. Output must be
   a transformation of the input; we diff-check that token overlap stays above
   a threshold in Light mode and reject hallucinated rewrites (fall back to
   rule-engine output). Prompts live in `cleanup/prompts/` and are versioned;
   changing one requires updating its golden-transcript test set.
4. **Dial semantics:** Raw = stage skipped. Light = rules + constrained LLM.
   Full = profile prompt rewrite. Per-app override; per-invocation override
   via secondary hotkey.

## 7. Model management

`models/` defines a registry (JSON): name, task, file, sha256, size, source
URL, license, min-hardware. The app downloads on demand, verifies checksums,
stores under app-data. **No weights in git.** ≥2 interchangeable local ASR
models at all times (supply-chain/licensing hedge).

## 8. Observability (local-only)

Structured tracing (`tracing` crate) to a local rotating file, per-session
timing spans matching the budget table. A "Diagnostics" panel in the UI reads
the local log. Nothing is ever transmitted. `--bench` mode emits the timing
table `scripts/bench.sh` consumes.

## 6b. Whisper-Ahead: prediction + context (ADR-0005 / 0006 / 0007)

**Prediction is a parallel, local-only layer**, orthogonal to the cleanup dial.
It branches off the same speech stream and obeys: only committed text injects;
dictation always wins latency contention.

### Surface router
At session start, `prediction/` asks `profiles/` whether the frontmost app can
host the HUD overlay (capability table + per-app override):
- **Surface A (HUD dual-line):** Streaming (continuous, ≤400 ms) or Paused-Merge
  (5 s). Tokens + an anchor hint go to the frontend; the blue lane tracks the
  mic's column. Merge → `PredictionMerged` → frontend gold pulse (~0.5 s) →
  settles to text color.
- **Surface B (inline fallback):** streaming disabled; after 10 s pause, one
  inline ghost the host app renders; Tab accepts.

### Context (opt-in, in-memory)
`context/` supplies surrounding text to the prediction context builder — Tier 1
accessibility read (AX/UIA) or Tier 2 local OCR — held in memory, never persisted
or transmitted (non-negotiable #1, ADR-0006). Bounded in size and frequency to
protect the prediction budget.

### Merge matcher
Compares newly recognized tokens against the live prediction (normalized,
leading-token / prefix match); say → merge; Tab/Shift-Tab → explicit accept;
mismatch → dismiss + re-guess. Word-by-word, accept-all via Shift-Tab.

### Timing ladder (configurable)
`5 s` → Surface A paused-merge prediction · `10 s` → Surface B inline prediction ·
`20 s` of silence → recording auto-stops (10 s + 5 s decide + 5 s grace).

### Events added to SessionEvent (§4)
`PredictionOffered{tokens, mode, surface}` · `PredictionMerged{tokens, source}` ·
`PredictionDismissed{reason}` · `PredictionStale`. Same coupling rule: stages
emit/consume events only.

### Analytics
`prediction_events` (extends `history/`): tokens, mode, surface, source, app, ts.
Aggregations power a local dashboard (words merged, offered-vs-merged acceptance,
by app, model). Local-only, exportable, deletable.
