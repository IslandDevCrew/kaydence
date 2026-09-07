# Remaining UI truth and capture integration boundary

Read-only audit on PR #36 head `9b069ec` (also unchanged in the responsive-nav unit).
No Rust file changed, native capture started, or backend lane assumed.

## Routine frontend queue

- **Native loading/error:** `App.tsx` initializes sample snapshot/history and restores
  them on IPC failure. A rejecting browser IPC fixture displayed operational status,
  recording text and three sample sessions with no visible error/fixture label.
  Separate loading/error/ready from explicit browser preview; history failure must
  not fabricate sessions. Preserve and label stale real history on refresh failure.
- **Reference truth:** Cleanup uses runtime permissions/delivery for other OS lanes,
  unconditionally passes its second gate, and equates delivery with round-trip proof.
  A macOS fixture switched to Windows showed Windows Active with Mac permissions;
  a blocked-permissions/empty-history fixture still showed Secure Input OK.
  Reference lanes must be labeled and unproved gates remain pending.
- **OS identity:** Cleanup -> Windows -> Dictate retained cobalt and UI Automation
  + SendInput while the footer said macOS. Bind Dictate accent/method to runtime OS.
- **Typography:** computed text at 900x600 includes 9.76px rail labels and 8.16px
  phase badges; Setup also has 8-10px text. The locked floor is 11px. The responsive
  nav unit restores narrow labels only, not a full type-system/a11y closeout.

## Backend-owned bridge needed — do not implement across the lane boundary

- `settings/mod.rs:31` AppSnapshot exposes app identity/settings. CaptureSettings
  at line 325 contains thresholds, not live phase/elapsed state.
- `hotkeys/mod.rs:49` owns Idle/Capturing/Finalizing but does not expose that state.
- `events.rs:129` has Started/final/delivery events. `lib.rs:2196` records Started
  in history; subsequent finals are recorded too, without a Tauri event bridge.
- Searching frontend/backend for Emitter/emit_all/listen/emit found no applicable
  bridge; the sole emit is the Linux uinput device operation.
- `lib.rs:2739` establishes an Instant clock. Started.at_ms is elapsed from it,
  NOT Unix time. Do not subtract it from Date.now().
- `recent_history` performs retention/recovery; it is not a high-frequency clock.

Request a separately owned backend projection with session identity, explicit
capture/tail/processing/discard/failure/completion state, backend-clock elapsed
duration, bootstrap/reopen synchronization and change notification. Include
short-capture, failure and reopen tests. Frontend consumes that agreed contract.
Any SessionEvent change requires an ADR and operator gate. Injection stays with
its current owner. Until a real bridge exists, native UI must not assert live
Ready/Recording/Microphone idle from a fixture or a locally invented timer.

Evidence limits: browser fixtures prove presentation branches only. Native
permissions/injection, three-WebView parity, packaged footprint and physical
release-to-focused-field latency remain separate acceptance evidence.
