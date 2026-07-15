# P1-G4 Design QA

## Hosted Windows Checkpoint

- Passing Windows 11 host set: `ops/mission/evidence/2026-07-14-windows-webview-proof/`
- Passing ledger: `ops/mission/evidence/2026-07-14-p1-g4-windows-host-proof.txt`
- Native diagnostic set: `ops/mission/evidence/windows-proof-run-29137950481/`
- Same-viewport combined comparison: `ops/mission/evidence/windows-proof-run-29137950481/browser-vs-native-comparison.png`
- Native provenance frames: `board{01,03,07,10}-native-windows-runner.png`
- Workflow record: `ops/mission/evidence/2026-07-11-p1-g4-windows-diagnostic.txt`
- Viewport/state: the source-reviewed Browser fixture and native Windows client are both 900 x 600 for each family; Browser uses the ready fixture, while native Windows preserves backend-owned pending/blocked states.

**Findings**

- Boards 01, 03, 07, and 10 preserve the reviewed composition, density, navigation, footer bounds, and control sizing in the native Windows WebView.
- Windows cobalt stays confined to lane selection and active controls. Shared prediction, warning, success, and neutral tokens do not drift into a one-note blue palette.
- No visible text overlap, clipped app content, broken icon, missing asset, or unstable layout appears in the combined comparison. Native titlebar frames separately establish Windows provenance.
- Differences from the Browser row are expected runtime truth, not visual regressions: the hosted app reports blocked model metadata, pending permissions, no local transcript/history, and setup-required delivery.
- The historical hosted set remains diagnostic: run 29137950481 failed closed before the permissions modal because its expected label differed from production. The second proof VM never exposed the initial WebView UIA tree. On 2026-07-14, the current `24957aa` shipping-feature release passed the same fail-closed harness on an interactive Windows 11 Pro x64 host and captured all four families plus the permissions modal. The five client captures, five native provenance frames, five UIA trees, source hashes, executable hash, and host manifest are retained in the passing set above.

**Windows Completion Checklist**

- [x] Four native Windows client captures produced and visually compared at 900 x 600
- [x] Native titlebar, UIA tree, source hash, host, and pixel diagnostics preserved
- [x] Exact permission labels fixed in PR #9 with a green 3-OS matrix
- [x] Startup retry hardening was consolidated through PR #27; proof PR #28 passed a fresh 3-OS matrix and merged as `71890fd`
- [x] One passing interactive Windows run captures all four families plus the permissions modal
- [ ] Final human visual signoff

checkpoint result: Windows evidence pass; parent gate remains pending only for final operator visual signoff

## Board 01

- Source visual truth: `assets/brand/screens/kaydence-screen-family-01.png`
- Browser implementation: `ops/mission/evidence/2026-07-10-p1-g4-board01-browser-540x800.png`
- Native implementation: `ops/mission/evidence/2026-07-10-p1-g4-board01-native-macos.png`
- Native Linux implementation: `ops/mission/evidence/2026-07-11-p1-g4-board01-native-linux.jpg`
- Combined comparison: `ops/mission/evidence/2026-07-10-p1-g4-board01-comparison.png`
- Viewport/state: macOS lane, Light cleanup, ready-state Browser fixture at 540 x 800; backend-owned native macOS and GNOME/Wayland Linux states at 900 x 600.

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
- [x] Real Debian 12 GNOME/Wayland WebView rendering proof
- [x] Backend-owned native readiness states
- [x] Shared `StatusCard`, `SegmentedDial`, `NavRail`, `BottomNav`, and `StatusFooter` primitives
- [x] Dictate view and CSS extracted into files under 800 lines

**Follow-up Polish**

- P3: native titlebar tint follows the user's macOS appearance while the board captures a light titlebar.
- P3: exact source icon glyphs differ slightly from the selected Lucide set, without changing meaning or layout.
- P1-G4: the Windows view has a diagnostic capture; a passing workflow capture, permissions modal, and final human signoff remain open.

final result: passed

## Board 03

