# Kaydence Visual-Build Prompt Pack (Fable-Safe)

Scoped, safe prompts for resuming Kaydence's UI/UX build against the **locked Visual
System v1** — five logo directions plus ten screen-family boards, each rendered for
macOS, Windows, and Linux (30 OS-specific views).

These prompts follow the `fable-safe-prompt-rewriter` rules: preserve the legitimate
goal, keep authorization and scope explicit, invent no access/claims/capabilities, keep
outputs evidence-first, and stop at every critical decision path. This is a
clarity-and-safety layer for authorized work on an **owned repo** — not a bypass of any
safety system.

- **Owned repo:** `/Users/IDC2.5/Kaydence/kaydence` (local-only, default branch `main`).
- **Locked reference:** `ops/mission/state.json` → `designDirection`, and the generation
  report at `/Users/IDC2.5/Documents/Kaydence/docs/generated/Kaydence-Visual-Generation-Report-v1-2026-07-08.html`.
- **Boards (in-repo since P1-D1):** `assets/brand/`
  (`logos/kaydence-logo-option-{1..5}.png`, `screens/kaydence-screen-family-{01..10}.png`);
  index at `docs/design/README.md`.
- **Governing loops:** `prompts/BUILD-LOOP.md` (ORIENT→PLAN→BUILD→JUDGE→AUDIT→GATE) and
  `prompts/JUDGE-AUDITOR.md` (critical decision paths). Mission-control governs phases.

## Safety core (applies to every prompt below)

- Authorized, owned repo only. No unauthorized access, scraping, credential handling,
  or bypass framing.
- Build/report only. Do not invent product capabilities, permissions, URLs, launch
  claims, or "verified" results that were not actually produced.
- Evidence-first. Every "done" saves an artifact under `ops/mission/evidence/`.
- The boards are the source of truth: match layout, left-nav order, 8px card radius, and
  product states from the board; the **only** free-tier per-OS variation is the selection
  accent (macOS aqua / Windows cobalt / Linux emerald).
- Shared invariants never vary by OS — nav order, card radius, prediction-blue,
  amber-gold merge pulse, destructive confirm gate, and no cloud-sync copy on core
  screens. Changing one is a critical decision path that **HALTS** for operator go + ADR.
- Stop conditions: a critical decision path, a red Audit, a non-negotiable at risk, or a
  gate that cannot close on this host (record the blocker; do not work around it).

---

## A · Session `/goal` — resume the visual-driven build

```md
/goal Resume the Kaydence UI/UX build against the locked Visual System v1.

Authorization: owned local repo /Users/IDC2.5/Kaydence/kaydence (default branch main),
local-only, no remote. This is authorized product/engineering work, not access to any
private system and not a bypass of any control.

Read first (source of truth, in order):
1. ops/mission/state.json  — especially designDirection, the active phase, and each
   phase's screen-family fidelity gate (P1-G4, P2-G3, P3-G4, P4-G-SCREENS).
2. ops/mission/state-of-the-union.html — generated view; never hand-edit it.
3. The generation report and the screen-family boards named in designDirection.

Operating rules:
- Build each screen to its screen-family board. The only free-tier per-OS variation is
  the selection accent (macOS aqua / Windows cobalt / Linux emerald).
- Keep the shared invariants fixed (nav order; 8px card radius; prediction-blue;
  ~0.5s amber-gold merge pulse; destructive confirm gate; no cloud-sync copy on core
  screens). Treat any change to them as a critical decision path — HALT for operator go
  and an ADR.
- Follow BUILD-LOOP.md inside each task; save gate evidence under ops/mission/evidence/.
- Do not invent access, capabilities, or "verified" claims. Report blockers loudly.
- End of session: update state.json, run `node ops/mission/render-sotu.mjs`, append
  ops/mission/journal.md, commit state + journal + HTML.

Deliver: the next unblocked task built to its board, its fidelity gate reviewed with
saved evidence, and an honest note of anything that could not close on this host.
```

## B · Build one screen family for the current phase

