import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import {
  MAXIMUM_CONTROLLER_RUNTIME_MS,
  MAXIMUM_IDLE_MS,
  MINIMUM_IDLE_MS,
  MINIMUM_OBSERVATION_MS,
  awaitWithTimeout,
  buildReport,
  computeIdleMetrics,
  parseMacCpuTime,
  parseLinuxSample,
  parseProbeOutput,
  parseWindowsSample,
  remainingControllerMs,
  runCommand,
  safeProbe,
  validateIdleMs,
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
assert.doesNotMatch(source, /node:url/);

const help = spawnSync(process.execPath, [runner, "--help"], {
  encoding: "utf8",
  killSignal: "SIGKILL",
  timeout: 2_000,
});
assert.equal(help.status, 0, help.stderr);
assert.equal(help.stdout.trim(), "usage: node scripts/bench-reference.mjs [--check]");

assert.equal(MINIMUM_OBSERVATION_MS, 3_000);
assert.ok(MINIMUM_IDLE_MS >= 5_000);
assert.equal(MAXIMUM_IDLE_MS, 30_000);
assert.ok(MAXIMUM_CONTROLLER_RUNTIME_MS <= 180_000);
assert.equal(validateIdleMs(30_000), 30_000);
assert.throws(() => validateIdleMs(30_001), /idle interval was invalid/);
assert.equal(remainingControllerMs(10_000, 9_000, 5_000), 1_000);
assert.equal(remainingControllerMs(10_000, 5_000, 2_000), 2_000);
assert.throws(() => remainingControllerMs(10_000, 10_000, 1_000), /deadline/);
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
    model_id: "ggml-base.en",
    model_sha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002",
    fixture_sha256: "085e4b157e9f3ce0114b072354d07ede5cbcecce95bc59771b808e78abd94247",
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

const expected = {
  expectedPlatform: "linux",
  requestedLane: "local_gpu",
  expectedModelId: "ggml-base.en",
  expectedModelSha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002",
};

const fallbackProbe = probe();
assert.doesNotThrow(() => validateProbe(fallbackProbe, expected));
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

for (const [overrides, message] of [
  [{ transcript_nonempty: false }, /non-empty transcript/],
  [{ events: ["raw_final"] }, /required events/],
  [{ events: ["audio_persisted"] }, /required events/],
  [{ events: ["audio_persisted", "raw_final", "failed"] }, /forbidden event/],
  [{ events: ["audio_persisted", "raw_final", "held"] }, /forbidden event/],
  [{ sample_count: 5.5 }, /invalid measurement/],
  [{ sample_count: 4 }, /too few samples/],
  [{ warmup_ms: 1.5 }, /invalid measurement/],
  [{ warmup_ms: -1 }, /invalid measurement/],
  [{ release_to_delivery_policy_p50_ms: Number.POSITIVE_INFINITY }, /invalid measurement/],
  [{ release_to_delivery_policy_p50_ms: 1_201 }, /p50 exceeded p95/],
  [{ audio_ms: 0 }, /audio duration/],
  [{ model_id: "../../model" }, /model identifier/],
  [{ model_sha256: "0".repeat(64) }, /model hash/],
  [{ fixture_sha256: "xyz" }, /fixture hash/],
]) {
  assert.throws(() => validateProbe(probe(overrides), expected), message);
}

assert.throws(
  () => validateProbe(probe({ lane: "local_gpu" }), { ...expected, requestedLane: "local_cpu" }),
  /requested CPU lane/,
);
assert.doesNotThrow(() =>
  validateProbe(probe({ lane: "local_cpu" }), { ...expected, requestedLane: "local_cpu" }),
);
assert.doesNotThrow(() =>
  validateProbe(probe({ lane: "local_gpu" }), { ...expected, requestedLane: "local_gpu" }),
);

const justOverReport = buildReport({
  probe: fallbackProbe,
  requestedLane: "local_gpu",
  before: { cpuSeconds: 0, rssBytes: 250.0004 * mib, observedAtMs: 30_000 },
  after: { cpuSeconds: 0.030012, rssBytes: 250.0004 * mib, observedAtMs: 33_000 },
  childStderrBytes: 0,
});
assert.equal(justOverReport.idle_ram_mb, 250);
assert.equal(justOverReport.idle_cpu_pct, 1);
assert.deepEqual(justOverReport.budget_failures, ["idle_ram_mb", "idle_cpu_pct"]);
assert.equal(justOverReport.status, "fail");

assert.throws(() => parseProbeOutput("not JSON"), /probe JSON was malformed/);
assert.throws(
  () => validateProbe(probe({ events: ["raw_final", "/private/source/path"] }), expected),
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
assert.doesNotThrow(() => validateReady({ pid: 42, phase: "idle", idle_ms: 30_000 }, 42));
assert.throws(
  () => validateReady({ pid: 42, phase: "idle", idle_ms: 30_001 }, 42),
  /spawned idle process/,
);

const windowsSample = parseWindowsSample(
  '{"Id":42,"CPU":1.25,"WorkingSet64":1048576,"SampledAtMs":1234}',
  42,
);
assert.deepEqual(windowsSample, { rssBytes: 1_048_576, cpuSeconds: 1.25, observedAtMs: 1_234 });
assert.throws(() => parseWindowsSample("not json", 42), /Windows process sampler data/);
assert.throws(
  () => parseWindowsSample('{"Id":41,"CPU":1,"WorkingSet64":1,"SampledAtMs":1}', 42),
  /Windows process sampler found no child/,
);

const linuxSample = parseLinuxSample(
  "42 (reference bench) S 1 2 3 4 5 6 7 8 9 10 120 30 0 0",
  "Name:\treference-bench\nVmRSS:\t2048 kB\n",
  100,
  1_234,
);
assert.deepEqual(linuxSample, { rssBytes: 2_097_152, cpuSeconds: 1.5, observedAtMs: 1_234 });
assert.throws(() => parseLinuxSample("malformed", "VmRSS:\t1 kB\n", 100, 1), /Linux process sampler data/);
assert.throws(
  () => parseLinuxSample("42 (x) S 1 2 3 4 5 6 7 8 9 10 11 nope 1", "VmRSS:\t1 kB\n", 100, 1),
  /Linux process CPU time/,
);

const timeoutStarted = Date.now();
await assert.rejects(
  awaitWithTimeout(new Promise(() => {}), 25, "probe completion timed out"),
  /probe completion timed out/,
);
assert.ok(Date.now() - timeoutStarted < 500, "completion timeout must reject promptly");

const fixtureDirectory = mkdtempSync(join(tmpdir(), "kaydence-command-timeout-"));
const fixturePidFile = join(fixtureDirectory, "pid");
const commandStarted = Date.now();
try {
  await assert.rejects(
    runCommand(
      process.execPath,
      [
        "-e",
        'require("node:fs").writeFileSync(process.argv[1], String(process.pid)); setInterval(() => {}, 1_000)',
        fixturePidFile,
      ],
      { timeoutMs: 150 },
    ),
    /process sampler command timed out/,
  );
  assert.ok(Date.now() - commandStarted < 1_500, "host command timeout must reject promptly");
  const fixturePid = Number(readFileSync(fixturePidFile, "utf8"));
  assert.throws(
    () => process.kill(fixturePid, 0),
    (error) => error && error.code === "ESRCH",
    "timed-out host command must be terminated",
  );
} finally {
  rmSync(fixtureDirectory, { recursive: true, force: true });
}

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
