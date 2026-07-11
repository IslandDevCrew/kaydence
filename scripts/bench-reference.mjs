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

const MINIMUM_SAMPLE_COUNT = 5;
export const MINIMUM_OBSERVATION_MS = 3_000;
export const MINIMUM_IDLE_MS = 7_000;
export const MAXIMUM_IDLE_MS = 30_000;
export const MAXIMUM_CONTROLLER_RUNTIME_MS = 180_000;
const SAMPLE_WAIT_MS = 3_100;
const HOST_COMMAND_TIMEOUT_MS = 1_500;
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
  "model_id",
  "model_sha256",
  "fixture_sha256",
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

export function runCommand(command, args, { timeoutMs = HOST_COMMAND_TIMEOUT_MS } = {}) {
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    return Promise.reject(fail("process sampler command timeout was invalid"));
  }
  return new Promise((resolveCommand, rejectCommand) => {
    execFile(
      command,
      args,
      {
        encoding: "utf8",
        killSignal: "SIGKILL",
        timeout: timeoutMs,
        windowsHide: true,
      },
      (error, stdout, stderr) => {
        if (error) {
          const code = error.killed || error.signal === "SIGKILL"
            ? "process sampler command timed out"
            : "process sampler command failed";
          rejectCommand(fail(code));
          return;
        }
        resolveCommand({ stdout, stderr });
      },
    );
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

async function sampleMac(pid, timeoutMs) {
  const startedAtMs = Date.now();
  const { stdout } = await runCommand("ps", ["-o", "rss=,time=", "-p", String(pid)], {
    timeoutMs,
  });
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

export function parseLinuxSample(stat, status, ticksPerSecond, observedAtMs) {
  if (!Number.isFinite(ticksPerSecond) || ticksPerSecond <= 0) {
    throw fail("Linux process clock tick rate was malformed");
  }
  const closeParen = stat.lastIndexOf(")");
  const fields = closeParen < 0 ? [] : stat.slice(closeParen + 2).trim().split(/\s+/);
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
    observedAtMs,
  };
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
  return parseLinuxSample(stat, status, ticksPerSecond, (startedAtMs + finishedAtMs) / 2);
}

export function parseWindowsSample(output, pid) {
  let sample;
  try {
    sample = JSON.parse(output);
  } catch {
    throw fail("Windows process sampler data was malformed");
  }
  if (
    !sample ||
    sample.Id !== pid ||
    !Number.isFinite(sample.CPU) ||
    sample.CPU < 0 ||
    !Number.isFinite(sample.WorkingSet64) ||
    !Number.isInteger(sample.WorkingSet64) ||
    sample.WorkingSet64 < 0 ||
    !Number.isInteger(sample.SampledAtMs) ||
    sample.SampledAtMs < 0
  ) {
    throw fail("Windows process sampler found no child");
  }
  return {
    rssBytes: sample.WorkingSet64,
    cpuSeconds: sample.CPU,
    observedAtMs: sample.SampledAtMs,
  };
}

async function sampleWindows(pid, timeoutMs) {
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
  ], { timeoutMs });
  return parseWindowsSample(stdout, pid);
}

