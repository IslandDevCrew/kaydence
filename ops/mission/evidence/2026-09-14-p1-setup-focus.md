# P1-G4 Setup keyboard amendment — 2026-09-14

Independent Judge rejected74272b0 despite24 passing original cases: the native
scrollable step rail and nested permission action had clipped focus outlines.
The correction keeps visible outline color/width but insets those two scopes.
The checked-in24-case guard now naturally tabs through the rail, controls and
each Evidence section, checking whole outlines against viewport/ancestor clips.

Expanded guard found two more edge cases after reading/scrolled content: Refresh
proof's external ring clipped at the workspace top, and Show microphone step
was half a pixel outside the scroller.4px scroll margins on these actions fix
their actual focus reveal without relaxing thresholds or hiding content.
Fresh final24 cases PASS; independent18 cases passed the initial two-rule fix,
before the added scroll margins. Raw RED/GREEN logs and original repro PNGs
are archived. HistoricalSeptember7 captures/hashes are not final-source proof.

Pending: independent final re-Judge, rebase onto merged accent/modal/currentmain,
full final local gates, refreshed captures, exact-head3OS CI/CLEAN and heartbeat.
This is frontend browser-fixture evidence, not native/OS scaling/P1-G4 closure.
