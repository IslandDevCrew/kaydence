# profiles/ — Per-App Context

## Owns
Frontmost-app detection (macOS AX / Windows GetForegroundWindow + process),
the AppProfile model (display name, match rule, tone, cleanup-dial override,
custom Full-mode prompt, preferred injection method), shipped starter profiles
(generic Email / Chat / Code / Docs), and profile resolution at session start.

## Invariants
1. Resolution happens once, at session start, and rides the session — focus
   changes mid-dictation never swap the profile (inject/ handles the
   focus-change hold separately).
2. Match rules: bundle-id/process-name exact first, then user-defined glob.
   Unknown apps get the Default profile; we never guess tone from window
   titles or content. **Reading window/screen content beyond app identity is
   banned (non-negotiable #1).**
3. Shipped profiles may set dial to Light at most; only user-edited profiles
   may select Full (see cleanup/ invariant 6).
4. Profile changes hot-apply; no restart.

## Tests
Resolution precedence, default fallback, the "shipped-profile cannot be Full"
guard, per-OS detection smoke tests.
