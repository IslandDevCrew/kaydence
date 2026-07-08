# audio/ — Capture & VAD

## Owns
Device enumeration/selection, the cpal capture stream, the lock-free ring
buffer, write-ahead persistence of raw audio (WAL), Silero VAD gating, and the
resampling to the 16 kHz mono PCM the engines expect.

## Does not own
Transcription (engine/), deciding when to record (hotkeys/), retention policy (history/).

## Invariants
1. **The cpal callback is sacred:** copy into the ring buffer and return.
   No locks, no allocation, no logging, no syscalls. Violations are defects
   even if they "work."
2. **WAL before ASR:** samples hit the on-disk session file before the engine
   sees them. fsync at session end and at every 5 s mark. A `kill -9` at any
   moment must leave a playable file (the crash-recovery test enforces this).
3. **Tail buffer:** keep streaming 300 ms after stop-signal and include a
   pre-roll of 250 ms before VAD's first speech frame — this is the fix for
   pitfall P1 (truncated short utterances).
4. VAD gates what reaches the engine but never what reaches the WAL file.
5. Device hot-unplug mid-session: emit `Failed{stage: Capture}` *after*
   flushing whatever was captured; never panic the audio thread.

## Conventions
Sample format normalized to f32 mono 16 kHz at this module's boundary; ring
buffer sized for ≥2 s at device rate; session files named `{ulid}.wav` under
app-data `sessions/`, FLAC-compressed by history/ after finalize.

## Tests that must exist
Ring-buffer overflow behavior, WAL crash recovery, 0.3 s utterance end-to-end
capture, VAD pre-roll inclusion, device-switch mid-session.
