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

**Decision 3 — status bar grid-aligned to the rail boundary.** Jon: the "All systems operational"
indicator should land exactly where the nav rail's vertical edge meets the workspace (the
90° intersection continuing down into the footer), with the mic-level meter pushed toward that
same boundary (more separated from "Kaydence Free", ending up adjacent to "All systems
operational" instead). Implemented structurally, not by hand-tuned margins: the footer is now a
CSS Grid mirroring `.app`'s `236px 1fr` columns — a `.sb-left` cell (Kaydence Free + meter,
meter pushed to the cell's far edge via `margin-left:auto`) sized to exactly match the rail
width, and a `.sb-right` cell (All systems operational, then the pills + version pushed further
right as already set) starting flush at the same x-coordinate as the content area. `.sb-left`
carries a `border-right` that continues the rail's own divider line straight down, making the
alignment visually explicit rather than just coincidental. This holds correctly on resize
(grid-based, not pixel-guessed) and was applied identically to all three layout candidates —
verified structurally on each (braces/divs balanced, reserved tokens intact, `.sb-left`/`.sb-right`
present exactly once). Candidate 3's fix is scoped to `#screen-cockpit` specifically since that
file's `.statusbar` class is shared with its (unchanged, per K3) Privacy screen footer.

**Decision 4 (final polish, operator-approved as "perfect" otherwise) — version moves under the
brand.** The `v0.9.4 · build 2317 · macOS` string no longer lives in the status bar's right
cluster; it now sits as a second, smaller line directly under "Kaydence Free" in `.sb-left`, via
a small `.sb-brand-block` (flex column). This freed the status bar's right side to hold only the
interactive content (pills + the boundary-aligned "All systems operational"). Applied identically
to all three candidates; candidate 3's Privacy-screen footer (its own separate, unrelated
`sb-ver`) was left untouched. **This closes the Cockpit v2 punch list — operator confirmed
"perfect" pending this one change.**

