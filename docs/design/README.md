# Kaydence Design Reference — Visual System v1 (locked 2026-07-08)

The vendored visual pack under [`assets/brand/`](../../assets/brand/) is the **canonical
UI/UX build reference** for every screen from P1 onward. It was generated (Codex) from
Plan of Attack v3, the Whisper-Ahead v2 interaction, and this repo's PRD / ARCHITECTURE /
ROADMAP, then locked by the operator. Source of truth for the rules: `designDirection`
in [`ops/mission/state.json`](../../ops/mission/state.json); full generation report at
`/Users/IDC2.5/Documents/Kaydence/docs/generated/Kaydence-Visual-Generation-Report-v1-2026-07-08.html`.

**Provenance:** vendored byte-identical (shasum-verified) from
`/Users/IDC2.5/Documents/Kaydence/assets/generated/kaydence-visual-pack/` on 2026-07-08
(mission task P1-D1). Do not edit the PNGs — regenerate upstream and re-vendor.

## The one rule

Build each screen to its family board. One interface system across all three desktop
apps; the **only** free-tier per-OS variation is the selection accent — **macOS aqua,
Windows cobalt, Linux emerald** — plus OS-specific permission/injection copy. The shared
invariants (left-nav order, 8px card radius, prediction-blue, ~0.5s amber-gold merge
pulse, destructive confirm gate, no cloud-sync copy on core screens, keyboard/a11y)
never vary; changing one is a critical decision path → operator go + ADR.

## Logo options -> ADR-0012 (accepted, task P1-D2)

ADR-0012 is accepted. The operator selected a dual-asset identity:
Option 1 is the sole product mark for app icon, tray/menu bar, installer,
HUD badge, and every isolated-logo use. Option 4 is the Kaydence wordmark
for public-site/docs/marketing headers only. The Option 4 standalone icon is
retired, and its serif display style must not appear inside the product UI.

| # | File | Direction |
|---|------|-----------|
| 1 | [`kaydence-logo-option-1.png`](../../assets/brand/logos/kaydence-logo-option-1.png) | **Selected mark.** K Waveform — K-shaped voice/caret mark, teal/blue; everyday app-icon direction |
| 2 | [`kaydence-logo-option-2.png`](../../assets/brand/logos/kaydence-logo-option-2.png) | Protected Path — circular voice-path/seal for the privacy-first promise |
| 3 | [`kaydence-logo-option-3.png`](../../assets/brand/logos/kaydence-logo-option-3.png) | Relay Monogram — cross-device K monogram, three-desktop story |
| 4 | [`kaydence-logo-option-4.png`](../../assets/brand/logos/kaydence-logo-option-4.png) | **Selected wordmark.** Wordmark Forward — typography-led, public-site/docs/marketing header only |
| 5 | [`kaydence-logo-option-5.png`](../../assets/brand/logos/kaydence-logo-option-5.png) | Voice Constellation — three-node local-fleet mark (premium feel) |

Screens use the Option 1 mark in the reserved slot. The mark and wordmark are never
composed side-by-side as twin logos; each has its own role.

## Screen families → build phase → fidelity gate

Each board contains macOS, Windows, and Linux variants of the same product moment
(10 families × 3 OS = 30 views). The gate closes when the built screens are
design-reviewed against the board plus the keyboard/a11y checklist, with evidence
saved under `ops/mission/evidence/`.

| # | Family | Board | Built in | Fidelity gate |
|---|--------|-------|----------|---------------|
| 01 | Main Dictation Cockpit | [`kaydence-screen-family-01.png`](../../assets/brand/screens/kaydence-screen-family-01.png) | P1 | P1-G4 |
| 02 | Whisper-Ahead HUD | [`kaydence-screen-family-02.png`](../../assets/brand/screens/kaydence-screen-family-02.png) | P3 | P3-G4 |
| 03 | Cleanup & Injection | [`kaydence-screen-family-03.png`](../../assets/brand/screens/kaydence-screen-family-03.png) | P1 (inject/Raw) → P2 (dial) | P1-G4 → P2-G3 |
| 04 | Relay | [`kaydence-screen-family-04.png`](../../assets/brand/screens/kaydence-screen-family-04.png) | P4 | P4-G-SCREENS |
| 05 | Voiceprint | [`kaydence-screen-family-05.png`](../../assets/brand/screens/kaydence-screen-family-05.png) | P4 | P4-G-SCREENS |
| 06 | Conductor | [`kaydence-screen-family-06.png`](../../assets/brand/screens/kaydence-screen-family-06.png) | P4 | P4-G-SCREENS |
| 07 | Privacy & Context | [`kaydence-screen-family-07.png`](../../assets/brand/screens/kaydence-screen-family-07.png) | P1 (posture) → P3 (context) | P1-G4 → P3-G4 |
| 08 | Dictionary & Profiles | [`kaydence-screen-family-08.png`](../../assets/brand/screens/kaydence-screen-family-08.png) | P2 (dictionary) → P3 (profiles) | P2-G3 → P3-G4 |
| 09 | Analytics & Performance | [`kaydence-screen-family-09.png`](../../assets/brand/screens/kaydence-screen-family-09.png) | P3 | P3-G4 |
| 10 | First Run & License | [`kaydence-screen-family-10.png`](../../assets/brand/screens/kaydence-screen-family-10.png) | P1 | P1-G4 |

## Build prompts

Fable-safe, scoped prompts for building against these boards live in
[`prompts/VISUAL-BUILD-PROMPT-PACK.md`](../../prompts/VISUAL-BUILD-PROMPT-PACK.md):
session resume `/goal`, per-screen build, fidelity-gate review, and the logo-selection
→ ADR-0012 decision prep.