- Source visual truth: `assets/brand/screens/kaydence-screen-family-03.png`
- Browser implementation: `ops/mission/evidence/2026-07-10-p1-g4-board03-browser-540x800.jpg`
- Native implementation: `ops/mission/evidence/2026-07-10-p1-g4-board03-native-macos.png`
- Native Linux implementation: `ops/mission/evidence/2026-07-11-p1-g4-board03-native-linux.jpg`
- Combined comparison: `ops/mission/evidence/2026-07-10-p1-g4-board03-comparison.png`
- Viewport/state: macOS lane, Light cleanup, ready-state Browser fixture at 540 x 800 and 900 x 600; backend-owned native macOS and GNOME/Wayland Linux states at 900 x 600.

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
- [x] Real Debian 12 GNOME/Wayland WebView rendering proof
- [x] Backend-owned native permission, delivery, and latency states
- [x] macOS, Windows, and Linux capability copy and accents
- [x] Cleanup view and CSS extracted into files under 800 lines

**Follow-up Polish**

- P3: native titlebar tint follows the user's macOS appearance while the board captures a light titlebar.
- P1-G4: the Windows view has a diagnostic capture; a passing workflow capture, permissions modal, and final human signoff remain required.
- P3: board info affordances and profile editing become interactive only with their owning P2 units.

final result: passed (Board 03 scope only)

## Board 07

- Source visual truth: `assets/brand/screens/kaydence-screen-family-07.png`
- Browser implementation: `ops/mission/evidence/2026-07-11-p1-g4-board07-browser-540x800.png`
- Native implementation: `ops/mission/evidence/2026-07-11-p1-g4-board07-native-macos.png`
- Native Linux implementation: `ops/mission/evidence/2026-07-11-p1-g4-board07-native-linux.jpg`
- Combined comparison: `ops/mission/evidence/2026-07-11-p1-g4-board07-comparison.png`
- Viewport/state: macOS Aqua lane and local preview fixture at 540 x 800 plus 900 x 600; backend-owned native macOS and GNOME/Wayland Linux states at 900 x 600.

**Findings**

- No actionable P0/P1/P2 mismatch remains for Board 07 composition.
- Typography and layout: the compact rail, Constitution strip, six-row Context Reader, permission rail, local audit, and status footer preserve the locked information hierarchy. Browser page and panel scroll bounds equal their viewports at both verified sizes.
- Colors and tokens: neutral light surfaces and 8px-or-smaller radii remain constant. macOS Aqua, Windows Cobalt, and Linux Emerald are the only lane color variation.
- Brand and icons: the selected Option 1-derived mark appears in the reserved positions. Existing cockpit/Lucide paths communicate privacy, persistence, model, and permission state without missing assets.
- Copy and truth: the implementation intentionally corrects illustrative source claims that are ahead of P1. It discloses 30-day local persistence, keeps context/OCR off, marks screen capture Not requested, renders only audit metadata, and labels non-host lanes as references.
- Interaction and accessibility: Constitution open/close, Escape close, autofocus, three OS lane switches, Setup handoff, and History navigation pass. Native accessibility exposes the dialog as modal and the controls with button/toggle roles.

**Comparison History**

1. RED baseline: the old route was a generic six-card posture list without the Board 07 regions or dedicated layout.
2. Initial board build: extracted the Privacy view and added Constitution, reader, permissions, audit, and footer regions.
3. Truth pass: replaced the board's no-persistence and active-context illustrations with current backend policy, 30-day retention, P3 opt-in boundaries, metadata-only audit copy, and conservative permission states.
4. Responsive pass: fixed the 540 x 800 compact rail and stable two-column content proportions; exact page and panel bounds now fit without overflow.
5. Native pass: proved the signed WebKit bundle, backend permission states, modal Constitution, all three lane vocabularies, and the real Screen Family 10 Setup handoff.

**Implementation Checklist**

- [x] Locked Board 07 composition
- [x] Browser interactions at 540 x 800 and 900 x 600
- [x] Signed native macOS rendering and interaction proof
- [x] Real Debian 12 GNOME/Wayland WebView rendering proof
- [x] Backend-owned native settings, permission, audit, and operational states
- [x] macOS, Windows, and Linux capability copy and accents
- [x] Privacy view and CSS extracted into files under 800 lines

**Follow-up Polish**

- P3: accessibility context and on-device OCR controls become interactive only when their owning backend capability lands.
- P1-G4: the Windows view has a diagnostic capture; a passing workflow capture, permissions modal, and final human signoff remain required.
- P3: native titlebar tint follows the user's macOS appearance while the board captures a light titlebar.