**Decision 5 (superseding decision 4's stacking) — brand and meter share the top row; version
gets its own unwrapped line.** Correction to decision 4: rather than stacking `Kaydence Free`
directly over the version string, `.sb-left` is now a column with two rows — row 1
(`.sb-left-top`) holds `Kaydence Free` + the mic-level meter side by side (meter still pushed to
the row's far end via `margin-left:auto`, keeping it near the rail boundary per decision 2); row 2
is the version/build/OS string alone, `white-space:nowrap` so it never wraps. Applied identically
to all three candidates; candidate 3's Privacy-screen footer keeps its own unrelated `.sb-ver`
(now just picking up the shared `white-space:nowrap` addition, harmlessly). The now-unused
`.sb-brand-block` rule was removed from all three.

**Decision 6 (FINAL status-bar edit, operator confirmed) — mic-level meter moves to center,
genuinely equidistant.** The meter leaves `.sb-left` entirely (Kaydence Free + version now stand
alone there, stacked) and moves into `.sb-right`, positioned between "All systems operational"
and the status pills. Centered by giving `.meter` `margin-left:auto` AND `margin-right:auto`
(removing the `margin-left:auto` that used to live on `.sb-pills`/`.sb-pillgroup`) — CSS flexbox
splits all remaining free space equally between an element's two auto margins, so the meter lands
truly equidistant from "All systems operational" and "Engine," not eyeballed. Applied identically
to all three layout candidates, verified structurally (braces/divs balanced, reserved tokens
intact, exactly one `.meter` per file, confirmed relocated out of `.sb-left-top`).

**Operator: "that would be the final edit to that status bar that I will make on today."
Cockpit v2 status bar is CLOSED. Proceeding to DictateView rebuild -> D5 -> P1-G4 signoff.**

**Decision 7 (post-signoff, pre-merge polish) — real logo replaces the placeholder mark; real
Cleanup/Voiceprint icons replace the candidates' own; compact rail shows logo+"Kaydence" as a row;
redundant Dictate titlebar chrome removed.** After P1-G4 signoff, operator reviewed localhost:1420
side-by-side against the published Candidate 2 artifact (two screenshots) and flagged four real-app
issues plus one candidate-mockup-fidelity issue:

1. **Candidates' placeholder inline-SVG brand mark → real logo asset.** All three gauntlet-round-2
   mockups used a generic inline-SVG waveform glyph as a stand-in brand mark. Cropped the real
   `assets/brand/logos/kaydence-logo-option-1.png` (discovered via inspection to be a full
   icon+wordmark+background lockup, not an isolated icon — isolated the icon programmatically via
   PIL dark-pixel bounding-box detection on the rounded-square mark, verified clean via visual
   read) down to a 120x120 icon-only PNG, base64-embedded, and swapped in for every in-app nav-rail
   brand-mark instance: candidate 1 (1 instance), candidate 2 (1 instance), candidate 3 (2
   instances — it embeds both a Cockpit and a Privacy nav rail). Each candidate's own separate
   outer meta-heading label (e.g. "Cockpit v2 · Candidate 2," a gauntlet-process artifact, not part
   of the designed app UI) intentionally kept its original placeholder mark — that heading isn't
   something Kaydence itself renders.
2. **Real Cleanup/Voiceprint icon paths pulled into all three candidates (operator preference:
   "I like the cleanup logo better... the voice print logo from local host more").** Extracted the
   real wand+sparkles path (Cleanup) and vertical-bars waveform path (Voiceprint) from
   `CockpitChrome.tsx`'s `glyphContents`, substituted into all three candidate files. Relay kept
   unchanged (operator preference: "I like the relay logo in candidate two better"). The
   candidates' own original Cleanup icon (now freed up) was reused as the new Setup icon, per
   operator's own reasoning ("if I'm using the cleanup logo from local host, the cleanup logo in
   candidate two actually would be a better setup icon"). Sequenced as Cleanup-swap, then
   Voiceprint-swap, then Setup-reuse (in that order) to avoid a double-transform hazard, verified
   by path-string grep counts in all three files afterward.
3. **Real app: compact nav rail now shows logo + "Kaydence" as a row, not logo-alone-centered.**
   `NavRail.tsx` already rendered the app-name text in the DOM; only the CSS was hiding it in
   compact mode. `.compact-sidebar .brand-lockup` in `DictateView.css`/`CleanupView.css`/
   `PrivacyView.css` changed from a single-column, centered, 50px-logo/text-hidden layout to a
   26px-logo + ellipsis-truncated-name row, matching Candidate 2's rail treatment and the existing
   `.compact-sidebar .nav-item-label` pattern already used for icon+label rows elsewhere in the
   same rail.
4. **Real app: Dictate's redundant titlebar chrome removed.** The titlebar's second logo+"Kaydence"
   block (duplicating the rail's own brand lockup) and the macOS/Windows/Linux lane-switcher
   buttons (operator: "doesn't need to show all three, only the one that is loaded... the bottom
   right version/OS string is sufficient") are gone from `DictateView.tsx`'s titlebar, replaced by
   a plain "Dictate" screen-name label. The version string in the status footer now includes a
   real runtime-derived OS label (`v0.1.0 · macOS`, via a new `osLabel` prop sourced from the
   existing `detectOsLane()` detection) instead of the bare version.
   **Scope-check finding (worth recording):** `activeLane`/`setActiveLane`/`onLaneChange` were
   *not* dead code once removed from Dictate — Cleanup, Privacy, and Setup (FirstRunView) each have
   their own independent, real OS-lane switcher UI (a genuine reference-lane preview feature, for
   inspecting another OS's injection method/permissions without actually running that OS). Only
   Dictate's own copy of this switcher was removed; the shared `activeLane` state, and the other
   three screens' switchers, were left intact and untouched.

Verified independently against the real gate (`bash scripts/check-frontend.sh` — PASS, run directly,
not just trusted from the workflow's own report) and via direct diff review of every changed file
(`App.tsx`, `DictateView.tsx`, `DictateView.css`, `CleanupView.css`, `PrivacyView.css`) against the
operator's actual four asks before proceeding. Landed as commit `8a34fae`.

**Operator: "solid I lock that in and then let's push this as a PR." Proceeding to branch push,
PR open, and merge.**

## 2026-08-10 D5 amendment and responsive closeout

The D5 capture gap and its one responsive finding are closed by the hashed
Playwright evidence set in
`ops/mission/evidence/2026-08-10-d5-playwright-captures/`. The approved
900x600 footer remains 17.80px / 17.78px equidistant. Below 760px, status and
meter occupy a deliberate first row and the five pills a bounded second row;
the meter docks right instead of losing an anchor. Decision S1 is also enforced
in the real app: idle is a silent flat line and waveform bars are recording-only.


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
