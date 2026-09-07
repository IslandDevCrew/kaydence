# apps/desktop/src/ — React Frontend (Presentation Only)

## Owns
Three surfaces: the **HUD** (tiny always-on-top overlay: recording state,
streaming partials, dial indicator, held-text recovery), **Settings** (one
clean screen first, advanced behind a disclosure — P0-8 forbids a settings
maze), and **History** (sessions list, raw↔clean diff view, audio playback,
export, purge).

## The hard rule
No business logic. No text transformation. No network. No timers that decide
product behavior. State arrives via the generated `SessionEvent` TS types;
intents leave via Tauri commands. If you import an LLM client or write a
regex over transcript text here, you are in the wrong layer — move it to Rust.

## Conventions
- TypeScript strict, no `any`; components in `components/`, route views in
  `views/`, a single store in `state/` (Zustand or equivalent — keep it one).
- Event types are generated from Rust (`scripts/gen-types`) — never hand-edit.
- HUD performance: it renders during dictation; keep it allocation-light,
  animation via CSS, target 60 fps on integrated graphics.
- Accessibility: full keyboard nav, visible focus, respects OS reduced-motion,
  legible at 200% scaling.
- Modal surfaces use `components/Modal`: native background inertness, bounded
  Tab/Shift-Tab navigation, Escape dismissal, and focus restoration to the opener.
  Keep Close reachable at the 450x300 effective viewport (900x600 at 200%).
- Visual tone: calm, quiet, utilitarian. The product's personality is
  restraint — no gamification, no confetti, no upsell surfaces.

## Definition of done
Works against mocked event streams (Storybook-style fixtures for: streaming,
held-focus, failure, recovery), passes typecheck/lint, verified on both OS
webviews (rendering differs!).
