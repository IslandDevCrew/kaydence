# hotkeys/ — Global Shortcuts

## Owns
Global hotkey registration (all three OSes: macOS/Windows/Linux X11+Wayland), push-to-talk vs toggle semantics,
debounce, the secondary per-invocation dial-override chord, conflict detection
with OS/app shortcuts, and emitting start/stop intents to the session manager.

## Invariants
1. **Timing discipline (pitfall P1):** debounce 30 ms; a press shorter than
   the 250 ms minimum capture window is treated as an accidental tap and
   discarded before downstream processing; release still carries a 300 ms tail
   buffer so real speech is not clipped.
2. Push-to-talk: key-down starts, key-up stops. Toggle: same key starts/stops
   with a hard 5-minute auto-stop safety. Both modes always available;
   default push-to-talk.
3. Registration failure (conflict) must surface in onboarding with a one-tap
   rebind — never silently dead.
4. macOS global monitoring requires Input Monitoring permission — degrade to
   an explanatory state, not a crash, when missing. Handy's macOS shortcut
   rewrite is the reference for the known OS quirks (ADR-0004).
5. Fn/Globe and media keys behave differently per OS/keyboard — keep a tested
   allowlist of recommended default chords (current default: hold Right-Alt /
   Right-Option; revisit in beta).

## Tests
Rapid double-tap, hold-under-250ms, toggle auto-stop, rebind flow, conflict
detection fake.
