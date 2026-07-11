import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const runner = resolve("scripts/bench-reference.mjs");

assert.ok(existsSync(runner), "reference benchmark runner must exist");

const source = readFileSync(runner, "utf8");

assert.match(source, /process\.platform === "win32"/);
assert.match(source, /process\.platform === "darwin"/);
assert.match(source, /\/proc\/\$\{pid\}\/stat/);
assert.match(source, /physical_os_field_injection/);
assert.match(source, /SAFE_EVENT_NAMES/);
assert.match(source, /idle_cpu_pct/);
assert.match(source, /child\.pid/);
assert.match(source, /release_to_delivery_policy_p95_ms/);
assert.match(source, /idle_ram_mb/);
assert.match(source, /budget_failures/);
assert.match(source, /250/);
assert.match(source, /1/);

console.log("reference-bench source contract: PASS");
