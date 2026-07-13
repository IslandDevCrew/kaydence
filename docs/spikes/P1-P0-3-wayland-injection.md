# Spike — Linux text injection (Wayland-first), P1-P0-3

**Status:** findings recorded 2026-07-08 · feeds ADR-0013 · code: `apps/desktop/src-tauri/src/inject/`
**Question:** How does Kaydence inject finalized text into the focused app on Linux —
across the compositor families real users run — without a cloud, without root by default,
and without weakening the secure-field refusal (non-negotiable #8) or the no-transmission
posture (#1, ADR-0006)? Wayland is the single highest platform risk in P1 (ROADMAP), so
it is spiked first.

This is a **feasibility + strategy** spike. It fixes the injection *abstraction* and the
*decision* now; the compositor-specific syscalls are implemented against the reference
machines once ADR-0013's dependency set is approved.

---

## 1. Why Wayland is hard (and X11 is not)

X11 has a global input-synthesis API (`XTEST`) any client can call — trivial to inject,
which is also why it is a security liability. **Wayland deliberately removed this**: clients
are isolated and cannot synthesize input into other clients by design. So "type text into
whatever app is focused" is not one API on Wayland — it is a *matrix* that depends on the
compositor and on whether the target app exposes accessibility. There is no single call
that works everywhere, and pretending otherwise is how tools ship silent failures.

## 2. Compositor matrix (what each actually exposes)

| Compositor family | Examples | Keystroke synth path | Native text insert | Notes |
|---|---|---|---|---|
| **wlroots** | Sway, Hyprland, river, Wayfire | `zwp_virtual_keyboard_v1` (+ `zwp_input_method_v2`) | via AT-SPI if app exposes it | Most permissive; a client may emit keys directly. |
| **GNOME / Mutter** | GNOME 43 (Debian 12 default), Ubuntu | **Not** virtual-keyboard. Use `xdg-desktop-portal` **RemoteDesktop** + **libei** (permission-gated) | via AT-SPI | The common case; forces the portal path. Debian-12 VM = this lane. |
| **KDE / KWin** | Plasma 5.27+ | RemoteDesktop portal (+ `fake_input`/virtual-keyboard on some builds) | via AT-SPI | Portal path is the portable choice. |
| **Any (below Wayland)** | all | `/dev/uinput` (evdev), as `ydotool` does | n/a | Compositor-agnostic but needs device permission (udev rule / `input` group); keycode-level, not text. |
| **XWayland / X11 sessions** | legacy apps, X11 login | `XTEST` | via AT-SPI | The existing X11 backend covers this. |

**Key finding:** the Debian 12 GNOME reference VM is the *hard, representative* lane — it
rejects virtual-keyboard and requires the **RemoteDesktop portal + libei** path, which is
exactly the path most desktop-Linux users need. Validating there is worth more than a
permissive wlroots pass.

## 3. Chosen strategy — a runtime capability ladder

Detect capabilities once at startup, then per injection pick the highest-trust path
available (implemented as the pure `select_plan` fn, already tested):

1. **Native text insertion via AT-SPI2 `EditableText`** — no synthetic keys, text-level,
   cross-desktop, and it is also our secure-field signal (see §4). Preferred whenever the
   focused app is AT-SPI-exposing. → `InjectMethod::Native`.
2. **Keystroke synthesis**, by compositor:
   - wlroots → `zwp_virtual_keyboard_v1`.
   - GNOME/KDE → **RemoteDesktop portal + libei** (user approves once; aligns with #1 — an
     explicit, revocable OS permission, no hidden capability). → `InjectMethod::Keystroke`.
3. **`/dev/uinput`** — last-resort, compositor-agnostic, gated behind an explicit opt-in +
   documented udev setup. Never enabled silently. → `InjectMethod::Keystroke`.
4. **Clipboard set + paste + restore** — layered on whichever keystroke channel exists
   (there is no "paste" on Wayland without a key path — a real constraint the selector
   encodes). Snapshot is always restored (Pitfall P2). → `InjectMethod::ClipboardRestore`.

X11 sessions collapse to `XTEST` + AT-SPI + clipboard.

## 4. Secure-field detection — the honest limitation

On macOS (AX secure-text / secure event input) and Windows (UIA `IsPassword`) the OS tells
us a field is secure. **Wayland has no global "is this a password box" signal** — client
isolation is the whole point. Our only secure-field signal on Linux is **AT-SPI**
(`STATE_EDITABLE` + role `PASSWORD_TEXT`). Therefore:

- App exposes AT-SPI → we read the role and **refuse on secure** (non-negotiable #8 holds).
- App is **opaque** (no AT-SPI, e.g. some Electron/games/remote clients) → we **cannot know**.
  This is a genuine platform gap, not something to paper over. The policy (tested in
  `decide_secure`) makes it explicit:
  - **Lenient (default):** inject, but mark the result `verified: false` so the HUD warns
    once per app that secure-field protection could not be confirmed there.
  - **Strict (opt-in):** treat an opaque client as possibly-secure and **refuse**.

Surfacing this to the user beats a false guarantee. It is called out in ADR-0013 as a
first-class product decision, and board 07 (Privacy & Context) should show the per-app
"secure-field detection: verified / unverified" state.

## 5. What is testable where (and what needs the VM)

| Layer | Verified on macOS host + CI | Needs the Debian-12 GNOME VM | Needs a Windows box |
|---|---|---|---|
| Policy / method-selection / clipboard-fallback logic | ✅ done (13 unit tests) | — | — |
| AT-SPI native insert + secure-field read | — | ✅ | — |
| RemoteDesktop portal + libei keystrokes (GNOME) | — | ✅ (the core spike proof) | — |
| `zwp_virtual_keyboard_v1` | — | needs a wlroots session (Sway) | — |
| X11 `XTEST` | — | ✅ (X11 session in the VM) | — |
| macOS AX insert / secure-field | partial (build) | — | — |
| Windows UIA / SendInput | — | — | ✅ |

The policy, selection, and clipboard-restore logic — the parts most likely to hide a bug —
are already implemented and unit-tested on the host. The remaining work is protocol
plumbing, validated on the machine that actually runs each compositor.

## 6. Dependencies this requires (ADR-0013 gate — not yet added)

Each is a new dependency **and** a new OS-input surface → audit-network allowlist + ADR +
operator go before adding (mission policy):

- `atspi` (AT-SPI2) — native insert + secure-field detection.
- `ashpd` (xdg-desktop-portal) + `reis`/libei — RemoteDesktop keystrokes on GNOME/KDE.
- `wayland-client` + `wayland-protocols` — `zwp_virtual_keyboard_v1` for wlroots.
- `x11rb` (or `xcb`) — X11 `XTEST` + `_NET_ACTIVE_WINDOW`.
- clipboard: `wl-clipboard-rs` / `arboard`.
- (opt-in only) a thin `/dev/uinput` writer — no crate needed; documented udev setup.

None are network crates; all must keep `audit-network.sh` green (no egress).

## 7. Validation plan on the reference VM (once it is up)

1. Operator: start the UTM Debian 12 VM; `sudo apt install -y openssh-server && sudo systemctl enable --now ssh`; confirm `loginctl show-session` reports `Type=wayland` under GNOME.
2. From the host: `ssh debian@<ip>`, install the Linux build deps (webkit2gtk-4.1, gtk-3, at-spi2, libei), clone/pull the repo, `cargo tauri dev`.
3. Prove, in order: (a) AT-SPI native insert into gedit/GNOME Text Editor; (b) secure-field refusal against a GTK password entry; (c) RemoteDesktop-portal keystrokes into an opaque app (approve the portal dialog once); (d) clipboard snapshot-restore round-trip.
4. Record evidence (screen capture + logs) under `ops/mission/evidence/` and close the Linux leg of the P1-G4 / injection gates. Repeat (a)/(c) under a Sway session for the wlroots path if available.

## 7b. Live findings from the Debian-12 GNOME VM (2026-07-08)

Validated `bin/atspi-selftest` against the reference VM. Three concrete results that
**revise the strategy**:

1. **AT-SPI connects + reads reliably on GNOME.** Connect → `AccessibleProxy` → tree
   traversal → `get_role`/`get_state`/`get_text`/`character_count` all work. Enumerated
   the desktop, found `gnome-text-editor`, walked to its `role=Text, editable=true`
   field. Detection works.
2. **AT-SPI `EditableText.InsertText` is UNRELIABLE on modern GNOME.** On both
   gnome-text-editor (GTK4) *and* gedit 44, `InsertText` returns `true` but the text
   never lands (readback stays empty). GTK4's newer accessibility backend treats these
   as effectively read-only; GTK's AT-SPI insertion is not a dependable inject path.
   → **AT-SPI is our detection + secure-field signal, NOT our primary insertion path.**
3. **uinput keystroke synthesis is the working inject path.** `ydotool` runs daemonless
   against `/dev/uinput` (exit 0). Keystroke synthesis below the compositor is
   compositor-agnostic and actually delivers text on GNOME/Wayland.

**Revised ladder (evidence-based):**
- **Detection / secure-field gate:** AT-SPI (`role`/`state`) — validated, keep.
- **Insertion:** keystroke synthesis — **uinput** (compositor-agnostic; opt-in device
  permission) or the **RemoteDesktop portal + libei** (permission-gated, no raw device
  access). AT-SPI native insert is a best-effort *bonus* only where an app honors it.
- **Focus:** a script-launched window does not reliably get keyboard focus on Wayland
  (same isolation), so the final "type into the focused app" check is operator-in-the-loop
  by nature: the operator focuses the target, the injector types, AT-SPI reads it back.
  **Validated 2026-07-11:** real GNOME Text focus typed `[Kaydence]` through uinput;
  real Zenity `PasswordText` focus was refused twice and stayed empty. See
  `ops/mission/evidence/2026-07-11-linux-human-focus-injection.txt`.

## 8. Recommendation

Adopt the capability ladder (§3) with AT-SPI-based secure-field detection and the honest
Unknown-focus policy (§4). Portal + libei is the strategic GNOME/KDE path; virtual-keyboard
for wlroots; uinput opt-in only. Proceed to ADR-0013 for the dependency approval, then
implement + validate on the VM. **Do not** ship a path that silently types into a field it
cannot confirm is non-secure.
