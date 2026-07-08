# prediction/ — Whisper-Ahead (predictive completion · core IP)

Read `docs/ARCHITECTURE.md` §6b and ADR-0005 before editing. This is the layer
no competitor ships cross-platform; treat its invariants as product-defining.

## Owns
The prediction LLM client (opt-in local model via `models/`), the **scheduler**
(two triggers), the context builder (committed text + current partial, plus
optional `context/` input), the **surface router** (A vs B by app capability),
the **merge matcher** (say-or-select), per-prediction analytics emission, and the
`Prediction*` events.

## Does not own
Rendering (frontend HUD), the analytics dashboard UI (frontend), ASR (engine/),
context acquisition (context/ — we consume its in-memory output), cleanup.

## The two modes (Surface A — HUD-capable)
- **Streaming:** debounced request on each new token; cancel-and-reissue on
  context shift; rate-capped; render in the blue lane below only if a fresh
  guess returns inside the ≤400 ms prediction budget. The lane tracks the mic's
  column (frontend positions; we supply tokens + anchor hint).
- **Paused-Merge:** a VAD-driven silence timer (default 5 s, configurable) fires
  one request, renders 2–5 words below, arms the matcher.

## Surface B — inline fallback (non-HUD apps: editors, Gmail, terminals)
Streaming is **disabled** here (P12). After a longer pause (default 10 s) emit a
single inline ghost prediction the host app renders Cotypist-style; Tab accepts.
The surface router selects B when the frontmost app can't host the overlay
(capability table in `profiles/` + a per-app override).

## Merge matcher (say-or-select, word-by-word)
When new recognized tokens arrive with a prediction live, normalize and compare
against the predicted sequence. Leading-token match → emit `PredictionMerged{source:Spoken}`
and signal the gold pulse; partial match merges the matched prefix; mismatch
dismisses and re-guesses. Explicit accept: **Tab** = next word
(`source:Selected`), **Shift-Tab** = whole predicted phrase.

## Invariants — do not weaken
1. **Local only.** Prediction never uses a cloud model. (Cleanup/ASR may BYOK;
   prediction may not — it is always on-device.) Non-negotiable #6.
2. **Opt-in, own budget.** The prediction model loads only when enabled; it must
   respect the §5 added-RAM and ≤400 ms budgets. If a guess threatens a
   dictation latency budget, **prediction yields** (non-negotiable, Principle 3).
3. **Only committed text injects.** Predictions live in the HUD (A) or as an
   un-accepted ghost (B); `inject/` only ever receives merged/committed words.
4. **Every merge is counted, locally.** Emit analytics on each merge (tokens,
   mode, surface, source, app, ts). Never transmitted. This answers "is this
   earning its place?" and may later feed local personalization.
5. **The gold pulse is a signal, not state.** Frontend renders the ~0.5 s
   amber-gold glow on merge, then settles to text color. Color/duration themeable
   via `settings/`; gold is the default (ADR-0005 rationale: red reads as error).
6. **Output language matches input language** (guard against a competitor's
   observed mid-output language swap).

## Events (added to SessionEvent)
`PredictionOffered{tokens, mode, surface}` · `PredictionMerged{tokens, source}` ·
`PredictionDismissed{reason}` · `PredictionStale`.

## Tests
Matcher fuzzy-match (say variants), Tab/Shift-Tab accept, surface routing (A vs B
per app), streaming cancel-reissue under rapid tokens, paused-trigger timing,
budget-yield under contention, analytics row correctness.
