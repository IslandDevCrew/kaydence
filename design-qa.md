# P1-G4 Design QA

## Board 01

- Source visual truth: `assets/brand/screens/kaydence-screen-family-01.png`
- Browser implementation: `ops/mission/evidence/2026-07-10-p1-g4-board01-browser-540x800.png`
- Native implementation: `ops/mission/evidence/2026-07-10-p1-g4-board01-native-macos.png`
- Combined comparison: `ops/mission/evidence/2026-07-10-p1-g4-board01-comparison.png`
- Viewport/state: macOS lane, Light cleanup, ready-state Browser fixture at 540 x 800; backend-owned native state at 900 x 600.

**Findings**

- No actionable P0/P1/P2 mismatch remains for Board 01 composition.
- Fonts and typography: system UI typography matches the source's compact desktop hierarchy. Native state labels remain legible and do not collide at either verified viewport.
- Spacing and layout rhythm: recording header, five-card status column, transcript/history center, cleanup/output rail, five-action nav, and two-line footer preserve the source proportions. Browser proof has no horizontal or vertical page overflow at 540 x 800 or 900 x 600.
- Colors and visual tokens: the light surface system, 8px cards, prediction blue, and status colors match the board. macOS aqua, Windows cobalt, and Linux emerald change selection accents and platform copy without changing layout.
- Image and icon fidelity: the selected Option 1 mark is present. Lucide line icons render in both Chromium and the native macOS WebView; no placeholder, emoji, or missing asset remains.
- Copy and content: the Browser fixture exercises the board's ready/recording state. The native app deliberately shows Blocked/Pending/Ready from backend truth rather than copying optimistic mock text.

**Comparison History**

1. Initial build: Board 01 regions were absent and the old page claimed a mock recording state. Replaced with the board composition and backend-derived native statuses.
2. Browser pass: the child min-content height pushed the nav/footer 97px below the 900 x 600 viewport. Fixed the fixed-window grid contract; post-fix scroll size equals viewport size.
3. Native pass: external SVG sprite fragments rendered in Chromium but disappeared in WebKit. Replaced them with the same Lucide paths in the component; post-fix native capture shows every status and navigation icon.
4. Refinement pass: waveform density, transcript fixture depth, target detail, microphone meter, and auto-paste toggle drifted from the source. Tightened all five and repeated the combined comparison.

**Implementation Checklist**

- [x] Locked Board 01 composition
- [x] Browser interactions and responsive proof
- [x] Signed native macOS rendering and interaction proof
- [x] Backend-owned native readiness states
- [x] Shared `StatusCard`, `SegmentedDial`, `NavRail`, `BottomNav`, and `StatusFooter` primitives
- [x] Dictate view and CSS extracted into files under 800 lines

**Follow-up Polish**

- P3: native titlebar tint follows the user's macOS appearance while the board captures a light titlebar.
- P3: exact source icon glyphs differ slightly from the selected Lucide set, without changing meaning or layout.

final result: passed

## Board 03

- Source visual truth: `assets/brand/screens/kaydence-screen-family-03.png`
- Browser implementation: `ops/mission/evidence/2026-07-10-p1-g4-board03-browser-540x800.jpg`
- Native implementation: `ops/mission/evidence/2026-07-10-p1-g4-board03-native-macos.png`
- Combined comparison: `ops/mission/evidence/2026-07-10-p1-g4-board03-comparison.png`
- Viewport/state: macOS lane, Light cleanup, ready-state Browser fixture at 540 x 800 and 900 x 600; backend-owned native state at 900 x 600.

**Findings**

- No actionable P0/P1/P2 mismatch remains for Board 03 composition.
- Typography and layout: the compact rail, cleanup dial, seven-rule stack, three before/after examples, per-app destination, injection evidence, fallback, health, latency, four gates, and save footer preserve the locked hierarchy. Browser page and panel scroll bounds equal their viewports; native WebKit shows every gate without an internal scrollbar.
- Colors and tokens: neutral light surfaces and 8px-or-smaller radii remain constant. macOS aqua, Windows cobalt, and Linux emerald are the only lane color variation.
- Brand and icons: the Option 1-derived native app icon replaces the board's illustrative K glyph. Existing cockpit/Lucide paths cover navigation and status meaning; no missing or invented asset remains.
- Copy and truth: Browser fixtures exercise enabled and delivered states. Native permission, target, delivery, and reference-p95 labels stay Setup required or Pending until backend evidence exists. Real latency budgets replace the board's illustrative values.
- Interaction: Browser and native Raw/Light/Full, reset/save, three OS lanes, and permission-navigation round trips pass. Per-rule and per-app profile editors stay disabled until their owning phases land.

**Comparison History**

1. RED baseline: the old generic Cleanup page was 885 x 1074 at a 900 x 600 viewport and lacked injection status, fallback, health, latency, gates, examples, and destination evidence.
2. Initial board build: extracted `CleanupView` and added the complete two-pane composition with backend-derived evidence.
3. Compact pass: fixed an inherited mobile rule that split the rail into two columns and removed right-pane horizontal scrolling at 540 x 800.
4. Fidelity pass: restored the source's single-column rules and vertical example cards, enlarged the brand anchor, and switched to the existing Option 1-derived app icon.
5. Native pass: WebKit's shorter content area hid Round Trip. Added a short-window density contract, rebuilt the signed bundle, and recaptured all four gates without scrolling.

**Implementation Checklist**

- [x] Locked Board 03 composition
- [x] Browser interactions at 540 x 800 and 900 x 600
- [x] Signed native macOS rendering and persisted dial proof
- [x] Backend-owned native permission, delivery, and latency states
- [x] macOS, Windows, and Linux capability copy and accents
- [x] Cleanup view and CSS extracted into files under 800 lines

**Follow-up Polish**

- P3: native titlebar tint follows the user's macOS appearance while the board captures a light titlebar.
- P3: real Windows and Linux WebView captures remain required before the parent P1-G4 gate can close.
- P3: board info affordances and profile editing become interactive only with their owning P2 units.

final result: passed (Board 03 scope only)
