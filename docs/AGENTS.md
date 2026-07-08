# docs/ — Agent Guide

**Purpose:** the why and what of Kaydence. Code answers "how"; this directory
answers everything else. If code and docs disagree, the docs are wrong until an
ADR says otherwise — fix the doc in the same PR.

## Files
- `PRD.md` — requirements with IDs (`P0-1`…). Every PR cites one.
- `ARCHITECTURE.md` — pipeline, event contract, platform abstraction, budgets.
- `ROADMAP.md` — phases with exit criteria.
- `COMPETITIVE-ANALYSIS.md` — June 2026 evidence base behind every feature decision.
- `decisions/` — ADRs, numbered, immutable once accepted (supersede, don't edit).

## Rules for agents
1. New feature ideas enter as PRD rows (with evidence) before any code.
2. Architectural changes (new dependency, new process, new network call, new
   stage) require an ADR using the template in `decisions/0000-template.md`.
3. Keep the PRD evidence column honest — if a claim came from a vendor's own
   marketing, mark it `(vendor-claimed)`.
4. Competitive data ages fast. When touching `COMPETITIVE-ANALYSIS.md`, stamp
   the verification date per section.
5. Writing quality bar: a fresh agent with zero conversation context must be
   able to build the right thing from these docs alone. That is the test.
