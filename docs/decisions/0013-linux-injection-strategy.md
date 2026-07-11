# ADR-0013: Linux text injection strategy (Wayland + X11)

- **Status:** Accepted <!-- Operator approved the capability ladder + dependency set 2026-07-08 ("GO" into the backend build). Removed from the check-adr-status.sh exception list on acceptance. Backends implemented incrementally, each dep with its own audit-network review; validated on the reference machines (Linux = the UTM Debian 12 GNOME VM). -->
- **Date:** 2026-07-08
- **PRD items affected:** P0-3 (universal injection) on the Linux lane; supports P1-G4
  (screen 03 Injection) and the injection portions of every later phase

## Context

Injection is the highest platform risk in P1 (ROADMAP), and Linux is the hardest lane.
Unlike X11 (global `XTEST`), Wayland deliberately gives clients no way to synthesize input
into other clients — so "inject text into the focused app" is a compositor-dependent matrix,
not one API. The spike `docs/spikes/P1-P0-3-wayland-injection.md` establishes the matrix and
a strategy. This ADR ratifies the strategy and the dependencies it needs. Adding new OS-input
surfaces + new crates is a critical decision path (mission policy), so it halts for operator go.

The platform-agnostic core (secure-field policy, method selection, clipboard fallback,
outcome→event mapping) is already implemented and unit-tested on the host
(`apps/desktop/src-tauri/src/inject/mod.rs`, 13 tests). This ADR governs the *platform*
plumbing that fills in behind the `TextInjector` trait.

## Decision

**APPROVED 2026-07-08.** Adopt a runtime **capability ladder** on Linux, choosing the
highest-trust available path per injection. Backends land incrementally against the
Debian-12 GNOME reference VM; each new dependency gets its own audit-network review at
add time. Ladder:

1. **AT-SPI2 `EditableText`** native insertion when the focused app exposes it (also our
   secure-field signal).
2. **Keystroke synthesis** by compositor: `zwp_virtual_keyboard_v1` on wlroots;
   **`xdg-desktop-portal` RemoteDesktop + libei** on GNOME/KDE (permission-gated, revocable).
3. **`/dev/uinput`** as an explicit opt-in last resort (documented udev setup; never silent).
4. **Clipboard set + paste + restore**, layered on an available keystroke channel, snapshot
   always restored.

