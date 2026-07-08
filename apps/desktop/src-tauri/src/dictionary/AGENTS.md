# dictionary/ — Vocabulary, Replacements, Snippets

## Owns
User term store (term, optional spoken-form hints, case sensitivity), engine
bias-hint export (to engine/), post-cleanup replacement pass, snippets (spoken
trigger → saved text), importers (Wispr Flow and Glaido dictionary exports —
P1-2), and the correction-learning intake (P1-3: a user edit within 30 s of
injection produces a *suggested* dictionary entry, applied only on explicit
confirm).

## Invariants
1. Replacement pass runs after cleanup/, is deterministic, and is word-boundary
   aware — "AI" must not rewrite "fail". Longest-match-first; user terms beat
   built-ins.
2. Imports are idempotent and previewable: show the diff before commit.
3. Correction-learning is never silent (privacy-of-intent: users must trust we
   don't mutate behavior behind their backs).
4. Snippet triggers require an exact isolated token by default; fuzzy matching
   is opt-in per snippet.
5. Store is plain SQLite via history/'s DB handle — exportable as JSON from
   the UI (user owns their data, fully).

## Tests
Boundary-safety property tests, importer fixtures (sample Wispr/Glaido export
files in `fixtures/`), longest-match ordering, snippet trigger isolation.
