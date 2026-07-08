# models/ — Model Registry (no weights in git, ever)

## Owns
`registry.json`: every model the app can use — name, task (asr|vad|cleanup),
artifact filename, sha256, size, source URL, license, min-hardware tag,
recommended flag. Download/verify scripts. The hardware-recommendation table
(which model the first-run flow auto-selects).

## Invariants
1. Weights never enter git or releases; the app downloads on demand and
   verifies sha256 before load. A checksum mismatch quarantines the file.
2. ≥2 viable local ASR models at all times (ADR-0002 supply-chain hedge).
   Current set: Parakeet V3 (CPU lane), Whisper large-v3-turbo (GPU lane),
   Whisper small (low-end fallback), Silero VAD.
3. Every registry entry records its license; anything non-permissive for
   commercial redistribution needs an ADR before inclusion.
4. Adding/upgrading a model requires the bench before/after table (WER +
   latency per platform) in the PR.
5. Mirror URLs: each artifact lists ≥1 fallback source.

## Prediction models (added — ADR-0007)
Three candidates registered for the Whisper-Ahead layer, runtime **llama.cpp/GGUF**
(cross-platform; not MLX, which is Mac-only):
- `qwen2.5-1.5b` (Apache-2.0) — macOS default.
- `gemma3-1b` (**Gemma Terms — license review required; download on demand, do
  not bundle**) — Windows mid-range default per operator.
- `qwen3-0.6b` (Apache-2.0) — low-end / CPU-only floor.
Per-platform defaults are locked from `scripts/bench-prediction.sh` run on the two
reference machines. Prediction models are opt-in (only downloaded if the user
enables Whisper-Ahead) and carry their own RAM budget (root §5).
