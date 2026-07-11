import { execFile, spawn } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const MINIMUM_SAMPLE_COUNT = 5;
export const MINIMUM_OBSERVATION_MS = 3_000;
export const MINIMUM_IDLE_MS = 5_000;
const SAMPLE_WAIT_MS = 3_100;
const READY_TIMEOUT_MS = 120_000;
const COMPLETION_MARGIN_MS = 2_000;
const TERMINATE_GRACE_MS = 1_000;
const FORCE_KILL_GRACE_MS = 1_000;
const RAM_BUDGET_MB = 250;
const CPU_BUDGET_PCT = 1;
const LANE_BUDGET_MS = {
  local_gpu: 700,
  local_cpu: 1_200,
};
const SAFE_EVENT_NAMES = new Set([
  "started",
  "audio_persisted",
  "partial",
  "raw_final",
  "clean_final",
  "injected",
  "held",
  "failed",
  "prediction_offered",
  "prediction_merged",
  "prediction_dismissed",
  "prediction_stale",
]);
const PROBE_FIELDS = [
  "schema",
  "platform",
  "lane",
  "warmup_ms",
  "sample_count",
  "release_to_delivery_policy_p50_ms",
  "release_to_delivery_policy_p95_ms",
  "audio_ms",
  "transcript_nonempty",
  "events",
  "unmeasured",
];

function fail(code) {
  const error = new Error(code);
  error.code = code;
  return error;
}

function platformLabel() {
  if (process.platform === "darwin") return "macos";
  if (process.platform === "win32") return "windows";
  return process.platform;
}

function usage() {
  return "usage: node scripts/bench-reference.mjs [--check]";
}

function sleep(ms) {
  return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function runCommand(command, args) {
  return new Promise((resolveCommand, rejectCommand) => {
    execFile(command, args, { encoding: "utf8" }, (error, stdout, stderr) => {
      if (error) {
        rejectCommand(fail("process sampler command failed"));
        return;
      }
      resolveCommand({ stdout, stderr });
    });
  });
}

export function awaitWithTimeout(promise, timeoutMs, code) {
  if (!Number.isFinite(timeoutMs) || timeoutMs < 0) {
    return Promise.reject(fail(code));
  }
  return new Promise((resolveWait, rejectWait) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      rejectWait(fail(code));
    }, timeoutMs);
    Promise.resolve(promise).then(
      (value) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        resolveWait(value);
      },
      (error) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        rejectWait(error);
      },
    );
  });
}

export function parseMacCpuTime(value) {
  const match = value.trim().match(/^(?:(\d+)-)?(?:(\d+):)?(\d+):(\d+(?:\.\d+)?)$/);
  if (!match) throw fail("macOS process CPU time was malformed");
  const days = Number(match[1] || 0);
  const hours = Number(match[2] || 0);
  const minutes = Number(match[3]);
  const seconds = Number(match[4]);
  return days * 86_400 + hours * 3_600 + minutes * 60 + seconds;
}

async function sampleMac(pid) {
  const startedAtMs = Date.now();
  const { stdout } = await runCommand("ps", ["-o", "rss=,time=", "-p", String(pid)]);
  const finishedAtMs = Date.now();
  const fields = stdout.trim().split(/\s+/);
  if (fields.length !== 2 || !/^\d+$/.test(fields[0])) {
    throw fail("macOS process sampler found no child");
  }
  return {
    rssBytes: Number(fields[0]) * 1024,
    cpuSeconds: parseMacCpuTime(fields[1]),
    observedAtMs: (startedAtMs + finishedAtMs) / 2,
  };
}

async function readLinuxClockTicks() {
  const { stdout } = await runCommand("getconf", ["CLK_TCK"]);
  const ticksPerSecond = Number(stdout.trim());
  if (!Number.isFinite(ticksPerSecond) || ticksPerSecond <= 0) {
    throw fail("Linux process clock tick rate was malformed");
  }
  return ticksPerSecond;
}

function sampleLinux(pid, ticksPerSecond) {
  let stat;
  let status;
  const startedAtMs = Date.now();
  try {
    stat = readFileSync(`/proc/${pid}/stat`, "utf8");
    status = readFileSync(`/proc/${pid}/status`, "utf8");
  } catch {
    throw fail("Linux process sampler found no child");
  }
  const finishedAtMs = Date.now();
  const closeParen = stat.lastIndexOf(")");
  const fields = stat.slice(closeParen + 2).trim().split(/\s+/);
  const rssMatch = status.match(/^VmRSS:\s+(\d+)\s+kB$/m);
  if (closeParen < 0 || fields.length < 13 || !rssMatch) {
    throw fail("Linux process sampler data was malformed");
  }
  const userTicks = Number(fields[11]);
  const systemTicks = Number(fields[12]);
  if (!Number.isFinite(userTicks) || !Number.isFinite(systemTicks)) {
    throw fail("Linux process CPU time was malformed");
  }
  return {
    rssBytes: Number(rssMatch[1]) * 1024,
    cpuSeconds: (userTicks + systemTicks) / ticksPerSecond,
    observedAtMs: (startedAtMs + finishedAtMs) / 2,
  };
}

