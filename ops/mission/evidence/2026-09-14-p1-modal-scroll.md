# P1-G4 shared modal scroll/focus restoration — local candidate

ORIENT: base02a1ff518f1c7d82b4cd7f6b41d195e313fb52c0, mission/p1-modal-scroll-focus.
Routine restoration of existing keyboard/inertness behavior; no new design, prompt,
privacy, network, Rust, IPC, dependency, model, identity or mission-state authority.
Systematic-debugging, test-first and frontend-testing skills used; Browser skill
unavailable, existing bundled Playwright/Chromium used against Vite1438.

RED: initial checker5ba4401 fails12/12 cases against unchanged base served1436. Native
focus can land on a scrolling section; content-click then Shift+Tab escapes to
BODY. Short Privacy opener and Setup reverse-wrap outlines clip. Earlier original
Judge also reproduced an already-focused Close remaining offscreen after wheel.
Baseline Modal.tsx SHA2568da6559d66dbb5ff96173f4cdceaed741ef1fb5ddf19a0a921c78a3c7a04532d.

BUILD: explicitly focus first control, bound Tab from unmanaged scroll-container
focus, reveal wrapped/restored focus even when already active. Native showModal,
inertness, Escape, Close and backdrop dismissal are preserved.8px scroll margins
retain whole existing outlines; only Privacy opener margin changes outside Modal.

Initial GREEN:12 cases (Privacy/Setup x450/501x300/900x600 xlight/dark), plus12 with
Privacy body6f22aec CSS added in-browser only. Independent review rejected that
integration probe: additive CSS did not model deleted declarations. No body packet
is included here; probe captures are not final contrast/design acceptance.
Checks cover real wheel, natural Tab/Shift+Tab, content click, dynamic Setup tabs,
Escape/Close/backdrop, containment, whole-outline visibility and opener return.
Wheel settlement is observed across ancestors; a fixed80ms wait was insufficient
for Chromium's large-wheel animation. Tests never force focus or set scroll offsets.
Existing modal4/nav10/hydration7, tsc/eslint and direct Vite build PASS on the
corrected product. Build output stays in /tmp, not tracked dist.

All captures, logs, source and checker are bound by manifest.sha256. Independent
Judge/integrated full gates/three-OS CI/PR/merge remain main-agent custody. This is
not native WebView/scaling, full-text/readability/contrast or whole-P1-G4 closure.

AMENDMENT: faithful body-only stylesheet replacement revealed background scroll
chaining at the dialog's upper boundary, followed by clipped opener restoration.
Observed trace: revealFocus initially returns opener correctly; competing browser
scroll later moves it offscreen. New containment-red.log.gz binds checker c77280b4
and pre-containment5ba4401 source: dark450 background scroll170→0 is RED.
Shared dialog overscroll-behavior:contain stops propagation without a focus timer.
The strengthened checker replaces frozen body declarations including deletions,
retains served rail/opener styles, observes every keyboard/wheel scroll settlement,
and checks background immobility at both modal boundaries. Initial red.log.gz is
historical; base/privacy-body captures and corresponding logs are refreshed below.

Harness correction: immediate opposite 10000px wheel gestures intermittently
retained the earlier target (1/5 timing probes); five probes with a 300ms quiet
interval reached the inner scroller. Historical gesture failures/captures remain
archived. Final checker observes actual wheel receipt with a 2s fail-closed deadline,
then requires 300ms ancestor-scroll quiescence. Listener registration is awaited;
listener/probe cleanup is guaranteed. Keyboard still requires four stable frames.
No focus/scroll assignment, skipped assertion or production delay was introduced.
Counterfactual RED: the final checker efe6088f with only the served Modal.css replaced
by exact pre-containment 5ba4401 CSS (SHA1952e873) still fails background scroll84→0.
See final-counterfactual-red.log.gz; quiet input timing does not conceal the defect.

Final local GREEN: 12/12 base plus 12/12 faithful body probes, no frontend errors.
Independent Judge repeats both matrices (24/24) and full-read modal paths; missing
wheel receipt independently fails in 2061ms and cleans up its listener/probe.
Sources remain Modal.tsx2b940622, Modal.css7f4aef0d, PrivacyView.css059d436b;
checker efe6088f. Full digests and raw results are archived in manifest.sha256.
The independent Privacy full-read artifact retains a separate View History outline
defect at450px in body6f22aec; it is NOT waived or fixed by this shared-modal packet.
No native OS/WebView verification or broader P1-G4 closure is claimed.

Independent runners remain uncommitted in /tmp; only their raw results are archived.
Repro: set PLAYWRIGHT_MODULE to the bundled Playwright path, CAPTURE_DIR to a fresh
temporary directory, and PRIVACY_STYLE_REF=6f22aec for the Privacy runner below.
Primary reproducible checker stays plain source in this packet (not compressed).
- /tmp/kaydence-modal-judge-20260914/independent.cjs SHA256073e96780d183223e5b8913c237ccb4ab0c5cd0a71b9c3702845709ddf06c693
- /tmp/kaydence-modal-judge-20260914/setup.cjs SHA2562946649795a362cfb57a287e9b33b02cd310c10a580cab1e2330b14b77b5bbb8
- /tmp/kaydence-modal-containment-judge-20260914/receipt-negative.cjs SHA256df54eec9156d57b442fb7d33dc5b1df4fd5bb12aeedda6a434ace1b5bdb7a463
