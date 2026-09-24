# ADR-0021: Race-free close-to-tray — intercept cockpit close in the app event loop

- **Status:** Accepted <!-- Operator gave explicit `go` on PR #55, 2026-09-24, after the 3-OS CI pass and the live Windows evidence. Removed from the check-adr-status.sh exception list on acceptance. -->
- **Date:** 2026-09-24
- **PRD items affected:** P1-G3 (Windows close-to-tray blocker); P1-P0-1 (tray-owned
  hotkey runtime); supports src-tauri AGENTS.md invariant 7.

## Context

Invariant 7: a user close destroys the cockpit WebView but keeps the tray-owned
hotkey/audio runtime alive; tray interaction recreates the window; explicit Quit
terminates. `DesktopLifecycle` implements the process half: a one-shot
`window_close_pending` flag lets exactly the next `ExitRequested { code: None }`
(Tauri's "last window destroyed" request) be prevented, while `app.exit(0)`
(`code: Some(0)`) always terminates.

The 2026-07-14 Windows diagnostic
(`ops/mission/evidence/2026-07-14-p1-g3-windows-runtime-diagnostic.txt`) posted
`WM_CLOSE` 47 ms after the window appeared and the process exited — recorded as a
Windows platform defect, cause unknown, "causal instrumentation required".

Instrumentation on the Windows NUC on 2026-09-24
(`ops/mission/evidence/2026-09-24-p1-g3-windows-close-race.txt`) found the cause.
It is a **startup race, not a Windows exit-code difference**:

1. The close interceptor (`prevent_close` + mark + `destroy`) was registered with
   `Builder::on_window_event`. Tauri attaches that per-window listener
   *asynchronously*: `tauri-runtime-wry` posts
   `WindowMessage::AddEventListener` through the event-loop proxy.
2. The window is visible before that message is processed. A close that arrives
   in the gap reaches `on_close_requested` with no listener, so it is not
   prevented and nothing marks the flag. The resulting
   `ExitRequested { code: None }` is not intercepted, and the process exits.
3. The gap is ~300–600 ms after the window appears on an idle release build
   (0/9 survived closes at 0–300 ms; 2/3 at 600 ms), and longer under CPU load.
   Settled closes (≥1 s) always survived — which is why normal testing missed it.

Windows reports `code: None` for the last-window exit request — the same as macOS
and Linux. An unmerged local draft (2026-07-15, numbered "0016", never pushed)
assumed Windows reports `Some(0)` and widened the guard to accept it; the
instrumented trace disproves that premise, and widening the guard would not have
prevented the race.

## Decision

Intercept the cockpit close in the **app-level `RunEvent` callback**
(`RunEvent::WindowEvent { event: CloseRequested { api }, .. }` in `app.run`), not a
`Builder::on_window_event` listener, and remove the listener path.

- The app callback is installed before the event loop starts, and
  `on_close_requested` invokes it before checking the prevent signal, so every
  close — including one in the startup gap — is marked and prevented.
- The proven sequence is unchanged: mark pending → `prevent_close` → `destroy`;
  a failed destroy cancels the one-shot flag.
- If the manager does not hold the window yet, the close is not prevented:
  Tauri's default close destroys it, and the already-marked flag keeps the tray
  runtime alive. Tauri's manager drops destroyed windows on the app-level path
  (`tauri` `app.rs` `on_event_loop_event` → `manager.on_window_close`), so tray
  reopen recreates the cockpit from config either way.
- `DesktopLifecycle` semantics and tests are unchanged.

## Alternatives considered

- **Accept `Some(0)` in the exit guard (the 2026-07-15 local draft).** Rejected:
  Windows emits `None` here, and a close that was never marked still exits.
- **Always prevent `ExitRequested { code: None }` (drop the one-shot flag).**
  Rejected for this unit: it changes the tested failed-close contract and the
  macOS-verified semantics without evidence that it's needed.
- **Hide instead of destroy.** Rejected by invariant 7 (the WebView must be freed).
- **Delay showing the window until listeners attach.** Rejected: slower perceived
  startup, and still depends on Tauri's internal message ordering.

## Consequences

- Easier: the close path no longer depends on Tauri's asynchronous listener
  timing; one synchronous place owns close interception on all three desktops.
- Must maintain: never move main-window close handling back into a
  `Builder::on_window_event` / per-window listener (invariant 7 now says so).
- Verification owed: this box proves Windows (race sweep 18/18, same-PID tray
  reopen, tray Quit). macOS and Linux run the same code path; their live
  close/reopen/Quit proofs should be re-run on their reference machines.