async function sampleWindows(pid) {
  const command =
    `$process = Get-Process -Id ${pid}; ` +
    "$sampledAtMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); " +
    "[PSCustomObject]@{Id=$process.Id;CPU=$process.CPU;WorkingSet64=$process.WorkingSet64;" +
    "SampledAtMs=$sampledAtMs} | ConvertTo-Json -Compress";
  const { stdout } = await runCommand("powershell", [
    "-NoProfile",
    "-NonInteractive",
    "-Command",
    command,
  ]);
  let sample;
  try {
    sample = JSON.parse(stdout);
  } catch {
    throw fail("Windows process sampler data was malformed");
  }
  if (
    !sample ||
    sample.Id !== pid ||
    !Number.isFinite(sample.CPU) ||
    !Number.isFinite(sample.WorkingSet64) ||
    !Number.isFinite(sample.SampledAtMs)
  ) {
    throw fail("Windows process sampler found no child");
  }
  return {
    rssBytes: sample.WorkingSet64,
    cpuSeconds: sample.CPU,
    observedAtMs: sample.SampledAtMs,
  };
}

async function prepareSampler() {
  const linuxClockTicks = process.platform === "linux" ? await readLinuxClockTicks() : null;
  return async (pid) => {
    if (process.platform === "darwin") return sampleMac(pid);
    if (process.platform === "win32") return sampleWindows(pid);
    if (process.platform === "linux") return sampleLinux(pid, linuxClockTicks);
    throw fail("unsupported host platform");
  };
}

function makeChild(binary, env) {
  const child = spawn(binary, [], { env, stdio: ["ignore", "pipe", "pipe"] });
  const state = { done: false, error: null, code: null, signal: null };
  const stdout = [];
  const stderr = [];
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => stdout.push(chunk));
  child.stderr.on("data", (chunk) => stderr.push(chunk));
  const completion = new Promise((resolveCompletion) => {
    child.once("error", (error) => {
      state.error = error;
    });
    child.once("close", (code, signal) => {
      state.done = true;
      state.code = code;
      state.signal = signal;
      resolveCompletion();
    });
  });
  return { child, state, stdout, stderr, completion };
}

function ensureRunning(state, child) {
  if (state.done || state.error || child.exitCode !== null || child.signalCode !== null) {
    throw fail("child exited before resident sampling completed");
  }
}

export function validateReady(ready, childPid) {
  if (!ready || typeof ready !== "object" || Array.isArray(ready)) {
    throw fail("ready file was malformed");
  }
  const keys = Object.keys(ready).sort();
  if (
    keys.length !== 3 ||
    keys.join(",") !== "idle_ms,phase,pid" ||
    !Number.isInteger(ready.pid) ||
    ready.pid !== childPid ||
    ready.phase !== "idle" ||
    !Number.isInteger(ready.idle_ms) ||
    ready.idle_ms < MINIMUM_IDLE_MS
  ) {
    throw fail("ready file did not describe the spawned idle process");
  }
  return ready;
}

function readReadyFile(readyFile, childPid) {
  let ready;
  try {
    ready = JSON.parse(readFileSync(readyFile, "utf8"));
  } catch {
    throw fail("ready file was malformed");
  }
  return validateReady(ready, childPid);
}

async function waitForReady(readyFile, child, state) {
  const deadline = Date.now() + READY_TIMEOUT_MS;
  while (Date.now() < deadline) {
    ensureRunning(state, child);
    if (existsSync(readyFile)) {
      return { ready: readReadyFile(readyFile, child.pid), observedAtMs: Date.now() };
    }
    await sleep(25);
  }
  throw fail("reference benchmark did not become ready");
}

export function parseProbeOutput(output) {
  const lines = output.trim().split(/\r?\n/).filter(Boolean);
  if (lines.length !== 1) throw fail("probe did not emit exactly one JSON line");
  try {
    return JSON.parse(lines[0]);
  } catch {
    throw fail("probe JSON was malformed");
  }
}