async function prepareSampler() {
  const linuxClockTicks = process.platform === "linux" ? await readLinuxClockTicks() : null;
  return async (pid, timeoutMs = HOST_COMMAND_TIMEOUT_MS) => {
    if (process.platform === "darwin") return sampleMac(pid, timeoutMs);
    if (process.platform === "win32") return sampleWindows(pid, timeoutMs);
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
    ready.idle_ms < MINIMUM_IDLE_MS ||
    ready.idle_ms > MAXIMUM_IDLE_MS
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

async function waitForReady(readyFile, child, state, controllerDeadlineMs) {
  const deadline = Math.min(Date.now() + READY_TIMEOUT_MS, controllerDeadlineMs);
  while (Date.now() < deadline) {
    ensureRunning(state, child);
    if (existsSync(readyFile)) {
      return { ready: readReadyFile(readyFile, child.pid), observedAtMs: Date.now() };
    }
    await sleep(Math.min(25, Math.max(0, deadline - Date.now())));
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

function requireNonnegativeInteger(value) {
  return Number.isInteger(value) && value >= 0;
}

const SHA256_PATTERN = /^[A-Fa-f0-9]{64}$/;
const MODEL_ID_PATTERN = /^[A-Za-z0-9._-]{1,64}$/;

export function validateProbe(probe, {
  expectedPlatform = platformLabel(),
  requestedLane,
  expectedModelId,
  expectedModelSha256,
} = {}) {
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
  if (requestedLane === "local_cpu" && probe.lane !== "local_cpu") {
    throw fail("requested CPU lane did not report local_cpu");
  }
  if (requestedLane === "local_gpu" && !["local_gpu", "local_cpu"].includes(probe.lane)) {
    throw fail("requested GPU lane did not report a reviewed lane");
  }
  if (!MODEL_ID_PATTERN.test(probe.model_id) || probe.model_id !== expectedModelId) {
    throw fail("probe model identifier did not match the reviewed model");
  }
  if (
    !SHA256_PATTERN.test(probe.model_sha256) ||
    probe.model_sha256.toLowerCase() !== expectedModelSha256?.toLowerCase()
  ) {
    throw fail("probe model hash did not match the expected hash");
  }
  if (!SHA256_PATTERN.test(probe.fixture_sha256)) {
    throw fail("probe fixture hash was malformed");
  }
  for (const field of [
    "warmup_ms",
    "sample_count",
    "release_to_delivery_policy_p50_ms",
    "release_to_delivery_policy_p95_ms",
    "audio_ms",
  ]) {
    if (!requireNonnegativeInteger(probe[field])) {
      throw fail("probe contained an invalid measurement");
    }
  }
  if (probe.sample_count < MINIMUM_SAMPLE_COUNT) throw fail("probe reported too few samples");
  if (probe.transcript_nonempty !== true) throw fail("probe did not report a non-empty transcript");
  if (!Array.isArray(probe.events)) throw fail("probe event metadata was malformed");
  if (!probe.events.every((event) => typeof event === "string" && SAFE_EVENT_NAMES.has(event))) {
    throw fail("probe event metadata was not safe to report");
  }
  if (!probe.events.includes("audio_persisted") || !probe.events.includes("raw_final")) {
    throw fail("probe omitted required events");
  }
  if (probe.events.includes("failed") || probe.events.includes("held")) {
    throw fail("probe contained a forbidden event");
  }
  if (probe.release_to_delivery_policy_p50_ms > probe.release_to_delivery_policy_p95_ms) {
    throw fail("probe p50 exceeded p95");
  }
  if (probe.audio_ms === 0) throw fail("probe audio duration was invalid");
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

function exceedsBudget(value, budget) {
  // Absorb binary arithmetic noise only; this is far below any report precision.
  const epsilon = Number.EPSILON * 512 * Math.max(1, Math.abs(value), Math.abs(budget));
  return value - budget > epsilon;
}

export function buildReport({ probe, requestedLane, before, after, childStderrBytes }) {
  if (!Object.hasOwn(LANE_BUDGET_MS, requestedLane)) {
    throw fail("requested lane was not reviewed");
  }
  const metrics = computeIdleMetrics(before, after);
  const latencyBudgetMs = LANE_BUDGET_MS[probe.lane];
  const budgetFailures = [];
  if (probe.release_to_delivery_policy_p95_ms > latencyBudgetMs) {
    budgetFailures.push("release_to_delivery_policy_p95_ms");
  }
  if (exceedsBudget(metrics.idleRamMb, RAM_BUDGET_MB)) budgetFailures.push("idle_ram_mb");
  if (exceedsBudget(metrics.idleCpuPct, CPU_BUDGET_PCT)) budgetFailures.push("idle_cpu_pct");
  return {
    ...safeProbe(probe),
    runner_schema: 1,
    requested_lane: requestedLane,
    idle_ram_mb: round(metrics.idleRamMb),
    idle_cpu_pct: round(metrics.idleCpuPct),
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

async function forceKill(child, deadlineMs) {
  if (process.platform === "win32") {
    try {
      await runCommand("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
        timeoutMs: remainingControllerMs(deadlineMs, Date.now(), FORCE_KILL_GRACE_MS),
      });
    } catch {
      child.kill();
    }
  } else {
    child.kill("SIGKILL");
  }
}

async function stopChild(child, state, completion, deadlineMs) {
  if (state.done) return;
  child.kill();
  try {
    await awaitWithTimeout(
      completion,
      remainingControllerMs(deadlineMs, Date.now(), TERMINATE_GRACE_MS),
      "child terminate timed out",
    );
    return;
  } catch {
    // Continue to the platform's forced termination path.
  }
  if (!state.done) {
    try {
      await forceKill(child, deadlineMs);
    } catch {
      child.kill(process.platform === "win32" ? undefined : "SIGKILL");
    }
  }
  try {
    await awaitWithTimeout(
      completion,
      remainingControllerMs(deadlineMs, Date.now(), FORCE_KILL_GRACE_MS),
      "child kill timed out",
    );
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

export function validateIdleMs(requested) {
  if (!Number.isInteger(requested) || requested < 0 || requested > MAXIMUM_IDLE_MS) {
    throw fail("requested idle interval was invalid");
  }
  return Math.max(requested, MINIMUM_IDLE_MS);
}

function requestedIdleMs() {
  const requested = Number(process.env.KAYDENCE_REFERENCE_IDLE_MS || MINIMUM_IDLE_MS);
  return validateIdleMs(requested);
}

export function remainingControllerMs(deadlineMs, nowMs, requestedMs) {
  const remaining = deadlineMs - nowMs;
  if (!Number.isFinite(remaining) || remaining <= 0) {
    throw fail("reference benchmark controller deadline exceeded");
  }
  return Math.min(remaining, requestedMs);
}

async function run() {
  const controllerDeadlineMs = Date.now() + MAXIMUM_CONTROLLER_RUNTIME_MS;
  const requestedLane = selectedLane();
  const idleMs = requestedIdleMs();
  const binary = process.env.KAYDENCE_REFERENCE_BIN || resolve("target", "release", "reference-bench");
  const model =
    process.env.KAYDENCE_WHISPER_MODEL ||
    join(homedir(), "Documents", "Kaydence", "models", "ggml-base.en.bin");
  const clip =
    process.env.KAYDENCE_WHISPER_CLIP ||
    join(homedir(), "Documents", "Kaydence", "models", "clip16k.wav");
  const modelId = process.env.KAYDENCE_WHISPER_MODEL_ID;
  const modelSha256 = process.env.KAYDENCE_WHISPER_SHA256;
  if (
    !existsSync(binary) ||
    !existsSync(model) ||
    !existsSync(clip) ||
    !MODEL_ID_PATTERN.test(modelId || "") ||
    !SHA256_PATTERN.test(modelSha256 || "")
  ) {
    throw fail("reference benchmark inputs were unavailable");
  }

  const sampleProcess = await prepareSampler();
  const readyFile = join(tmpdir(), `kaydence-reference-${process.pid}-${Date.now()}.json`);
  rmSync(readyFile, { force: true });
  const child = makeChild(binary, {
    ...process.env,
    KAYDENCE_WHISPER_MODEL: model,
    KAYDENCE_WHISPER_MODEL_ID: modelId,
    KAYDENCE_WHISPER_SHA256: modelSha256,
    KAYDENCE_WHISPER_CLIP: clip,
    KAYDENCE_WHISPER_LANE: requestedLane === "local_gpu" ? "gpu" : "cpu",
    KAYDENCE_REFERENCE_SAMPLES: process.env.KAYDENCE_REFERENCE_SAMPLES || "10",
    KAYDENCE_REFERENCE_IDLE_MS: String(idleMs),
    KAYDENCE_REFERENCE_READY_FILE: readyFile,
  });

  try {
    const readyState = await waitForReady(
      readyFile,
      child.child,
      child.state,
      controllerDeadlineMs,
    );
    const completionDeadlineMs =
      readyState.observedAtMs + readyState.ready.idle_ms + COMPLETION_MARGIN_MS;
    const before = await sampleProcess(
      child.child.pid,
      remainingControllerMs(controllerDeadlineMs, Date.now(), HOST_COMMAND_TIMEOUT_MS),
    );
    ensureRunning(child.state, child.child);
    await sleep(remainingControllerMs(controllerDeadlineMs, Date.now(), SAMPLE_WAIT_MS));
    const after = await sampleProcess(
      child.child.pid,
      remainingControllerMs(controllerDeadlineMs, Date.now(), HOST_COMMAND_TIMEOUT_MS),
    );
    ensureRunning(child.state, child.child);
    const completionRemainingMs = remainingControllerMs(
      controllerDeadlineMs,
      Date.now(),
      Math.max(0, completionDeadlineMs - Date.now()),
    );
    await awaitWithTimeout(child.completion, completionRemainingMs, "probe completion timed out");
    if (child.state.error || child.state.code !== 0 || child.state.signal !== null) {
      throw fail("probe exited unsuccessfully");
    }

    const probe = parseProbeOutput(child.stdout.join(""));
    validateProbe(probe, {
      requestedLane,
      expectedModelId: modelId,
      expectedModelSha256: modelSha256,
    });
    return buildReport({
      probe,
      requestedLane,
      before,
      after,
      childStderrBytes: child.stderr.join("").length,
    });
  } catch (error) {
    await stopChild(child.child, child.state, child.completion, controllerDeadlineMs);
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

function pathFromFileUrl(value) {
  const url = new URL(value);
  let pathname = decodeURIComponent(url.pathname);
  if (process.platform === "win32") {
    if (url.hostname) return `\\\\${url.hostname}${pathname.replaceAll("/", "\\")}`;
    if (/^\/[A-Za-z]:/.test(pathname)) pathname = pathname.slice(1);
    return pathname.replaceAll("/", "\\");
  }
  return url.hostname ? `//${url.hostname}${pathname}` : pathname;
}

const isEntryPoint =
  process.argv[1] && resolve(process.argv[1]) === resolve(pathFromFileUrl(import.meta.url));
if (isEntryPoint) {
  main().catch(() => {
    process.exitCode = 1;
  });
}
