import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  MINIMUM_IDLE_MS,
  MINIMUM_OBSERVATION_MS,
  awaitWithTimeout,
  buildReport,
  computeIdleMetrics,
  parseMacCpuTime,
  parseProbeOutput,
  safeProbe,
  validateProbe,
  validateReady,
} from "../scripts/bench-reference.mjs";

const runner = resolve("scripts/bench-reference.mjs");

assert.ok(existsSync(runner), "reference benchmark runner must exist");

const source = readFileSync(runner, "utf8");

assert.match(source, /process\.platform === "win32"/);
assert.match(source, /process\.platform === "darwin"/);
assert.match(source, /\/proc\/\$\{pid\}\/stat/);
assert.match(source, /physical_os_field_injection/);
assert.match(source, /idle_cpu_pct/);

assert.equal(MINIMUM_OBSERVATION_MS, 3_000);
assert.ok(MINIMUM_IDLE_MS >= 5_000);
assert.equal(parseMacCpuTime("00:03.25"), 3.25);
assert.equal(parseMacCpuTime("01:02:03.50"), 3_723.5);
assert.equal(parseMacCpuTime("2-01:02:03.25"), 176_523.25);
assert.throws(() => parseMacCpuTime("not-a-time"), /malformed/);

const mib = 1024 * 1024;
const atCpuBudget = computeIdleMetrics(
  { cpuSeconds: 10, rssBytes: 200 * mib, observedAtMs: 1_000 },
  { cpuSeconds: 10.03, rssBytes: 250 * mib, observedAtMs: 4_000 },
);
assert.ok(Math.abs(atCpuBudget.idleCpuPct - 1) < 1e-9);
assert.equal(atCpuBudget.idleRamMb, 250);
assert.equal(atCpuBudget.observationMs, 3_000);
assert.throws(
  () =>
    computeIdleMetrics(
      { cpuSeconds: 1, rssBytes: 1, observedAtMs: 0 },
      { cpuSeconds: 1, rssBytes: 1, observedAtMs: 2_999 },
    ),
  /observation interval was too short/,
);

function probe(overrides = {}) {
  return {
    schema: 1,
    platform: "linux",
    lane: "local_cpu",
    warmup_ms: 50,
    sample_count: 10,
    release_to_delivery_policy_p50_ms: 600,
    release_to_delivery_policy_p95_ms: 1_200,
    audio_ms: 2_496,
    transcript_nonempty: true,
    events: ["audio_persisted", "partial", "raw_final"],
    unmeasured: ["physical_os_field_injection"],
    ...overrides,
  };
}

const fallbackProbe = probe();
assert.doesNotThrow(() => validateProbe(fallbackProbe, "linux"));
const fallbackReport = buildReport({
  probe: fallbackProbe,
  requestedLane: "local_gpu",
  before: { cpuSeconds: 5, rssBytes: 200 * mib, observedAtMs: 10_000 },
  after: { cpuSeconds: 5.03, rssBytes: 250 * mib, observedAtMs: 13_000 },
  childStderrBytes: 17,
});
assert.equal(fallbackReport.lane, "local_cpu");
assert.equal(fallbackReport.requested_lane, "local_gpu");
assert.equal(fallbackReport.budgets.release_to_delivery_policy_p95_ms, 1_200);
assert.deepEqual(fallbackReport.budget_failures, []);
assert.equal(fallbackReport.status, "pass");

assert.throws(() => parseProbeOutput("not JSON"), /probe JSON was malformed/);
assert.throws(
  () => validateProbe(probe({ events: ["raw_final", "/private/source/path"] }), "linux"),
  /event metadata was not safe/,
);
const redacted = safeProbe(
  probe({ unmeasured: ["physical_os_field_injection", "/private/source/path"] }),
);
assert.deepEqual(redacted.unmeasured, ["physical_os_field_injection"]);
assert.equal(JSON.stringify(redacted).includes("/private/source/path"), false);

assert.throws(
  () => validateReady({ pid: 41, phase: "idle", idle_ms: 5_000 }, 42),
  /spawned idle process/,
);

const timeoutStarted = Date.now();
await assert.rejects(
  awaitWithTimeout(new Promise(() => {}), 25, "probe completion timed out"),
  /probe completion timed out/,
);
assert.ok(Date.now() - timeoutStarted < 500, "completion timeout must reject promptly");

const failedReport = buildReport({
  probe: probe({ release_to_delivery_policy_p95_ms: 1_201 }),
  requestedLane: "local_gpu",
  before: { cpuSeconds: 2, rssBytes: 251 * mib, observedAtMs: 20_000 },
  after: { cpuSeconds: 2.031, rssBytes: 252 * mib, observedAtMs: 23_000 },
  childStderrBytes: 99,
});
assert.deepEqual(failedReport.budget_failures, [
  "release_to_delivery_policy_p95_ms",
  "idle_ram_mb",
  "idle_cpu_pct",
]);
assert.equal(failedReport.release_to_delivery_policy_p95_ms, 1_201);
assert.equal(failedReport.idle_ram_mb, 252);
assert.ok(failedReport.idle_cpu_pct > 1);
assert.equal(failedReport.status, "fail");
assert.deepEqual(Object.keys(failedReport.diagnostics), ["child_stderr_bytes"]);

console.log("reference-bench behavioral contract: PASS");
