# Kaydence Cockpit v2 — Operator Walkthrough Punch List (2026-08-09)

## Round 2 resolution (2026-08-09, evening) — READ THIS FIRST

**Decision 1 — the flank question is resolved by not resolving it.** Rather than pick one
winner, all three round-2 candidates ship as **selectable Cockpit layouts in Setup** (like an
IDE's layout presets): `Pill Bar`, `Mic Capsule`, `Stacked Panel`. Default = the mic-capsule
layout (candidate 2 — Jon: "definitely looks like candidate number two, which is kind of how I
laid it out"). L3's left/right ambiguity is moot once switching is free.

**Decision 2 — status-bar left cluster fixed.** Applied identically to all three candidates:
- **Left cluster** (brand-adjacent, ambient/non-interactive): `Kaydence Free` → mic-level meter
  (no flanking separator lines — just floats there) → `All systems operational` (green). Nothing
  selectable sits left of "All systems operational" except the brand and the meter.
- **Right cluster** (interactive, pushed via `margin-left:auto` on the pill group): the five
  status pills (Engine, Target App, Hotkey, Privacy, WAL Recovery) now cluster near the version
  string on the far right, not immediately after the brand.
- Verified structurally on all 3 files: braces balanced, `:root` token block byte-identical to
  the locked reference (this was chrome-reorder only, no new tokens), reserved `--blue`/`--gold`
  markers present.

Candidates archived: `ops/mission/evidence/2026-08-09-cockpit-v2-hud-winner.html` (mic-capsule,
default) plus the pill-bar and stacked-panel siblings in the same gauntlet-round2 output —
promote all three into the D4 rebuild as real layout options, not just the default.


Captured verbatim-in-intent from Jon's click-through of the locked artifact
(`2026-08-09-design-gauntlet-winner-locked-v1.html`). Grouped by type. The **L** and **H**
items are the "gauntlet the actual look" work he explicitly asked for; **P/S/W** fold into that
same rebuild; **K/C** need no change.

## K — Confirmed keeps (do NOT change)
- **K1 Output insert-mode toggle** (At cursor / Clipboard). Jon: "very nice… toggle what you'd prefer as the words are being spoken." Keep exactly.
- **K2 Whisper-Ahead** prediction + Tab-to-accept + timings — "properly showing… we have clean output in practice." Keep. (Still mock data; real model wiring is P3.)
- **K3 Privacy screen** — "I don't see anything there I would change… useful meaningful data, I like the layout." Keep as-is.
- **K4 Status content** — "I like the status and what it reflects." The *content* stays; only its *placement* moves (see L1).

## P — Palette fix (the deferred item, now scheduled)
- **P1** The Cool/Warm toggle has no effect in **dark mode** — `[data-palette="warm"]` only overrides the `--l-*` (light) neutrals. Add a warm-tinted **dark** neutral set so Warm reads distinctly in both themes (a warm-charcoal ground vs. the cool near-black), reserved `--blue`/`--gold`/`--accent` unchanged.

## S — State / animation fixes
- **S1 Idle capture is over-animated.** At "Ready / 00:00" the waveform should be a **silent flat line** (no flowing animation); keep only the mic **ready-pulse**. On click → mic turns **red**, label "Ready" → **"Recording"**, the **timer counts**, and only *then* does the waveform move. (The record→red+animate+timer path already exists; the fix is making the *idle* state calm.)

## W — Interactive wiring
- **W1 Cleanup tag must reflect the dial.** The transcript header shows "Light cleanup" only because Light is selected — but choosing **Raw** or **Full** doesn't update it. Wire it: Raw → "Raw", Light → "Light cleanup", Full → "Full cleanup" (Jon also said "or heavy cleanup" — naming TBD).
- **W2 Transcript scroll-back.** The Recent Clean Transcript should **scroll** as output accumulates, so the user can read back the full assembled transcription (design: auto-follow newest, with scroll-back to review). Confirm this behavior on the larger surface (see L4).

## L — Cockpit layout restructure (GAUNTLET THIS)
> Jon's stated goal, high confidence: **give the transcript the room; demote static chrome.**
> "The entire bottom half section can be used for the recent clean transcript… spread out through
> the full width… the entire bottom half can effectively be the interface."
- **L1 Demote Status to the bottom bar.** Engine, Target App, Global Hotkey, Privacy, WAL Recovery leave the left card column and live in the **bottom status bar** (beside "All systems operational") as **clickable arrow/popover controls** — selectable/changeable inline or deep-linking to Setup. Rationale: it's mostly static; reclaim the space.
- **L2 Capture → center.** The record control moves to the **middle** (top band), not the left column.
- **L3 Cleanup + History → flanks.** They become compact top flanks around centered Capture. ⚠️ **Ambiguity flag:** Jon's exact left/right assignment was tangled by voice-to-text ("cleanup all the way on the right where capture is coming from" vs. "history off to the right where cleanup is"). Best reading: **Cleanup and History flank the centered Capture, one per side** — the three gauntlet directions will each try a different arrangement so Jon picks the one matching his mental model.
- **L4 Transcript → full-width, bottom half, primary.** The Recent Clean Transcript spans the **full application width** across the **bottom ~half** and is the star surface — no longer a compacted center card.
- Net top→bottom: [ Cleanup · **Capture (center)** · History ] compact top band → **full-width Transcript** bottom half → **Status bar** with popovers.

## H — New mode: compact / background HUD (GAUNTLET THIS + competitor best-practices)
- **H1** The full-window layout is great on a **multi-monitor** setup (Cadence on its own screen while you work on another). On a **single screen** it takes too much space. Design the **compact / background / floating HUD** — what Cadence looks like running in the background: a small, non-intrusive overlay showing the **hotkey affordance** + live state (recording, timer, waveform, Whisper-Ahead prediction) so the working screen stays usable. Benchmark the **compact modes** of the peg apps (Wispr Flow floating bar, Windows Win+H bar, native macOS mic widget, Aqua streaming, Superwhisper mini recorder, Glaido hotkey overlay). Must be a **non-activating** overlay that never steals focus (pre-launch finding #7). Ties to Whisper-Ahead (family 02).

## C — Clarifications (NOT bugs)
- **C1** The left-nav items that "aren't live" (Whisper-Ahead, Relay, Voiceprint, Conductor, Dictionary, Analytics) are **intentionally disabled P2/P3/P4 placeholders** — they populate as those phases ship. Also: in the **artifact you clicked**, only the top-right **Cockpit/Privacy switch** is wired (a mockup affordance), which is why left-nav "Privacy" looked dead there. In the **real rebuilt app**, Dictate / Cleanup / Privacy / Setup are live via the shared NavRail.

## Build routing
- **Gauntlet round 2** designs L (restructured Cockpit) + H (compact HUD) from the **locked** Editorial Precision language (same `:root` tokens, same component grammar — layout changes only, not a new style), with **P1 / S1 / W1 / W2 folded into the shared base** so the winner demonstrates them.
- On operator pick → rebuild `DictateView` from the new composition (supersedes tonight's D4 cockpit), keep Privacy as-is (K3), then D5 capture + signoff.
