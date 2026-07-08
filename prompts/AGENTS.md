# prompts/ — the build system

These files turn the repo's docs into a self-driving, evidence-gated build.

- `KICKOFF.md` — the single-command entry point (mirrored as `/kickoff` in
  `.claude/commands/`). Paste into Claude Code to launch the loop.
- `BUILD-LOOP.md` — the loop: ORIENT→PLAN→BUILD→JUDGE→AUDIT→GATE→loop, with the
  human gates and command vocabulary.
- `JUDGE-AUDITOR.md` — the two review lenses (quality vs evidence) and the
  definition of critical decision paths.

Rules for agents editing these: the loop and gates are load-bearing — changing
them is itself a critical decision path (ADR + operator gate). Keep them in sync
with the root charter's §7 (workflow) and §8 (Definition of Done).
