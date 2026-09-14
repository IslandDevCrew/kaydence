# P1-G4 Dictate History focus correction — 2026-09-14

Fresh independent Judge rejected candidate5652998 despite its passing54-case suite:
Stacked Panel at761/800x600 clipped whole History keyboard targets in both themes.
At761px the20px client height could not contain25px buttons or36px summaries.
The independent48-case sweep found16 assertions failing across four combinations.

Correction: only at761..899px, allocate the two stacked flank panels equal shrinkable
rows. Both remain scrollable; no text is hidden or shrunk. Default900x600 composition
and its17.140625/17.125px meter gaps remain unchanged; <=760px intrinsic reflow remains.
The checked-in54-case test now naturally tabs through History, opens disclosures,
checks whole targets against panel/viewport bounds, and reaches the footer without
activating destructive controls. All54 cases pass, including expanded session actions.
The independent48-case suite also passes after the correction; raw logs are archived.
Main visually inspected the761px Refresh focus capture: History is now121px tall.

Source bindings before integration:
- DictateView.css: a8c2a82713ae1370b25a28ba61f7c4f7557afa32851b63a13ba9e47d4aa7340e
- readability-check.cjs: ffbbdd7f459216a0f7275d6c00bb2e9d403deb7e3a015aa2c8811701a37bdc5c

Pending: independent re-Judge, current-main rebase, refreshed integrated captures,
full local gates and exact-head3OS CI. HistoricalSeptember7 hashes/captures are not
proof of this amendment. Browser fixtures are not native/VM or complete P1-G4 proof.
