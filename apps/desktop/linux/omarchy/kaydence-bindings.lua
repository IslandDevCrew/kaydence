-- Kaydence dictation keys for Omarchy 4 / Hyprland (Lua config) — ADR-0023.
--
-- Wayland lets no app grab a global key, so the compositor runs
-- `kaydence-ctl record …` (a ~2 ms client with no GUI libraries), which drives
-- the running app over its private, same-user control socket
-- ($XDG_RUNTIME_DIR/kaydence/control.sock).
--
-- Install: paste into ~/.config/hypr/bindings.lua (Hyprland reloads on save),
-- then validate with `hyprctl reload && hyprctl configerrors`.
--
-- Both keys were free in Omarchy 4.0.4's defaults. Omarchy's own voxtype
-- binding (F9 / SUPER+CTRL+X) is left untouched, so the two can coexist.
-- The descriptions start with "Kaydence:" — the app uses them to confirm the
-- binding exists (Lua binds hide their command from `hyprctl binds`).

if o.cmd_present("kaydence-ctl") then
  -- Hold F10 to dictate; release to finish (same shape as Omarchy's F9 voxtype
  -- bind: no modifiers, so the release always matches).
  o.bind("F10", "Kaydence: start dictation (push-to-talk)", "kaydence-ctl record start")
  o.bind("F10", "Kaydence: stop dictation (push-to-talk)", "kaydence-ctl record stop", { release = true })

  -- One-key toggle alternative (press to start, press again to finish).
  o.bind("SUPER + ALT + D", "Kaydence: toggle dictation", "kaydence-ctl record toggle")
end