function requireFiniteNumber(value) {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

export function validateProbe(probe, expectedPlatform = platformLabel()) {
  if (!probe || typeof probe !== "object" || Array.isArray(probe)) {
    throw fail("probe JSON was not an object");
  }
  const keys = Object.keys(probe).sort();
  if (keys.length !== PROBE_FIELDS.length || keys.join(",") !== [...PROBE_FIELDS].sort().join(",")) {
    throw fail("probe JSON schema was not recognized");
  }
  if (probe.schema !== 1 || probe.platform !== expectedPlatform) {
    throw fail("probe schema or platform did not match this host");
  }
  if (!Object.hasOwn(LANE_BUDGET_MS, probe.lane)) {
    throw fail("probe reported an unreviewed lane");
  }
  for (const field of [
    "warmup_ms",
    "sample_count",
    "release_to_delivery_policy_p50_ms",
    "release_to_delivery_policy_p95_ms",
    "audio_ms",
  ]) {
    if (!requireFiniteNumber(probe[field])) throw fail("probe contained an invalid measurement");
  }
  if (probe.sample_count < MINIMUM_SAMPLE_COUNT) throw fail("probe reported too few samples");
  if (typeof probe.transcript_nonempty !== "boolean" || !Array.isArray(probe.events)) {
    throw fail("probe transcript or event metadata was malformed");
  }
  if (!probe.events.every((event) => typeof event === "string" && SAFE_EVENT_NAMES.has(event))) {
    throw fail("probe event metadata was not safe to report");
  }
  if (
    !Array.isArray(probe.unmeasured) ||
    !probe.unmeasured.includes("physical_os_field_injection")
  ) {
    throw fail("probe omitted the physical injection limitation");
  }
  return probe;
}

export function safeProbe(probe) {
  const report = {};
  for (const field of PROBE_FIELDS) {
    if (field !== "events" && field !== "unmeasured") report[field] = probe[field];
  }
  report.events = probe.events;
  report.unmeasured = ["physical_os_field_injection"];
  return report;
}

export function computeIdleMetrics(before, after) {
  const observationMs = after.observedAtMs - before.observedAtMs;
  if (!Number.isFinite(observationMs) || observationMs < MINIMUM_OBSERVATION_MS) {
    throw fail("idle CPU observation interval was too short");
  }
  const cpuDelta = after.cpuSeconds - before.cpuSeconds;
  if (!Number.isFinite(cpuDelta) || cpuDelta < 0) {
    throw fail("idle CPU observation moved backwards");
  }
  const idleRamMb = Math.max(before.rssBytes, after.rssBytes) / (1024 * 1024);
  if (!Number.isFinite(idleRamMb) || idleRamMb < 0) {
    throw fail("idle RAM observation was malformed");
  }
  return {
    idleCpuPct: (cpuDelta / (observationMs / 1_000)) * 100,
    idleRamMb,
    observationMs,
  };
}

function round(value) {
  return Math.round(value * 1_000) / 1_000;
}

export function buildReport({ probe, requestedLane, before, after, childStderrBytes }) {
  if (!Object.hasOwn(LANE_BUDGET_MS, requestedLane)) {
    throw fail("requested lane was not reviewed");
  }
  const metrics = computeIdleMetrics(before, after);
  const idleRamMb = round(metrics.idleRamMb);
  const idleCpuPct = round(metrics.idleCpuPct);
  const latencyBudgetMs = LANE_BUDGET_MS[probe.lane];
  const budgetFailures = [];
  if (probe.release_to_delivery_policy_p95_ms > latencyBudgetMs) {
    budgetFailures.push("release_to_delivery_policy_p95_ms");
  }
  if (idleRamMb > RAM_BUDGET_MB) budgetFailures.push("idle_ram_mb");
  if (idleCpuPct > CPU_BUDGET_PCT) budgetFailures.push("idle_cpu_pct");
  return {
    ...safeProbe(probe),
    runner_schema: 1,
    requested_lane: requestedLane,
    idle_ram_mb: idleRamMb,
    idle_cpu_pct: idleCpuPct,
    idle_sample_interval_ms: round(metrics.observationMs),
    budgets: {
      release_to_delivery_policy_p95_ms: latencyBudgetMs,
      idle_ram_mb: RAM_BUDGET_MB,
      idle_cpu_pct: CPU_BUDGET_PCT,
    },
    budget_failures: budgetFailures,
    status: budgetFailures.length === 0 ? "pass" : "fail",
    diagnostics: { child_stderr_bytes: childStderrBytes },
  };
}

function writeReport(report) {
  const directory = resolve("bench-results");
  mkdirSync(directory, { recursive: true });
  const filename = `reference-${report.platform}-${report.lane}.json`;
  const target = join(directory, filename);
  const temporary = join(directory, `.${basename(filename)}-${process.pid}.tmp`);
  writeFileSync(temporary, `${JSON.stringify(report, null, 2)}\n`);
  renameSync(temporary, target);
  return target;
}

async function forceKill(child) {
  if (process.platform === "win32") {
    try {
      await awaitWithTimeout(
        runCommand("taskkill", ["/PID", String(child.pid), "/T", "/F"]),
        FORCE_KILL_GRACE_MS,
        "forced termination command timed out",
      );
    } catch {
      child.kill();
    }
  } else {
    child.kill("SIGKILL");
  }
}

async function stopChild(child, state, completion) {
  if (state.done) return;
  child.kill();
  try {
    await awaitWithTimeout(completion, TERMINATE_GRACE_MS, "child terminate timed out");
    return;
  } catch {
    // Continue to the platform's forced termination path.
  }
  if (!state.done) await forceKill(child);
  try {
    await awaitWithTimeout(completion, FORCE_KILL_GRACE_MS, "child kill timed out");
  } catch {
    // Cleanup is bounded even if the host never reports process closure.
  }
}

function selectedLane() {
  const request = process.env.KAYDENCE_WHISPER_LANE || "gpu";
  if (request === "gpu") return "local_gpu";
  if (request === "cpu") return "local_cpu";
  throw fail("requested lane was not reviewed");
}

function requestedIdleMs() {
  const requested = Number(process.env.KAYDENCE_REFERENCE_IDLE_MS || MINIMUM_IDLE_MS);
  if (!Number.isInteger(requested) || requested < 0) {
    throw fail("requested idle interval was invalid");
  }
  return Math.max(requested, MINIMUM_IDLE_MS);
}

async function run() {
  const requestedLane = selectedLane();
  const idleMs = requestedIdleMs();
  const binary = process.env.KAYDENCE_REFERENCE_BIN || resolve("target", "release", "reference-bench");
  const model =
    process.env.KAYDENCE_WHISPER_MODEL ||
    join(homedir(), "Documents", "Kaydence", "models", "ggml-base.en.bin");
  const clip =
    process.env.KAYDENCE_WHISPER_CLIP ||
    join(homedir(), "Documents", "Kaydence", "models", "clip16k.wav");
  if (!existsSync(binary) || !existsSync(model) || !existsSync(clip)) {
    throw fail("reference benchmark inputs were unavailable");
  }

  const sampleProcess = await prepareSampler();
  const readyFile = join(tmpdir(), `kaydence-reference-${process.pid}-${Date.now()}.json`);
  rmSync(readyFile, { force: true });
  const child = makeChild(binary, {
    ...process.env,
    KAYDENCE_WHISPER_MODEL: model,
    KAYDENCE_WHISPER_CLIP: clip,
    KAYDENCE_WHISPER_LANE: requestedLane === "local_gpu" ? "gpu" : "cpu",
    KAYDENCE_REFERENCE_SAMPLES: process.env.KAYDENCE_REFERENCE_SAMPLES || "10",
    KAYDENCE_REFERENCE_IDLE_MS: String(idleMs),
    KAYDENCE_REFERENCE_READY_FILE: readyFile,
  });

  try {
    const readyState = await waitForReady(readyFile, child.child, child.state);
    const completionDeadlineMs =
      readyState.observedAtMs + readyState.ready.idle_ms + COMPLETION_MARGIN_MS;
    const before = await sampleProcess(child.child.pid);
    ensureRunning(child.state, child.child);
    await sleep(SAMPLE_WAIT_MS);
    const after = await sampleProcess(child.child.pid);
    ensureRunning(child.state, child.child);
    const completionRemainingMs = Math.max(0, completionDeadlineMs - Date.now());
    await awaitWithTimeout(child.completion, completionRemainingMs, "probe completion timed out");
    if (child.state.error || child.state.code !== 0 || child.state.signal !== null) {
      throw fail("probe exited unsuccessfully");
    }

    const probe = parseProbeOutput(child.stdout.join(""));
    validateProbe(probe);
    return buildReport({
      probe,
      requestedLane,
      before,
      after,
      childStderrBytes: child.stderr.join("").length,
    });
  } catch (error) {
    await stopChild(child.child, child.state, child.completion);
    throw error;
  } finally {
    rmSync(readyFile, { force: true });
  }
}

async function main() {
  const args = process.argv.slice(2);
  if (args.includes("--help")) {
    if (args.length !== 1) throw fail("unsupported arguments");
    console.log(usage());
    return;
  }
  if (args.some((arg) => arg !== "--check")) throw fail("unsupported arguments");

  let report;
  try {
    report = await run();
  } catch (error) {
    const requestedLane = (() => {
      try {
        return selectedLane();
      } catch {
        return "unreviewed";
      }
    })();
    report = {
      runner_schema: 1,
      platform: platformLabel(),
      lane: requestedLane,
      requested_lane: requestedLane,
      status: "fail",
      failure: error && typeof error.code === "string" ? error.code : "reference benchmark failed",
    };
  }
  writeReport(report);
  console.log(JSON.stringify(report));
  if (report.status !== "pass") process.exitCode = 1;
}

const isEntryPoint =
  process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
if (isEntryPoint) {
  main().catch(() => {
    process.exitCode = 1;
  });
}
