# crates/ — Shared Rust Crates

Extraction targets, intentionally empty until Phase 2+. The rule: a module
graduates from `apps/desktop/src-tauri/src/<module>/` into a crate here only
when (a) its API has been stable for a full phase, and (b) a second consumer
exists or is scheduled (CLI, MCP server, mobile core, bench harness).

Planned:
- `asr-core/` — the AsrEngine trait + implementations, decoupled from Tauri,
  so the bench harness and a future MCP server consume it directly.
- `cleanup-core/` — rule engine + correction collapse + prompt contracts,
  fuzzable and benchmarkable in isolation.

Premature extraction is a known failure mode — do not create crates here to
"organize"; create them to serve a second consumer. Each crate gets its own
AGENTS.md at graduation, carrying over the module guide's invariants verbatim.
