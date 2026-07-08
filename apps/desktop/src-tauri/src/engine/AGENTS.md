# engine/ — ASR

## Owns
The `AsrEngine` trait and its implementations: Parakeet V3 (ONNX Runtime, CPU
default), whisper.cpp large-v3-turbo (GPU/Metal), BYOK cloud (Groq/Deepgram).
Streaming partial emission, engine selection/fallback chain, model lifecycle
(load/warm/unload), language detection handoff.

## Does not own
Model downloading/verification (models/ registry + settings/), audio capture,
any text mutation beyond what the model outputs (cleanup/ owns that).

## Invariants
1. Emit `Partial` events ≤300 ms behind speech when the active engine supports
   streaming; emit `RawFinal` within budget (root AGENTS §5) after end-of-speech.
2. Fallback chain is mandatory and ordered: BYOK (if configured) → local GPU →
   local CPU. A cloud timeout (>2 s) silently falls back and logs locally;
   the user gets text, not an error.
3. Dictionary hints (biasing terms) are accepted as input where the engine
   supports them; ignore gracefully where not.
4. Engines are hot-swappable at runtime; switching never drops an in-flight session.
5. Raw model output is preserved verbatim into the session record — partial
   hallucination cleanup is cleanup/'s job, with the raw kept for diffing.

## Benchmarks
`scripts/bench.sh` drives this module with the golden audio corpus; WER and
latency per engine per platform land in `bench-results/` (gitignored, summary
in PR). A model upgrade PR must show the before/after table.

## Pitfalls
Whisper hallucinates on silence/noise — VAD gating upstream is the first
defense; reject finals whose no-speech probability exceeds threshold. ONNX
Runtime + Metal/DirectML feature flags differ per OS — keep provider selection
in one function with explicit logging of what was chosen.
