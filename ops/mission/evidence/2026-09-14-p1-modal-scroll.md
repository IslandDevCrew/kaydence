# P1-G4 shared modal scroll/focus restoration — local candidate

ORIENT: base02a1ff518f1c7d82b4cd7f6b41d195e313fb52c0, mission/p1-modal-scroll-focus.
Routine restoration of existing keyboard/inertness behavior; no new design, prompt,
privacy, network, Rust, IPC, dependency, model, identity or mission-state authority.
Systematic-debugging, test-first and frontend-testing skills used; Browser skill
unavailable, existing bundled Playwright/Chromium used against Vite1438.

RED: final checker fails12/12 cases against unchanged base served1436. Native
focus can land on a scrolling section; content-click then Shift+Tab escapes to
BODY. Short Privacy opener and Setup reverse-wrap outlines clip. Earlier original
Judge also reproduced an already-focused Close remaining offscreen after wheel.
Baseline Modal.tsx SHA2568da6559d66dbb5ff96173f4cdceaed741ef1fb5ddf19a0a921c78a3c7a04532d.

BUILD: explicitly focus first control, bound Tab from unmanaged scroll-container
focus, reveal wrapped/restored focus even when already active. Native showModal,
inertness, Escape, Close and backdrop dismissal are preserved.8px scroll margins
retain whole existing outlines; only Privacy opener margin changes outside Modal.

GREEN:12 cases (Privacy/Setup x450/501x300/900x600 xlight/dark), plus12 with frozen
Privacy body6f22aec CSS injected in-browser only. No body packet is included here;
overlay captures are integration probes, not final contrast/design acceptance.
Checks cover real wheel, natural Tab/Shift+Tab, content click, dynamic Setup tabs,
Escape/Close/backdrop, containment, whole-outline visibility and opener return.
Wheel settlement is observed across ancestors; a fixed80ms wait was insufficient
for Chromium's large-wheel animation. Tests never force focus or set scroll offsets.
Final browser runs report no frontend errors; existing modal4/nav10/hydration7,
tsc/eslint and direct Vite build PASS. Build output stays in /tmp, not tracked dist.

All captures, logs, source and checker are bound by manifest.sha256. Independent
Judge/integrated full gates/three-OS CI/PR/merge remain main-agent custody. This is
not native WebView/scaling, full-text/readability/contrast or whole-P1-G4 closure.