final result: passed (Board 07 scope only)

## Board 10

- Source visual truth: `assets/brand/screens/kaydence-screen-family-10.png`
- Browser implementation: `ops/mission/evidence/2026-07-11-p1-g4-board10-browser-540x800.jpg`
- Native implementation: `ops/mission/evidence/2026-07-11-p1-g4-board10-native-macos.jpg`
- Native Linux implementation: `ops/mission/evidence/2026-07-11-p1-g4-board10-native-linux.jpg`
- Native Linux permission dialog: `ops/mission/evidence/2026-07-11-p1-g4-board10-native-linux-permissions.jpg`
- Combined comparison: `ops/mission/evidence/2026-07-11-p1-g4-board10-comparison.png`
- Viewport/state: macOS Aqua ready-state Browser fixture at 540 x 800 and 900 x 600; backend-owned native macOS and GNOME/Wayland Linux states at 900 x 600.

**Findings**

- No actionable P0/P1/P2 mismatch remains for Board 10 composition.
- Typography and layout: the compact nine-step rail, progress line, next-step proof strip, six setup controls, four license/service choices, pricing clarification, and action footer preserve the locked information hierarchy. Browser document and board bounds equal their viewports at both verified sizes.
- Colors and tokens: neutral light surfaces and 8px-or-smaller radii remain constant. macOS Aqua, Windows Cobalt, and Linux Emerald are the only lane color variation.
- Brand and icons: the selected Option 1-derived mark occupies the reserved product positions. Existing Lucide paths communicate model, permissions, hotkey, cleanup, privacy, and tier state without placeholder assets.
- Copy and truth: the implementation deliberately corrects illustrative claims that conflict with accepted ADRs or current capability. Whisper-Ahead remains disabled/P3; desktop capability is one-time while Harbor is optional and self-hostable; only Free is selectable; and native model, permission, timing, and first-dictation state come from Rust instead of optimistic fixtures.
- Platform truth: the host macOS lane renders backend permission rows and working runtime actions. Windows and Linux are explicit reference lanes with their own permission vocabulary and disabled host-only model, hotkey, proof-refresh, and runtime actions.
- Interaction and accessibility: model/hotkey/cleanup selectors, Hold/Toggle, Evidence dialog tabs, Escape/close/autofocus, setup proof export, three OS lanes, and Start Dictating navigation pass. Native accessibility exposes the evidence dialog and disabled reference controls with correct roles.

**Comparison History**

1. RED baseline: the prior Setup surface was embedded in the App monolith and did not match the board's nine-step rail, pricing choices, progress line, or dedicated first-run hierarchy.
2. Initial board build: extracted `FirstRunView` plus split CSS, implemented the full source-order composition, and retained deep model/permission/proof controls in an evidence dialog.
3. Truth pass: replaced "No subscriptions. Pay once." with the accepted one-time desktop plus optional Harbor contract; kept Whisper-Ahead at P3 and paid choices informational until their owning flows exist.
4. Responsive/browser pass: proved exact 540 x 800 and 900 x 600 bounds, stable controls, modal tabs, lane accents, Dictate round trip, and an empty warning/error log.
5. Native pass: caught and repaired a cross-OS truth defect where Windows initially inherited macOS permission labels. Rebuilt the signed app and proved macOS host controls plus Windows/Linux reference contracts and disabled runtime-only actions.

**Implementation Checklist**

- [x] Locked Board 10 composition
- [x] Browser interactions at 540 x 800 and 900 x 600
- [x] Signed native macOS rendering and interaction proof
- [x] Real Debian 12 GNOME/Wayland WebView rendering and permission-dialog proof
- [x] Backend-owned model, permission, hotkey, timing, and first-dictation states
- [x] Explicit macOS, Windows, and Linux capability copy and accents
- [x] First-run view and both CSS files remain under 800 lines

**Follow-up Polish**

- P1-G4: all four Windows views have diagnostic captures; a passing workflow set including the permissions modal plus final human signoff remain required.
- P1-P0-8: reviewed model metadata/download flow, remaining live permission proof, real first-dictation journey, and <=60-second reference-machine timing remain open.
- P3: Whisper-Ahead controls become interactive only when their owning capability lands.

final result: passed (Board 10 scope only; P1-G4 remains pending)
