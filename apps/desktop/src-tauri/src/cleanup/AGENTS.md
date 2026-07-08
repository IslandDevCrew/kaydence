# cleanup/ — The Differentiator (treat as core IP)

## Owns
The Raw/Light/Full dial. Stage 1: deterministic rule engine (filler removal,
repeated-word dedupe, punctuation/caps, spoken formatting tokens). Stage 2:
self-correction collapse (rules first, constrained LLM assist in Light).
Stage 3: Full-mode profile-prompt rewrite. LLM client (local Ollama or BYOK
OpenAI-compatible). Prompt versioning under `prompts/`.

## Does not own
Per-app profile *selection* (profiles/ hands us the active profile),
dictionary replacements (dictionary/ runs after us), ASR.

## Invariants — these encode the product thesis, do not weaken them
1. **Raw means untouched.** Dial=Raw bypasses this module entirely.
2. **Light edits, never authors.** The constrained LLM pass must keep token
   overlap with input above the configured threshold (default 0.7); below it,
   discard the LLM output and ship rule-engine output. This is the structural
   guard against pitfall P4 ("the soul is gone").
3. **Self-correction collapse keeps the corrected span:** "Tuesday, no wait,
   Friday" → "Friday". Marker lexicon lives in `rules/corrections.rs`,
   locale-aware, with a golden test per marker.
4. **Fallback chain:** LLM unavailable/timeout (>800 ms Light budget) →
   rule-engine output → never block injection on a model.
5. **Prompts are versioned artifacts.** Any prompt change updates its golden
   transcript suite in the same PR; CI diffs outputs against goldens.
6. Both raw and cleaned text persist to history for every session (enables
   future correction-learning, P1-3, and user trust via diff view).

## Quality bar
The PRD metric lives here: ≥95% of golden-corpus dictations need zero manual
edits at Light. The corpus (`tests/corpus/`) must include: fillers, mid-
sentence restarts, technical vocab, numbers/dates/times, spoken punctuation,
multi-clause corrections, and deliberately messy thinking-out-loud samples.

## Pitfalls
LLMs in non-English locales substituting languages mid-output (observed in a
competitor) — assert output language matches input. Never let Full mode become
the silent default through a profile misconfiguration: profile-driven dial
escalation to Full requires the profile to have been user-edited, not shipped.
