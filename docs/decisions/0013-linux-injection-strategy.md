# ADR-0013: Linux text injection strategy (Wayland + X11)

- **Status:** Proposed <!-- OPERATOR DECISION PENDING: approve the capability ladder + the dependency set before the platform backends are implemented. Declared by-design exception in scripts/check-adr-status.sh until then, same mechanism as ADR-0009. -->
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

**PENDING OPERATOR APPROVAL.** Adopt a runtime **capability ladder** on Linux, choosing the
highest-trust available path per injection:

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
