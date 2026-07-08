# history/ — Persistence & Recovery

## Owns
The SQLite schema and migrations, session records (id, timestamps, app,
engine, dial, raw text, clean text, audio path, per-stage timings, injection
method/holds/failures), audio retention policy (FLAC-compress after finalize;
user-configurable retention, default 30 days; "delete all" that actually
deletes), crash recovery (orphaned WAL files re-transcribed or surfaced on
next launch), and the export path (JSON/text).

## Invariants
1. **History is the safety net (non-negotiable #2):** a `Failed` at any stage
   still writes whatever exists. The HUD's "recover last" reads from here.
2. Migrations are forward-only, tested against a fixture DB from every
   released version.
3. All data under the OS app-data dir; documented plainly; no other location;
   secure-delete on user purge (best-effort per filesystem, documented).
4. Timings recorded per stage power the diagnostics panel and bench — keep the
   span names in lockstep with `scripts/bench.sh`.
5. DB access via one pool owned here; other modules get handles, never paths.

## Tests
Crash-orphan recovery, retention sweep, purge completeness, migration ladder.