```md
Build screen family <NN · Name> for the Kaydence desktop app (Tauri shell,
React/TypeScript presentation; business logic stays in the Rust modules per
docs/ARCHITECTURE.md). This is authorized work on the owned monorepo.

Reference (do not deviate from the board without operator go + ADR):
- Board: assets/brand/screens/kaydence-screen-family-<NN>.png
- designDirection in ops/mission/state.json (invariants + OS lanes).

Requirements:
- Reproduce the board's layout, left-nav order, component set, 8px card radius, and
  product states. Prediction suggestions are blue; accepted predictions pulse amber-gold
  ~0.5s; destructive actions require an explicit confirm gate.
- Implement all three OS lanes from one interface system; vary only the free-tier
  selection accent (aqua/cobalt/emerald) and the OS-specific permission/injection copy.
- Keyboard-reachable, readable at high OS scaling, no text overlap at narrow widths.
- Never imply cloud sync for core features; Harbor is optional/self-hostable only.
- Use a placeholder logo in the reserved slot until ADR-0012 locks the winner.

Do not: invent modules, states, or copy not on the board or in the architecture docs;
add network surfaces without an audit-network allowlist entry + ADR.

Output: the built screen(s), the files changed, and a short self-check against each
board element and invariant. Save a screenshot/diff note under ops/mission/evidence/.
```

## C · Screen-family fidelity design-review (gate)

```md
Run the screen-family fidelity gate for phase <P?>. Authorized review of owned,
already-built screens — compare them to their boards; do not modify product behavior.

Compare each built screen against its board
(assets/brand/screens/kaydence-screen-family-<NN>.png) and check:
- Layout, left-nav order, component set, and 8px card radius match the board.
- Product states match (prediction-blue, ~0.5s amber-gold merge pulse where applicable,
  destructive confirm gate where applicable).
- OS lanes differ only by the free-tier selection accent + OS permission/injection copy.
- Keyboard reachability, high-scaling readability, no text overlap at narrow widths.
- No cloud-sync copy on core screens; Harbor shown as optional/self-hostable only.

Output: a per-screen PASS / NEEDS-WORK list citing the specific board element for each
finding, plus the keyboard/a11y checklist result. Save the review under
ops/mission/evidence/ and set the gate status honestly. Uncertain items are labeled as
questions, not passes. Do not claim a pass you did not verify.
```

## D · Logo selection review → ADR-0012 (operator decision)

```md
Prepare the logo-selection decision for the operator. This is a design recommendation,
not an authority to lock identity — the operator chooses.

Using only the five generated options
(assets/brand/logos/kaydence-logo-option-{1..5}.png) and their
descriptions in designDirection, produce for each option:
- Fit for an everyday app icon at small sizes (macOS/Windows/Linux).
- Fit for the public-site header and the three-desktop / Relay story.
- Scalability, legibility, and how it sits with the aqua/cobalt/emerald lanes.

Then give one recommendation with reasons and the trade-offs of the runner-up. Draft
ADR-0012 (visual identity + OS accent lanes) as PROPOSED, with the decision left blank
for the operator. Do not lock the choice, do not change any layout, and do not invent
brand claims. HALT for operator go before marking ADR-0012 Accepted or task P1-D2 done.
```

## E · Vendor the visual pack into the monorepo (P1-D1) — ✅ COMPLETED 2026-07-08

> Kept for provenance/re-vendoring (e.g. after an upstream regeneration). The pack now
> lives at `assets/brand/` with the index at `docs/design/README.md`; all fidelity-gate
> paths point in-repo.

```md
Vendor Visual System v1 into the owned monorepo as the in-repo build reference. This is
a report-only copy of owned assets — no layout invention, no new claims.

Steps:
- Copy assets/generated/kaydence-visual-pack/{logos,screens}/ from
  /Users/IDC2.5/Documents/Kaydence/ into this repo under assets/brand/ (or the location
  the operator prefers) and docs/design/.
- Add a design-reference index (docs/design/README.md) mapping each screen family →
  its board file → the phase and fidelity gate that builds it
  (P1-G4 / P2-G3 / P3-G4 / P4-G-SCREENS).
- Once vendored, update the fidelity-gate `command` paths in state.json to the in-repo
  board locations, re-render the SOTU, and note it in the journal.

Output: the copied files, the index, and the exact paths. Do not alter the images or
fabricate any board content. Verify with a file listing before committing.
```

---

_Provenance: prompts derived from the `fable-safe-prompt-rewriter` skill (PULSE / Jon
Isaac). Keep the safety core visible in any HTML/public derivative of this pack — an
abbreviated version must not be materially weaker than this source._
