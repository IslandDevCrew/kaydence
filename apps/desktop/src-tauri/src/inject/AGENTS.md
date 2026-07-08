# inject/ — Text Delivery (highest platform risk in the repo)

## Owns
Getting final text into the focused control: macOS Accessibility API insertion
(CGEvent fallback), Windows UI Automation ValuePattern/TextPattern insertion
(SendInput fallback), the shared clipboard fallback (snapshot → set → paste →
restore ≤200 ms), secure-field detection/refusal, focus-binding.

## Does not own
Choosing the text (upstream), app identification for profiles (profiles/ —
though we share the frontmost-window plumbing via a common helper).

## Invariants
1. **Method ladder per OS:** native insertion → synthesized keystrokes →
   clipboard fallback. Record which method succeeded in the `Injected` event;
   per-app method overrides live in profiles when an app is known-broken.
2. **Clipboard fallback never destroys user data:** snapshot all formats we
   can, restore within 200 ms, keep the snapshot until restore is confirmed.
   (Pitfall P2 — this is a top-3 category complaint.)
3. **Secure fields:** if the focused control is a password/secure input,
   refuse, emit `Held{reason: SecureField}`, notify via HUD. Never type into
   it, never store what would have gone there beyond normal history.
4. **Focus binding:** the injection target is captured at session start. If
   focus changed by delivery time, do NOT inject — emit `Held{reason:
   FocusChanged}`, park the text in the HUD with one-tap "insert here /
   copy". (Pitfall P9.)
5. Unicode-complete: emoji, CJK, RTL, combining marks. Keystroke synthesis
   paths must handle layout independence (use unicode injection, not VK codes,
   wherever the OS allows).

## The app-compat matrix
`tests/integration/inject_matrix.md` tracks the top-20 target apps per OS
(browsers ×3, VS Code/Cursor, terminals, Slack/Discord, Office/Google Docs,
Mail, Notion, Obsidian…). Every release verifies the matrix; a regression on a
matrix app blocks release. Electron apps and web editors are the usual
offenders — expect per-app quirks and document each workaround inline.

## Definition of done
Both OSes, matrix green, clipboard-restore test green, secure-field test
green, focus-change test green.