**Secure-field detection (non-negotiable #8).** Where AT-SPI exposes the field, refuse on a
secure/password role. Where the client is opaque (no AT-SPI), secure status **cannot be
determined** — default `Lenient` (inject but flag `verified: false` so the UI warns per app);
`Strict` mode (opt-in) refuses opaque focus. This limitation is disclosed to the user, not
hidden.

**Dependencies to add on approval** (each: audit-network allowlist entry, keeps `audit-network.sh`
green — none are network crates):
`atspi`, `ashpd` + `reis`/libei, `wayland-client` + `wayland-protocols`, `x11rb`,
`wl-clipboard-rs`/`arboard`. macOS (AX/CGEvent) and Windows (UIA/SendInput) backends fill the
same trait and are governed here for consistency but use OS frameworks, not new crates.

**Validation.** Backends are proven on the reference machines before their gate closes —
GNOME + X11 on the UTM Debian 12 VM (portal/libei is the core proof), wlroots on a Sway
session if available, macOS on the host, Windows via a real box. Evidence under `ops/mission/evidence/`.

## Alternatives considered

- **X11-only (ignore Wayland).** Rejected: Wayland is the default on modern GNOME/KDE and is
  first-class per ADR-0011; X11-only would silently fail for most new Linux desktops.
- **`ydotool`/uinput as the primary path everywhere.** Rejected as default: needs elevated
  device permission, bypasses the compositor's consent model, and is keycode—not text—level.
  Kept only as an explicit opt-in fallback.
- **Assume a global secure-field API exists on Wayland.** Rejected: it does not. Pretending
  otherwise would risk typing into an undetected password field — a silent #8 violation.
- **Refuse all opaque (non-AT-SPI) focus.** Rejected as default: it would break injection into
  many real apps. Offered as `Strict` mode instead, with the default disclosing the gap.

## Amendment 2026-07-08 (validated on the GNOME VM)

Live testing (`bin/atspi-selftest`) refined the ladder with evidence:

- **AT-SPI is DETECTION, not the primary INSERTION path.** `EditableText.InsertText`
  returns success but no-ops on modern GNOME apps (GTK4 gnome-text-editor *and* gedit 44).
  So AT-SPI is used for the secure-field gate + focused-role reads (both validated), and
  **keystroke synthesis is the primary insertion path**: uinput (validated available via
  ydotool, daemonless) or the RemoteDesktop portal + libei. AT-SPI native insert stays as
  a best-effort bonus for apps that honor it.
- This does **not** weaken non-negotiable #8: detection (the refusal signal) is exactly
  the part of AT-SPI that works. Where detection is unavailable (opaque client), the
  Unknown-focus policy still governs (Lenient flags unverified / Strict refuses).
- The end-to-end "types into the focused app" validation is operator-in-the-loop (Wayland
  won't let a script move keyboard focus — the same isolation the whole ADR is about).

## Amendment 2026-07-09 (Windows backend landed + validated live)

The Windows backend (`inject/windows.rs`) fills the same `TextInjector` trait, governed by
this ADR for consistency. It is the **inverse of the Linux ladder** — native-primary —
because on Windows the native path actually works:

- **UI Automation `IsPassword`** is the secure-field signal (non-negotiable #8). The
  UIA-facts→`FieldKind` mapping is pure + unit-tested; an *unreadable* `IsPassword` bit
  **fails closed** (treated as Secure). Live: a WinForms password field returned
  `Held{SecureField}` and nothing was entered (twice).
- **UIA `ValuePattern.SetValue` is the PRIMARY insertion path** (caret/selection-aware via
  `TextPattern` when present, else append). Live PASS into Notepad (`method: Native`).
- **`SendInput` `KEYEVENTF_UNICODE`** is the keystroke fallback — full Unicode,
  layout-independent. Live PASS into a classic Win32 Edit. Documented quirk: the *new*
  WinUI/RichEdit Notepad coalesces rapid synthetic key events, so the pure keystroke rung
  drops repeated chars there — but Notepad gets the native path anyway; logged for the
  `inject/AGENTS.md` app-compat matrix, not a backend defect.
- **Clipboard fallback** rides on a `SendInput` Ctrl+V through the shared `clipboard_paste()`
  helper: snapshot `CF_UNICODETEXT` → set → paste → **restore always** (≤200 ms; Pitfall P2).
  It refuses when the clipboard holds non-text it cannot restore (text-only MVP snapshot).
  Live PASS: injected text landed and a pre-seeded user clipboard was restored intact.

**Dependency:** the `windows` crate (Microsoft official), `cfg(target_os = "windows")` only,
version 0.61 — **already in `Cargo.lock` via tauri**, no new version. Minimal features
(Foundation, Com, DataExchange, Memory, Threading, UI Accessibility / Input / WindowsAndMessaging).
Local OS input/COM/clipboard APIs — **not a network surface**; `audit-network.sh` stays green,
no `network-allowlist.json` entry needed. No elevation; degrades to keystroke/clipboard if UIA
is unavailable (opaque focus → the Unknown-focus policy governs). Evidence:
`ops/mission/evidence/2026-07-09-windows-injection.txt` (+ native/secure/clipboard screenshots).

`GetForegroundWindow` + `QueryFullProcessImageNameW` (executable name) provides the
frontmost-app identity for `profiles/` — the blessed shared helper, per `inject/AGENTS.md`.

## Amendment 2026-07-11 (Linux human-focus path validated live)

The operator-in-the-loop boundary is now closed on the Debian 12 GNOME/Wayland VM:

- With GNOME's Run a Command entry actually focused, AT-SPI reported
  `role=Text editable=true`; the integrated uinput backend typed `[Kaydence]` and
  the marker was visible in the real field.
- With a real Zenity password entry focused, AT-SPI reported
  `role=PasswordText editable=true`; Kaydence classified it as `Secure`, refused
  twice across the focus events, and the field remained empty.
- No password value was entered or submitted. The prompt was cancelled after the
  proof. Full UTM frames and exact harness logs are preserved at
  `ops/mission/evidence/2026-07-11-linux-human-focus-injection.txt`.

This proves the selected AT-SPI gate + uinput insertion ladder on an accessible
client. It does not erase the opaque-client limitation or finish Unicode beyond
ASCII / optional portal+libei work.

## Consequences

- **Easier:** one `TextInjector` trait with the tested policy/selection/fallback core already
  in place; each compositor path drops in behind it; the injection method is recorded on the
  wire (`InjectMethod`) for analytics/debugging.
- **Harder:** the portal + libei path is real integration work and requires a user-consent
  dialog flow; multiple Linux code paths to maintain; the opaque-focus secure-field gap needs
  clear UI (board 07).
- **Maintained:** `scripts/check-adr-status.sh` carries 0013 as a by-design Proposed exception
  until approved; on approval it flips to Accepted and is removed from the exception list. Each
  dependency addition is its own audit-network + review step at implementation time.
