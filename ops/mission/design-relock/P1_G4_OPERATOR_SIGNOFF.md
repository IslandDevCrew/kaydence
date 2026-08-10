# P1-G4 — Operator Signoff (2026-08-09, evening)

## Verdict: APPROVED

Jon's own words: *"P1 through G4 sign off is approved please proceed."*

This is the human gate. Only the operator can close it, and he has. Recorded here precisely,
per archipelago discipline — the exact scope of what was approved, the one carve-out, and what
still needs to land as evidence before `ops/mission/state.json` reflects it.

## What was approved

The full design-relock chain, end to end:
1. The gauntlet-lite process (3 directions, blind critics, judge) and its Editorial Precision
   winner, locked at operator 8.5/10 with an explicit, honest threshold waiver.
2. The Cockpit v2 restructure from the operator's full click-through walkthrough: status demoted
   to a clickable bottom bar, capture centered, transcript given the full-width bottom half,
   the compact-HUD direction, and every status-bar micro-decision (grid-boundary alignment,
   brand+version placement, the mic-level meter centered in the negative space) refined across
   six rounds of "final edit" feedback, each one implemented, verified against real gates, and
   confirmed correct against the operator's own description before moving to the next.
3. All three Cockpit layouts (Pill Bar, Mic Capsule, Stacked Panel) shipping as selectable
   presets rather than a single forced winner — resolving the original flank-arrangement
   ambiguity by making it a user choice instead of an agent's guess.
4. The `DictateView` rebuild implementing all of the above in real, gate-verified production
   code (not just mockups) — `tsc`/`eslint` clean, design-token rubric clean, logic-preservation
   independently diffed against the pre-relock component.

## The one carve-out — logo

Jon's own words: *"the only thing that may change later is the logo... I've not said anything
about it [because] it can stay... if I decide to change the logo later, that would just be a
small nuance tweak."*

The current mark (Option 1, "K Waveform," ADR-0012) is **not** part of this signoff's design
judgment — it was already locked separately under ADR-0012 before tonight's session, and Jon is
explicit that tonight's approval does not re-litigate it. He has independently produced
candidate replacement logos in the past and may swap the mark in later; that is scoped as a
**separate, small, future change** (an asset swap against the same identity slot ADR-0012
already reserves), not a blocker to closing P1-G4 now and not a reason to hold this signoff open.

## What is still required before `state.json` P1-G4 flips to passed

Per `plan.lock.json`'s own governing rule (`hitl.conditions`): *"No... `ops/mission/state.json`
gate flip is authorized by this plan alone — P1-G4 remains PENDING until P4 (D5) evidence lands
and the operator signs off explicitly."* The operator signoff has now landed (this document).
The D5 evidence — real captures of the actual running app at this branch's HEAD — is what
converts "operator says proceed" into a gate that can honestly be marked closed. That capture is
in progress; this document records the human decision, and the gate flip itself happens in the
same work session once the evidence file exists and is hashed, per the ledger discipline used
throughout this design-relock sub-mission.

## Scope note for whoever reads this later

This signoff closes P1-G4 for the **design-language and Cockpit-v2 restructure** work done
tonight. It does not touch, and has no bearing on, P1-G3 (packaged-app resident-footprint
lifetime, still BLOCKED pending the exclusive diagnosis window) or ADR-0016 (still
direction-approved, not ship-authorized). Those remain exactly where the fleet's earlier control
packet left them.
