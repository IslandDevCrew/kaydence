# Kaydence on Omarchy (and the rest of Linux)

**Omarchy is the priority Linux platform** (operator decision, 2026-09-25):
Arch Linux + Hyprland, Lua-configured since Omarchy 4. Ubuntu and Debian remain
the packaging baselines. Design record: [ADR-0023](../decisions/0023-omarchy-hyprland-linux-lane.md)
(builds on ADR-0011 and ADR-0013).

## What you get on Omarchy / Hyprland

| Capability | How | Root needed |
|---|---|---|
| Typing into the focused app | `zwp_virtual_keyboard_v1` through Hyprland, with a per-injection keymap, so any Unicode text types exactly (emoji, CJK, RTL, accents) | No |
| Global hotkey | A Hyprland keybinding runs `kaydence-ctl record …` (Wayland lets no app grab keys) | No |
| Wrong-window protection (P9) | Hyprland IPC tells Kaydence which window had focus when you started speaking; if focus moved, the text is held, not typed | No |
| Password-field refusal (#8) | AT-SPI marks password fields; that verdict counts only when it comes from the window Hyprland says is focused | No |
| Per-app profiles | Hyprland window class = app identity | No |

Nothing here opens a network connection. Hyprland IPC, the Wayland socket, the
AT-SPI bus and Kaydence's own control socket are all local, per-user Unix sockets.

## Install

**From source (Arch/Omarchy package):**

```bash
git clone https://github.com/IslandDevCrew/kaydence
cd kaydence/apps/desktop/linux/arch
makepkg -si            # builds kaydence + kaydence-ctl, installs the Omarchy snippet
```

**Ubuntu / Debian:** the `.deb` (and the AppImage) produced by `tauri build`
install `kaydence`, `kaydence-ctl`, and the same Omarchy snippet under
`/usr/share/kaydence/omarchy/`.

## Bind the dictation keys (one time)

Append the shipped snippet to your Hyprland bindings:

```bash
cat /usr/share/kaydence/omarchy/kaydence-bindings.lua >> ~/.config/hypr/bindings.lua
hyprctl reload && hyprctl configerrors     # must print nothing
```

It binds:

- **Hold F10** — push-to-talk (release to finish). No modifiers, so the release
  always matches, which is the same shape as Omarchy's own F9 voxtype binding.
- **SUPER + ALT + D**: press to start, press again to finish.

Both keys were free in Omarchy 4.0.4 defaults. **voxtype keeps F9 / SUPER+CTRL+X**;
the two tools coexist. To make Kaydence your only dictation key, `hl.unbind("F9")`
first and bind F9 to `kaydence-ctl record start` / `stop` instead.

Why `kaydence-ctl` and not `kaydence`? Both accept `record …`, but the full app
links the GTK/WebKit stack (~145 shared libraries, ~74 ms to exec). `kaydence-ctl`
links three and answers in ~2 ms, which keeps the key press well inside the 50 ms
hotkey budget.

## Status bar and scripts

```bash
kaydence-ctl record status    # idle | recording | finalizing   (exit 2 = app not running)
kaydence-ctl record toggle
```

The control socket (`$XDG_RUNTIME_DIR/kaydence/control.sock`) is private to your
user (0700 dir, 0600 socket, peer-uid checked). It accepts one verb and answers
with one state word. It never carries audio or transcript text.

## Troubleshooting

```bash
# Is Hyprland visible to Kaydence, and can it create a virtual keyboard?
cargo run --manifest-path apps/desktop/src-tauri/Cargo.toml --bin wayland-selftest -- --probe

# Byte-exact Unicode typing proof into a throwaway foot window (needs an
# unlocked session; it moves focus to its own window and gives it back):
bash scripts/linux-wayland-inject-proof.sh
```

- **First-run says the hotkey is not registered.** Expected until a binding exists:
  Kaydence reads `hyprctl binds` and looks for a description starting with
  "Kaydence:" (Lua binds hide their command). Pressing the key once also marks
  it registered.
- **Nothing types while the screen is locked.** Correct: the lock screen holds
  keyboard focus, so no window is a valid target and the text is held in history.
- **Hyprland `ecosystem:enforce_permissions`**: if enabled, allow Kaydence's
  virtual keyboard when Hyprland asks; if denied, delivery reports "no keystroke
  channel" and your text stays in history.

## Other Linux desktops

| Desktop | Typing | Hotkey | Focus binding |
|---|---|---|---|
| Hyprland / Omarchy | virtual keyboard (rootless, Unicode) | keybinding → `kaydence-ctl` | Hyprland IPC |
| Sway / other wlroots | virtual keyboard (rootless, Unicode) | `bindsym` → `kaydence-ctl` (`--release` for push-to-talk) | not yet: text is held with "insert here" |
| GNOME / KDE (Wayland) | `/dev/uinput` only if you opted in (`scripts/linux-inject-demo.sh`), ASCII only | custom shortcut → `kaydence-ctl record toggle` | not available: held with "insert here" |
| X11 sessions | uinput opt-in | native grab (the Right-Alt default cannot be grabbed on Linux yet) | not yet |

The rootless GNOME/KDE path is the RemoteDesktop portal + libei rung of ADR-0013,
still open.
