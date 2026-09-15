# P1-G4 cleanup capability wording — local candidate, 2026-09-14

ORIENT: origin/main86a389a. Frontend-only correction; no Rust, event contract,
dictionary wiring, model rewrite or profile implementation is introduced.

Source evidence: cleanup/mod.rs clean_text maps Light and Full to RuleCleaner;
Raw is untouched and emits no CleanFinal. Pipeline defaults to an empty DictionaryPass;
with_dictionary_pass has only a test caller. Existing RuleCleaner tests bind all three
new illustrative input/output pairs exactly. The helper exists; production custom
dictionary entries are not connected, rather than the whole implementation being absent.

BUILD: Dictate no longer promises Full profile rewriting or shows active rule checks
in Raw. Cleanup reports six built-in rules and explicitly disconnected dictionary;
Raw bypasses all rules. Examples identify themselves as illustrative and use tested
spoken punctuation/filler/correction markers, not unsupported semantic rewriting.

Local presentation regression PASS:18 combinations (2themes x3cockpits x3dials),
selected dial persists across navigation, Raw examples remain unchanged, dictionary
stays unchecked, no Profile rule claim, no browser console/page errors. tsc/eslint PASS.
Six900x600 Cleanup PNGs and the browser log are hashed. Main visually inspected Full:
current-main tiny/truncated body text remains, requiring the separate readability packet.
These captures are not a readability/contrast/native or whole-P1-G4 acceptance.

Source SHA256:
- DictateView.tsx ae0386ecc84f9dfb9aa5eb5b3dd042fe7e757cb54c526c75dcab036234f64bf6
- CleanupView.tsx cc2ea1ed4e81e7954538689f98485fd393527fa79333ac39e272839f4fce078b
- unchanged cleanup/mod.rs 8fb6da3b8de43e96fcd1ca68cd1c54dcd3a13b7cd513fc3e28433b42e60a5ab5

Pending: integrate after contrast/body packets, rerun visible full-text/reflow tests,
independent Judge, full local gates, exact-head3OS CI before PR merge. No completion claim.
