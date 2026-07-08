# ADR-0002: Local-first hybrid ASR engine

- **Status:** Accepted
- **Date:** 2026-06-12
- **PRD items affected:** P0-2, P1-5

## Context
Cloud-only tools die offline and leak audio by design (Aqua's founders state
local ASR+LLM at their speed target isn't feasible *for them*). Local-only
tools cap accuracy on technical vocabulary. Spokenly's free-local + BYOK-cloud
model is the strongest architecture observed in the category.

## Decision
`AsrEngine` trait with three implementations: Parakeet V3 via ONNX Runtime
(CPU default), whisper.cpp large-v3-turbo (GPU/Metal), and an opt-in BYOK
cloud lane (Groq/Deepgram). Fallback chain: cloud → local GPU → local CPU.
Local must always produce a result.

## Alternatives considered
- Whisper-only: weak CPU latency story. Parakeet-only: GPU owners leave
  accuracy on the table. Cloud-default: violates non-negotiable #1.

## Consequences
Two inference runtimes to maintain; the model registry (`models/`) must keep
≥2 viable local models; engine selection logic needs hardware detection.
