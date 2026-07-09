# P1-P0-3 Windows Injection Backend — Build Prompt (Fable-Safe)

The build prompt for the **Windows lane** of universal text injection, to run on the
Windows Fable box. It mirrors the already-validated Linux backend and reuses the tested,
platform-agnostic core. This is authorized product engineering on the **owned, private**
repo `IslandDevCrew/kaydence` — it is not a bypass of any OS control; Windows injection
here uses documented, user-facing OS input APIs (UI Automation, SendInput) with an
explicit **secure-field refusal** as the safety invariant.

> Paste the ROLE + CONTEXT + TASK below into the Fable session on the Windows box.
> Everything outside the fenced ```prompt``` block is guidance to you, not the model.

---

## What's already true (don't re-derive)

- **Repo:** `IslandDevCrew/kaydence` (private), branch `main`, 3-OS GitHub Actions CI is
  live and green. Clone it on the Windows box.
- **The platform-agnostic injection core is DONE + tested** (`apps/desktop/src-tauri/src/inject/mod.rs`):
  `FieldKind`, `decide_secure()` (the secure-field gate — non-negotiable #8),
  `KeystrokeChannel` (incl. `WindowsSendInput`), `InjectorCaps` + `select_plan()`,
  `Clipboard` trait + `clipboard_paste()` (snapshot→set→paste→**restore always**),
  `outcome_event()`, and the `TextInjector` trait. Build **behind** these — do not
  reinvent them.
- **The Linux backend is the reference pattern** (`inject/linux.rs` = AT-SPI detection;
  `inject/uinput.rs` = keystroke synth). Validated live on the GNOME VM. Strategy +
  findings: `docs/spikes/P1-P0-3-wayland-injection.md` and ADR-0013.
- **Key Windows advantage over Linux:** on Linux, AT-SPI `InsertText` no-ops on modern
  GNOME, so keystrokes are primary. On **Windows, UIA `ValuePattern.SetValue` /
  `TextPattern` native insertion actually works**, so native insert is the PRIMARY path,
  with `SendInput` as the keystroke fallback — the inverse priority of Linux.

## The prompt

```prompt
ROLE
You are a senior Windows systems engineer implementing the Windows text-injection backend
for Kaydence (Tauri 2 + Rust). You value correctness, least privilege, and native Windows
idioms. Small, reviewable, tested increments; you stop at declared decision points.

CONTEXT (authorized, owned private repo)
- Clone IslandDevCrew/kaydence (private), branch main. 3-OS CI is live + green — keep it
  green on every push (Windows leg included).
- Read + conform to: AGENTS.md (non-negotiables), docs/PRD.md (P0-3),
  docs/ARCHITECTURE.md §5 (Windows: UIA ValuePattern/TextPattern insert; SendInput
  fallback; GetForegroundWindow + process name for profiles), ADR-0013 (injection
  strategy + amendment), docs/spikes/P1-P0-3-wayland-injection.md (the Linux findings
  that shaped the shared architecture).
- Build BEHIND the existing platform-agnostic core in src/inject/mod.rs — reuse
  decide_secure(), select_plan(), clipboard_paste(), outcome_event(), and implement the
  TextInjector trait. Mirror the module shape of inject/linux.rs.

OBJECTIVE
Implement the Windows backend of P1-P0-3 to production quality, validated live on this
Windows machine.

REQUIREMENTS
1. Dependencies: add the `windows` crate (Microsoft official) under
   [target.'cfg(target_os = "windows")'.dependencies] with only the features you need
   (UI_Automation, UI_Input_KeyboardAndMouse for SendInput, Win32_Foundation). Each dep
   add is a critical decision path -> audit-network allowlist note + a short ADR/heartbeat
   (none are network surfaces; audit-network must stay green).
2. Secure-field DETECTION (non-negotiable #8): via UI Automation on the focused element —
   refuse when the control is a password/secure field (UIA IsPassword property, or
   ControlType Edit + the password AutomationProperty). Map the focused element to the
   shared FieldKind (Editable / Secure / NoTarget / Unknown) and run it through
   decide_secure(); NEVER inject into a Secure field.
3. INSERTION ladder (Windows-native, primary = native, unlike Linux):
   a. Native: UIA `ValuePattern.SetValue` (or `TextPattern` insertion) on the focused
      editable element -> InjectMethod::Native.
   b. Keystroke fallback: `SendInput` (Unicode KEYEVENTF_UNICODE — handles arbitrary text,
      unlike a raw scancode map) -> InjectMethod::Keystroke, KeystrokeChannel::WindowsSendInput.
   c. Clipboard fallback via clipboard_paste(): snapshot -> set (CF_UNICODETEXT) -> paste
      (SendInput Ctrl+V) -> RESTORE the original within <=200ms, always (Pitfall P2).
4. Focused-app detection: GetForegroundWindow + process name (for later per-app profiles).
5. Wire a WindowsInjector: TextInjector; make platform_injector() return it on
   target_os="windows". Finalized text is never lost — Held/Failed still keep it in
   history (outcome_event()).

SECURITY & QUALITY BAR
- Least privilege: no elevation; function (degraded) if UIA is unavailable; never assume.
- No network, no telemetry, no persistence of injected content (audit-network stays green).
- Errors surfaced to the user, never swallowed. Files <800 lines, functions <50.
- The secure-field refusal is a HARD, unit-tested invariant.

VALIDATION (this box CAN do what Linux couldn't self-drive)
- Unit-test the pure mapping (UIA control-type/property -> FieldKind) on all OSes.
- Live, operator-observed (you have a real desktop): focus Notepad/WordPad -> Kaydence
  inserts text (native UIA); focus a password field (e.g. a browser login, or the Windows
  credential prompt) -> Kaydence REFUSES. Capture evidence (screenshot/log) under
  ops/mission/evidence/.
- Also close the Windows leg of P0-G5: `cargo tauri dev` opens a window (visually confirm).
- Keep the 3-OS CI green (Windows compiles + clippy -D warnings + tests).

WORKING PROCEDURE
- One task = one branch (mission/p1-p0-3-windows-injection) = one PR/merge; <=400 lines.
- Follow prompts/BUILD-LOOP.md inside the task. Run fmt/clippy(-D warnings)/test + push so
  CI covers all three OSes; save gate evidence under ops/mission/evidence/.
- For each dependency + platform decision: audit-network note + ADR/heartbeat, then HALT
  for operator go before merging (critical decision paths).
- Heartbeat at the end: update ops/mission/state.json (P1-P0-3 windows notes),
  run `node ops/mission/render-sotu.mjs`, append ops/mission/journal.md, commit.

STOP CONDITIONS (report loudly, do not work around)
- UIA cannot detect secure status for a target -> apply the Unknown-focus policy
  (Lenient flags unverified / Strict refuses); never inject blindly into a maybe-secure field.
- A decision would weaken a non-negotiable -> stop; that's a blocking defect.
- A new dependency/surface -> HALT for operator go + ADR.

DELIVER
The Windows backend behind TextInjector, files changed, tests + results, the live
validation evidence (native insert + secure refusal + windowed dev), the CI run URL, the
ADR/dep notes, and an honest list of anything that couldn't close.
```

---

## Notes for the operator (you)

- **Coordinate the lane:** the Linux lane (AT-SPI detect + uinput type) and the shared
  core are on `main`. This Windows prompt owns only the Windows backend behind the same
  trait — it won't collide with the Linux work.
- **Windows is the easy platform for injection:** UIA native insert works (Linux's didn't),
  and SendInput with `KEYEVENTF_UNICODE` handles full Unicode (the Linux uinput keymap is
  ASCII-only for now). So expect a cleaner "it types" result than Linux gave.
- **This box also closes two things CI can't:** the windowed `cargo tauri dev` check
  (P0-G5 Windows leg) and hands-on injection validation — do both while you're here.
