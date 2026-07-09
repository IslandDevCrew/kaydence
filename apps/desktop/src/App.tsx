import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";

const markUrl = new URL(
  "../../../assets/brand/logos/kaydence-logo-option-1.png",
  import.meta.url,
).href;

type OsLane = "mac" | "windows" | "linux";
type HotkeyMode = "push_to_talk" | "toggle";

interface LaneSpec {
  id: OsLane;
  label: string;
  accent: string;
  injection: string;
}

interface NavItem {
  label: string;
  phase: string;
}

interface Metric {
  label: string;
  value: string;
  detail: string;
}

interface PrivacyPostureItem {
  label: string;
  value: string;
  detail: string;
}

interface ChecklistItem {
  label: string;
  status: "Ready" | "Needs hardware" | "Next" | "Issue";
  detail?: string;
}

interface AppRef {
  id: string;
  name: string;
}

type FirstRunModelState = "ready" | "missing" | "blocked";

interface FirstRunModelStatus {
  id: string;
  task: string;
  lane: string | null;
  runtime: string;
  file: string;
  state: FirstRunModelState;
  detail: string;
  download_available: boolean;
  download_size_mb: number | null;
  download_source_count: number;
  license: string;
  license_review_required: boolean;
}

interface FirstRunAsrCandidate {
  id: string;
  lane: string | null;
  runtime: string;
  size_mb: number;
  min_hw: string;
  state: FirstRunModelState;
  detail: string;
  download_available: boolean;
  download_size_mb: number | null;
  download_source_count: number;
  selected: boolean;
  recommendation: string | null;
  license: string;
  license_review_required: boolean;
}

type FirstRunPermissionState = "ready" | "needs_hardware" | "needs_review" | "blocked";

interface FirstRunPermissionRequirement {
  id: string;
  label: string;
  state: FirstRunPermissionState;
  detail: string;
  action: string;
  action_label: string;
}

interface FirstRunPermissionActionOutcome {
  requirement_id: string;
  label: string;
  state: FirstRunPermissionState;
  action_label: string;
  settings_target: string | null;
  settings_open_label: string | null;
  manual_step: string;
  proof_requirement: string;
  proof_command: string | null;
  expected_evidence: string;
  ready_boundary: string;
}

interface FirstRunPermissionSettingsOpenOutcome {
  requirement_id: string;
  label: string;
  opened: boolean;
  settings_target: string | null;
  manual_step: string;
  proof_requirement: string;
  expected_evidence: string;
}

type FirstRunNextStepKind =
  | "setup"
  | "model_metadata"
  | "model_install"
  | "permission"
  | "hotkey"
  | "dictation"
  | "complete";

interface FirstRunNextStep {
  kind: FirstRunNextStepKind;
  target_id: string | null;
  title: string;
  detail: string;
  action_label: string;
  proof_requirement: string;
}

interface FirstRunSetupTiming {
  started_at_ms: number | null;
  completed_at_ms: number | null;
  elapsed_ms: number | null;
  target_ms: number;
  within_target: boolean | null;
}

interface FirstRunProofExportOutcome {
  exported: boolean;
  json_path: string | null;
  item_count: number;
}

interface FirstRunModelDownloadPreflight {
  model_id: string;
  task: string;
  lane: string | null;
  runtime: string;
  file: string;
  state: FirstRunModelState;
  detail: string;
  available: boolean;
  destination_path: string | null;
  expected_sha256: string | null;
  size_mb: number | null;
  source_count: number;
  sources: string[];
  license: string;
  license_review_required: boolean;
  blocked_reason: string | null;
  operator_action: string;
  proof_requirement: string;
}

type FirstRunAsrRuntimeState = "pending" | "blocked" | "verified_artifact";

interface FirstRunAsrRuntimeStatus {
  state: FirstRunAsrRuntimeState;
  selected_model_id: string | null;
  lane: string | null;
  runtime: string | null;
  artifact_path: string | null;
  artifact_size_bytes: number | null;
  adapter_ready: boolean;
  detail: string;
  proof_requirement: string;
}

interface HotkeyBindingOption {
  id: string;
  label: string;
  detail: string;
}

interface AppSnapshot {
  app_name: string;
  app_identifier: string;
  settings: {
    hotkey: {
      mode: "push_to_talk" | "toggle";
      primary_binding: string;
      primary_binding_options: HotkeyBindingOption[];
      secondary_dial_override_binding: string;
    };
    capture: {
      min_capture_ms: number;
      tail_buffer_ms: number;
      debounce_ms: number;
    };
    engine: {
      default_local_asr: "parakeet_cpu" | "whisper_gpu";
      auto_recommend_by_hardware: boolean;
    };
    cleanup: {
      default_dial: "raw" | "light" | "full";
      full_requires_explicit_opt_in: boolean;
    };
    injection: {
      unknown_focus_policy: "warn_and_allow" | "refuse";
      prefer_clipboard_fallback: boolean;
    };
    privacy: {
      history_retention_days: number;
      local_context_enabled: boolean;
      local_ocr_enabled: boolean;
    };
    first_run: {
      model_ready: boolean;
      model_readiness_error: string | null;
      required_models: FirstRunModelStatus[];
      asr_candidates: FirstRunAsrCandidate[];
      recommended_asr_model_id: string | null;
      selected_asr_model_id: string | null;
      asr_runtime: FirstRunAsrRuntimeStatus;
      next_step: FirstRunNextStep;
      permission_requirements: FirstRunPermissionRequirement[];
      microphone_permission_ready: boolean;
      input_permission_ready: boolean;
      hotkey_registered: boolean;
      hotkey_registration_error: string | null;
      setup_timing: FirstRunSetupTiming;
      first_dictation_completed: boolean;
    };
  };
}

type HistoryStage = "capture" | "vad" | "recognize" | "clean" | "inject" | "history";
type InjectMethod = "native" | "keystroke" | "clipboard_restore";
type HoldReason = "focus_changed" | "secure_field" | "no_target";

interface HistoryFailure {
  stage: HistoryStage;
  error: string;
}

interface HistorySession {
  id: string;
  started_ms: number | null;
  target_app: AppRef | null;
  audio_path: string | null;
  raw_text: string | null;
  clean_text: string | null;
  cleanup_dial: "raw" | "light" | "full" | null;
  injected_method: InjectMethod | null;
  held_reason: HoldReason | null;
  failure: HistoryFailure | null;
  event_count: number;
}

interface HistoryExportOutcome {
  exported: boolean;
  json_path: string | null;
  text_path: string | null;
}

interface HistoryPurgeOutcome {
  sessions_deleted: number;
  audio_files_removed: number;
  export_files_removed: number;
}

interface HistoryAudioPlayback {
  asset_path: string;
  mime_type: string;
  byte_length: number;
}

interface ActiveHistoryAudio {
  sessionId: string;
  src: string;
  mimeType: string;
  byteLength: number;
}

const previewSnapshot: AppSnapshot = {
  app_name: "Kaydence",
  app_identifier: "io.kaydence.app",
  settings: {
    hotkey: {
      mode: "push_to_talk",
      primary_binding: "RightAlt",
      primary_binding_options: [
        {
          id: "RightAlt",
          label: "Right Alt / Option",
          detail: "Default hold key, best when the OS accepts the right-side modifier.",
        },
        {
          id: "F13",
          label: "F13",
          detail: "Dedicated function-key fallback for extended keyboards.",
        },
        {
          id: "F14",
          label: "F14",
          detail: "Second dedicated function-key fallback for extended keyboards.",
        },
        {
          id: "Control+Space",
          label: "Control Space",
          detail: "Chord fallback for compact keyboards without F13/F14.",
        },
        {
          id: "Shift+F13",
          label: "Shift F13",
          detail: "Conflict-escape chord when a plain function key is already taken.",
        },
      ],
      secondary_dial_override_binding: "Shift+RightAlt",
    },
    capture: {
      min_capture_ms: 250,
      tail_buffer_ms: 300,
      debounce_ms: 30,
    },
    engine: {
      default_local_asr: "parakeet_cpu",
      auto_recommend_by_hardware: true,
    },
    cleanup: {
      default_dial: "light",
      full_requires_explicit_opt_in: true,
    },
    injection: {
      unknown_focus_policy: "warn_and_allow",
      prefer_clipboard_fallback: false,
    },
    privacy: {
      history_retention_days: 30,
      local_context_enabled: false,
      local_ocr_enabled: false,
    },
    first_run: {
      model_ready: false,
      model_readiness_error: null,
      required_models: [],
      asr_candidates: [],
      recommended_asr_model_id: null,
      selected_asr_model_id: null,
      asr_runtime: {
        state: "pending",
        selected_model_id: null,
        lane: null,
        runtime: null,
        artifact_path: null,
        artifact_size_bytes: null,
        adapter_ready: false,
        detail: "Select and verify a local ASR model before runtime load.",
        proof_requirement:
          "A real ASR adapter must load a verified artifact and emit transcript events before first dictation can be claimed.",
      },
      next_step: {
        kind: "model_metadata",
        target_id: null,
        title: "Resolve model readiness",
        detail: "Model readiness has not been proven yet.",
        action_label: "Review models",
        proof_requirement: "Refresh model readiness with verified ASR and VAD artifacts.",
      },
      permission_requirements: [
        {
          id: "microphone",
          label: "Microphone",
          state: "needs_hardware",
          detail: "Required before local capture can produce speech audio.",
          action: "Grant Kaydence access in System Settings -> Privacy & Security -> Microphone.",
          action_label: "Show microphone step",
        },
        {
          id: "accessibility",
          label: "Accessibility",
          state: "needs_hardware",
          detail: "Required for native insertion plus focus and secure-field checks.",
          action: "Enable Kaydence in System Settings -> Privacy & Security -> Accessibility.",
          action_label: "Show accessibility step",
        },
        {
          id: "input_monitoring",
          label: "Input Monitoring",
          state: "needs_hardware",
          detail: "Required for the global hotkey monitoring path on macOS.",
          action: "Enable Kaydence in System Settings -> Privacy & Security -> Input Monitoring.",
          action_label: "Show input step",
        },
      ],
      microphone_permission_ready: false,
      input_permission_ready: false,
      hotkey_registered: false,
      hotkey_registration_error: null,
      setup_timing: {
        started_at_ms: null,
        completed_at_ms: null,
        elapsed_ms: null,
        target_ms: 60000,
        within_target: null,
      },
      first_dictation_completed: false,
    },
  },
};

const previewHistory: HistorySession[] = [];

const lanes: LaneSpec[] = [
  {
    id: "mac",
    label: "macOS",
    accent: "#14a7a1",
    injection: "AX native insert",
  },
  {
    id: "windows",
    label: "Windows",
    accent: "#2f72f2",
    injection: "UI Automation + SendInput",
  },
  {
    id: "linux",
    label: "Linux",
    accent: "#1f9d63",
    injection: "AT-SPI detect + uinput",
  },
];

const navItems: NavItem[] = [
  { label: "Dictate", phase: "P1" },
  { label: "Whisper-Ahead", phase: "P3" },
  { label: "Cleanup", phase: "P2" },
  { label: "Relay", phase: "P4" },
  { label: "Voiceprint", phase: "P4" },
  { label: "Conductor", phase: "P4" },
  { label: "Privacy", phase: "P1" },
  { label: "Dictionary", phase: "P2" },
  { label: "Analytics", phase: "P3" },
  { label: "Setup", phase: "P1" },
];

const metrics: Metric[] = [
  { label: "Raw latency", value: "ready", detail: "bench gate wired" },
  { label: "Crash recovery", value: "33/33", detail: "local tests green" },
  { label: "Network audit", value: "0", detail: "unreviewed call sites" },
  { label: "Remote", value: "live", detail: "private origin restored" },
];

function privacyPosture(snapshot: AppSnapshot, lane: LaneSpec): PrivacyPostureItem[] {
  const contextState = snapshot.settings.privacy.local_context_enabled
    ? "Opt-in local"
    : "Off by default";
  const ocrState = snapshot.settings.privacy.local_ocr_enabled ? "local OCR on" : "OCR off";

  return [
    {
      label: "Telemetry",
      value: "None",
      detail: "No usage beacon, account event stream, or hidden crash upload path.",
    },
    {
      label: "Network",
      value: "0 unreviewed",
      detail: "audit-network is the source gate for every new egress-capable call site.",
    },
    {
      label: "Screen capture",
      value: "Banned",
      detail: "Cloud screenshots and screen streams are permanently out of scope.",
    },
    {
      label: "Context",
      value: contextState,
      detail: `${ocrState}; held in memory only and blocked for secure fields.`,
    },
    {
      label: "History",
      value: `${snapshot.settings.privacy.history_retention_days} days`,
      detail: "Audio, raw text, cleaned text, playback, export, delete, and purge stay local.",
    },
    {
      label: `${lane.label} secure fields`,
      value: "Refuse",
      detail: `${lane.injection} must hold instead of injecting when the target is secure.`,
    },
  ];
}

function firstRunChecklist(snapshot: AppSnapshot): ChecklistItem[] {
  const firstRun = snapshot.settings.first_run;
  const permissionsReady = permissionRequirementsReady(firstRun.permission_requirements);

  return [
    {
      label: "Choose local ASR engine",
      status: firstRun.model_ready
        ? "Ready"
        : firstRun.model_readiness_error
          ? "Issue"
          : "Next",
      detail: firstRun.model_readiness_error ?? undefined,
    },
    {
      label: "Grant OS permissions",
      status: permissionsReady ? "Ready" : "Needs hardware",
      detail: permissionsReady ? undefined : "Review runtime requirements below.",
    },
    {
      label: "Set global hotkey",
      status: firstRun.hotkey_registered
        ? "Ready"
        : firstRun.hotkey_registration_error
          ? "Issue"
          : "Next",
      detail: firstRun.hotkey_registration_error ?? undefined,
    },
    { label: "Confirm privacy defaults", status: "Ready" },
    {
      label: "Start first dictation",
      status: firstRun.first_dictation_completed ? "Ready" : "Next",
    },
  ];
}

function setupTimingLabel(timing: FirstRunSetupTiming): string {
  if (timing.within_target === true && timing.elapsed_ms !== null) {
    return `Complete in ${formatDuration(timing.elapsed_ms)}`;
  }
  if (timing.within_target === false && timing.elapsed_ms !== null) {
    return `${formatDuration(timing.elapsed_ms)} total`;
  }
  if (timing.completed_at_ms !== null) {
    return "Completion recorded";
  }
  if (timing.started_at_ms !== null) {
    return "Timer armed";
  }
  return "Waiting for app data";
}

function setupTimingDetail(timing: FirstRunSetupTiming): string {
  const target = formatDuration(timing.target_ms);
  if (timing.within_target === true) {
    return `Inside ${target} target.`;
  }
  if (timing.within_target === false) {
    return `Over ${target} target; keep proof visible.`;
  }
  if (timing.completed_at_ms !== null) {
    return "Start time unavailable; keep proof visible.";
  }
  if (timing.started_at_ms !== null) {
    return `${target} target armed for the first injected dictation.`;
  }
  return `${target} target starts when the desktop app opens with app data.`;
}

function asrRuntimeStateLabel(runtime: FirstRunAsrRuntimeStatus): string {
  if (runtime.adapter_ready) {
    return "Adapter ready";
  }
  switch (runtime.state) {
    case "pending":
      return "Waiting";
    case "blocked":
      return "Blocked";
    case "verified_artifact":
      return "Artifact ready";
  }
}

function asrRuntimeArtifactDetail(runtime: FirstRunAsrRuntimeStatus): string | null {
  if (!runtime.artifact_path) {
    return null;
  }
  if (runtime.artifact_size_bytes === null) {
    return runtime.artifact_path;
  }
  const sizeKb = Math.max(1, Math.ceil(runtime.artifact_size_bytes / 1024));
  return `${runtime.artifact_path} (${sizeKb} KB)`;
}

function formatDuration(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / 100) / 10);
  return `${seconds.toFixed(seconds % 1 === 0 ? 0 : 1)}s`;
}

function permissionStateLabel(state: FirstRunPermissionState): string {
  switch (state) {
    case "ready":
      return "Ready";
    case "needs_hardware":
      return "Needs hardware";
    case "needs_review":
      return "Needs review";
    case "blocked":
      return "Blocked";
  }
}

function permissionSummary(requirements: FirstRunPermissionRequirement[]): string {
  if (requirements.length === 0) {
    return "No runtime contract";
  }
  return requirements.map((requirement) => requirement.label).join(" / ");
}

function permissionRequirementsReady(requirements: FirstRunPermissionRequirement[]): boolean {
  return (
    requirements.length > 0 && requirements.every((requirement) => requirement.state === "ready")
  );
}

function statusClassName(status: string): string {
  return status.toLowerCase().replace(/\s+/g, "-");
}

function historyStatus(session: HistorySession): string {
  if (session.failure) {
    return `Failed: ${session.failure.stage}`;
  }
  if (session.held_reason) {
    return `Held: ${session.held_reason}`;
  }
  if (session.injected_method) {
    return `Injected: ${session.injected_method}`;
  }
  if (session.audio_path) {
    return "Audio saved";
  }
  return "Started";
}

function historySummary(session: HistorySession): string {
  return (
    session.clean_text ??
    session.raw_text ??
    session.failure?.error ??
    session.audio_path ??
    "Session opened"
  );
}

// Presentation only. The Rust backend owns all logic (root AGENTS §9).
export function App(): JSX.Element {
  const [activeLane, setActiveLane] = useState<OsLane>("mac");
  const [snapshot, setSnapshot] = useState<AppSnapshot>(previewSnapshot);
  const [snapshotSource, setSnapshotSource] = useState<"backend" | "preview">("preview");
  const [historySessions, setHistorySessions] = useState<HistorySession[]>(previewHistory);
  const [historyRefreshPending, setHistoryRefreshPending] = useState(false);
  const [deletingHistoryId, setDeletingHistoryId] = useState<string | null>(null);
  const [exportingHistoryId, setExportingHistoryId] = useState<string | null>(null);
  const [historyExportOutcome, setHistoryExportOutcome] =
    useState<HistoryExportOutcome | null>(null);
  const [historyPurgeOutcome, setHistoryPurgeOutcome] =
    useState<HistoryPurgeOutcome | null>(null);
  const [purgingHistory, setPurgingHistory] = useState(false);
  const [loadingHistoryAudioId, setLoadingHistoryAudioId] = useState<string | null>(null);
  const [activeHistoryAudio, setActiveHistoryAudio] =
    useState<ActiveHistoryAudio | null>(null);
  const [historyPlaybackIssue, setHistoryPlaybackIssue] = useState<string | null>(null);
  const [setupRefreshPending, setSetupRefreshPending] = useState(false);
  const [setupRefreshIssue, setSetupRefreshIssue] = useState<string | null>(null);
  const [modelRefreshPending, setModelRefreshPending] = useState(false);
  const [installingModelId, setInstallingModelId] = useState<string | null>(null);
  const [modelInstallIssue, setModelInstallIssue] = useState<string | null>(null);
  const [modelPreflightPendingId, setModelPreflightPendingId] = useState<string | null>(null);
  const [modelDownloadPreflight, setModelDownloadPreflight] =
    useState<FirstRunModelDownloadPreflight | null>(null);
  const [modelDownloadPreflightIssue, setModelDownloadPreflightIssue] =
    useState<string | null>(null);
  const [hotkeyModePending, setHotkeyModePending] = useState<HotkeyMode | null>(null);
  const [hotkeyModeIssue, setHotkeyModeIssue] = useState<string | null>(null);
  const [hotkeyBindingPending, setHotkeyBindingPending] = useState<string | null>(null);
  const [hotkeyBindingIssue, setHotkeyBindingIssue] = useState<string | null>(null);
  const [permissionActionPendingId, setPermissionActionPendingId] = useState<string | null>(null);
  const [permissionActionOutcome, setPermissionActionOutcome] =
    useState<FirstRunPermissionActionOutcome | null>(null);
  const [permissionActionIssue, setPermissionActionIssue] = useState<string | null>(null);
  const [permissionSettingsPendingId, setPermissionSettingsPendingId] = useState<string | null>(
    null,
  );
  const [permissionSettingsOutcome, setPermissionSettingsOutcome] =
    useState<FirstRunPermissionSettingsOpenOutcome | null>(null);
  const [permissionSettingsIssue, setPermissionSettingsIssue] = useState<string | null>(null);
  const [firstRunActionNote, setFirstRunActionNote] = useState<string | null>(null);
  const [exportingFirstRunProof, setExportingFirstRunProof] = useState(false);
  const [firstRunProofExport, setFirstRunProofExport] =
    useState<FirstRunProofExportOutcome | null>(null);
  const [firstRunProofIssue, setFirstRunProofIssue] = useState<string | null>(null);
  const lane = useMemo(
    () => lanes.find((candidate) => candidate.id === activeLane) ?? lanes[0],
    [activeLane],
  );
  const hotkeyMode =
    snapshot.settings.hotkey.mode === "push_to_talk" ? "Push-to-talk" : "Toggle";
  const hotkeyBindingOptions = snapshot.settings.hotkey.primary_binding_options;
  const cleanupDefault = snapshot.settings.cleanup.default_dial;
  const asrCandidates = snapshot.settings.first_run.asr_candidates;
  const selectedAsr = asrCandidates.find((candidate) => candidate.selected);
  const asrRuntime = snapshot.settings.first_run.asr_runtime;
  const asrRuntimeArtifact = asrRuntimeArtifactDetail(asrRuntime);
  const engineLabel =
    selectedAsr?.id ??
    (snapshot.settings.engine.default_local_asr === "parakeet_cpu"
      ? "Parakeet CPU"
      : "Whisper GPU");
  const unknownFocus =
    snapshot.settings.injection.unknown_focus_policy === "warn_and_allow"
      ? "Warn on opaque focus"
      : "Refuse opaque focus";
  const privacyLabel = snapshot.settings.privacy.local_context_enabled
    ? "Local context on"
    : "Local context off";
  const checklist = firstRunChecklist(snapshot);
  const permissionRequirements = snapshot.settings.first_run.permission_requirements;
  const permissionSummaryText = permissionSummary(permissionRequirements);
  const setupTiming = snapshot.settings.first_run.setup_timing;
  const nextStep = snapshot.settings.first_run.next_step;
  const firstRunActionDisabled =
    nextStep.kind === "setup" ||
    ((nextStep.kind === "model_install" || nextStep.kind === "permission") &&
      nextStep.target_id === null) ||
    installingModelId !== null ||
    permissionActionPendingId !== null ||
    setupRefreshPending ||
    modelRefreshPending ||
    modelPreflightPendingId !== null;
  const firstRunProofExportLabel = firstRunProofExport?.exported
    ? `${firstRunProofExport.item_count} proof items`
    : "Build-agent handoff";
  const requiredModels = snapshot.settings.first_run.required_models;
  const showModelRecheck =
    !snapshot.settings.first_run.model_ready &&
    (requiredModels.length > 0 ||
      snapshot.settings.first_run.model_readiness_error !== null);
  const privacyItems = privacyPosture(snapshot, lane);

  useEffect(() => {
    let active = true;
    void invoke<AppSnapshot>("app_snapshot")
      .then((nextSnapshot) => {
        if (active) {
          setSnapshot(nextSnapshot);
          setSnapshotSource("backend");
        }
      })
      .catch(() => {
        if (active) {
          setSnapshot(previewSnapshot);
          setSnapshotSource("preview");
        }
      });
    void invoke<HistorySession[]>("recent_history", { limit: 4 })
      .then((sessions) => {
        if (active) {
          setHistorySessions(sessions);
        }
      })
      .catch(() => {
        if (active) {
          setHistorySessions(previewHistory);
        }
      });
    return () => {
      active = false;
    };
  }, []);

  function refreshSetupSnapshot() {
    setSetupRefreshPending(true);
    setSetupRefreshIssue(null);

    if (snapshotSource === "preview") {
      setSetupRefreshIssue("Open the desktop runtime to refresh first-run proof state.");
      setSetupRefreshPending(false);
      return;
    }

    void invoke<AppSnapshot>("refresh_first_run_runtime_proofs")
      .then((nextSnapshot) => {
        setSnapshot(nextSnapshot);
        setSnapshotSource("backend");
      })
      .catch((error) => {
        console.error("Kaydence setup snapshot refresh failed", error);
        setSetupRefreshIssue("First-run proof state could not be refreshed.");
      })
      .finally(() => {
        setSetupRefreshPending(false);
      });
  }

  function selectAsrModel(modelId: string) {
    void invoke<AppSnapshot>("select_asr_model", { modelId })
      .then((nextSnapshot) => {
        setSnapshot(nextSnapshot);
        setSnapshotSource("backend");
      })
      .catch((error) => {
        console.error("Kaydence ASR selection failed", error);
      });
  }

  function setHotkeyMode(mode: HotkeyMode) {
    setHotkeyModePending(mode);
    setHotkeyModeIssue(null);
    void invoke<AppSnapshot>("set_hotkey_mode", { mode })
      .then((nextSnapshot) => {
        setSnapshot(nextSnapshot);
        setSnapshotSource("backend");
      })
      .catch((error) => {
        console.error("Kaydence hotkey mode update failed", error);
        setHotkeyModeIssue("Hotkey mode can be changed when capture is idle.");
      })
      .finally(() => {
        setHotkeyModePending(null);
      });
  }

  function setHotkeyBinding(binding: string) {
    setHotkeyBindingPending(binding);
    setHotkeyBindingIssue(null);
    void invoke<AppSnapshot>("set_hotkey_binding", { binding })
      .then((nextSnapshot) => {
        setSnapshot(nextSnapshot);
        setSnapshotSource("backend");
      })
      .catch((error) => {
        console.error("Kaydence hotkey binding update failed", error);
        setHotkeyBindingIssue("Hotkey binding could not register. Try another option when capture is idle.");
      })
      .finally(() => {
        setHotkeyBindingPending(null);
      });
  }

  function refreshModelReadiness() {
    setModelRefreshPending(true);
    setModelInstallIssue(null);
    setModelDownloadPreflightIssue(null);
    void invoke<AppSnapshot>("refresh_model_readiness")
      .then((nextSnapshot) => {
        setSnapshot(nextSnapshot);
        setSnapshotSource("backend");
      })
      .catch((error) => {
        console.error("Kaydence model readiness refresh failed", error);
      })
      .finally(() => {
        setModelRefreshPending(false);
      });
  }

  function reviewModelDownload(modelId: string) {
    setModelPreflightPendingId(modelId);
    setModelDownloadPreflight(null);
    setModelDownloadPreflightIssue(null);

    if (snapshotSource === "preview") {
      setModelDownloadPreflightIssue("Open the desktop runtime to review model download metadata.");
      setModelPreflightPendingId(null);
      return;
    }

    void invoke<FirstRunModelDownloadPreflight>("first_run_model_download_preflight", {
      modelId,
    })
      .then((preflight) => {
        setModelDownloadPreflight(preflight);
      })
      .catch((error) => {
        console.error("Kaydence model download preflight failed", error);
        setModelDownloadPreflightIssue("Model download metadata could not be reviewed.");
      })
      .finally(() => {
        setModelPreflightPendingId(null);
      });
  }

  function showPermissionAction(requirementId: string) {
    setPermissionActionPendingId(requirementId);
    setPermissionActionIssue(null);
    setPermissionActionOutcome(null);
    setPermissionSettingsIssue(null);
    setPermissionSettingsOutcome(null);

    if (snapshotSource === "preview") {
      setPermissionActionIssue("Open the desktop runtime to load permission proof guidance.");
      setPermissionActionPendingId(null);
      return;
    }

    void invoke<FirstRunPermissionActionOutcome>("first_run_permission_action", {
      requirementId,
    })
      .then((outcome) => {
        setPermissionActionOutcome(outcome);
      })
      .catch((error) => {
        console.error("Kaydence permission action failed", error);
        setPermissionActionIssue("Permission guidance is unavailable for this requirement.");
      })
      .finally(() => {
        setPermissionActionPendingId(null);
      });
  }

  function openPermissionSettings(requirementId: string) {
    setPermissionSettingsPendingId(requirementId);
    setPermissionSettingsIssue(null);
    setPermissionSettingsOutcome(null);

    if (snapshotSource === "preview") {
      setPermissionSettingsIssue("Open the desktop runtime to open OS settings.");
      setPermissionSettingsPendingId(null);
      return;
    }

    void invoke<FirstRunPermissionSettingsOpenOutcome>("open_first_run_permission_settings", {
      requirementId,
    })
      .then((outcome) => {
        setPermissionSettingsOutcome(outcome);
      })
      .catch((error) => {
        console.error("Kaydence permission settings open failed", error);
        setPermissionSettingsIssue("OS settings could not be opened from Kaydence.");
      })
      .finally(() => {
        setPermissionSettingsPendingId(null);
      });
  }

  async function installModelArtifact(modelId: string) {
    setInstallingModelId(modelId);
    setModelInstallIssue(null);
    setModelDownloadPreflightIssue(null);
    try {
      const selected = await openDialog({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Model artifact",
            extensions: ["onnx", "bin", "gguf"],
          },
        ],
      });
      if (selected === null || Array.isArray(selected)) {
        return;
      }
      const nextSnapshot = await invoke<AppSnapshot>("install_model_artifact", {
        modelId,
        sourcePath: selected,
      });
      setSnapshot(nextSnapshot);
      setSnapshotSource("backend");
    } catch (error) {
      console.error("Kaydence model artifact install failed", error);
      setModelInstallIssue("Model artifact was not installed.");
    } finally {
      setInstallingModelId(null);
    }
  }

  async function handleFirstRunAction() {
    setFirstRunActionNote(null);

    if (snapshotSource === "preview") {
      setFirstRunActionNote(nextStep.proof_requirement);
      return;
    }

    if (nextStep.kind === "permission" && nextStep.target_id) {
      showPermissionAction(nextStep.target_id);
      return;
    }

    if (nextStep.kind === "model_install" && nextStep.target_id) {
      await installModelArtifact(nextStep.target_id);
      return;
    }

    if (nextStep.kind === "model_metadata") {
      setFirstRunActionNote(nextStep.proof_requirement);
      if (nextStep.target_id) {
        reviewModelDownload(nextStep.target_id);
      }
      refreshModelReadiness();
      return;
    }

    if (nextStep.kind === "hotkey") {
      setFirstRunActionNote(nextStep.proof_requirement);
      setHotkeyBindingIssue(nextStep.proof_requirement);
      return;
    }

    if (nextStep.kind === "dictation") {
      setFirstRunActionNote(nextStep.proof_requirement);
      return;
    }

    if (nextStep.kind === "complete") {
      setFirstRunActionNote(nextStep.proof_requirement);
    }
  }

  function exportFirstRunProofPlan() {
    setFirstRunProofExport(null);
    setFirstRunProofIssue(null);

    if (snapshotSource === "preview") {
      setFirstRunProofIssue("Open the desktop runtime to write the local proof JSON.");
      return;
    }

    setExportingFirstRunProof(true);
    void invoke<FirstRunProofExportOutcome>("export_first_run_proof_plan")
      .then((outcome) => {
        setFirstRunProofExport(outcome);
      })
      .catch((error) => {
        console.error("Kaydence first-run proof export failed", error);
        setFirstRunProofIssue("First-run proof export failed.");
      })
      .finally(() => {
        setExportingFirstRunProof(false);
      });
  }

  function refreshHistory() {
    setHistoryRefreshPending(true);
    void invoke<HistorySession[]>("recent_history", { limit: 4 })
      .then((sessions) => {
        setHistorySessions(sessions);
        setHistoryPurgeOutcome(null);
        setHistoryPlaybackIssue(null);
        setActiveHistoryAudio((current) =>
          current && sessions.some((session) => session.id === current.sessionId)
            ? current
            : null,
        );
      })
      .catch((error) => {
        console.error("Kaydence history refresh failed", error);
      })
      .finally(() => {
        setHistoryRefreshPending(false);
      });
  }

  function deleteHistorySession(sessionId: string) {
    const confirmed = window.confirm(
      "Delete this local history session and its audio file?",
    );
    if (!confirmed) {
      return;
    }

    setDeletingHistoryId(sessionId);
    void invoke<HistorySession[]>("delete_history_session", { sessionId })
      .then((sessions) => {
        setHistorySessions(sessions);
        setHistoryExportOutcome(null);
        setActiveHistoryAudio((current) =>
          current?.sessionId === sessionId ? null : current,
        );
      })
      .catch((error) => {
        console.error("Kaydence history delete failed", error);
      })
      .finally(() => {
        setDeletingHistoryId(null);
      });
  }

  function exportHistorySession(sessionId: string) {
    setExportingHistoryId(sessionId);
    setHistoryExportOutcome(null);
    setHistoryPurgeOutcome(null);
    setHistoryPlaybackIssue(null);
    void invoke<HistoryExportOutcome>("export_history_session", { sessionId })
      .then((outcome) => {
        setHistoryExportOutcome(outcome);
      })
      .catch((error) => {
        console.error("Kaydence history export failed", error);
      })
      .finally(() => {
        setExportingHistoryId(null);
      });
  }

  function playHistoryAudio(sessionId: string) {
    setLoadingHistoryAudioId(sessionId);
    setHistoryPlaybackIssue(null);
    void invoke<HistoryAudioPlayback | null>("play_history_audio", { sessionId })
      .then((playback) => {
        if (!playback) {
          setActiveHistoryAudio(null);
          setHistoryPlaybackIssue("No safe local audio is available for playback.");
          return;
        }
        setActiveHistoryAudio({
          sessionId,
          src: convertFileSrc(playback.asset_path),
          mimeType: playback.mime_type,
          byteLength: playback.byte_length,
        });
      })
      .catch((error) => {
        console.error("Kaydence history audio playback failed", error);
        setHistoryPlaybackIssue("Audio playback could not be prepared.");
      })
      .finally(() => {
        setLoadingHistoryAudioId(null);
      });
  }

  function purgeHistory() {
    const confirmed = window.confirm(
      "Purge all local history sessions, audio, and exports from this device?",
    );
    if (!confirmed) {
      return;
    }

    setPurgingHistory(true);
    setHistoryExportOutcome(null);
    setHistoryPurgeOutcome(null);
    setActiveHistoryAudio(null);
    setHistoryPlaybackIssue(null);
    void invoke<HistoryPurgeOutcome>("purge_history")
      .then((outcome) => {
        setHistoryPurgeOutcome(outcome);
        setHistorySessions([]);
      })
      .catch((error) => {
        console.error("Kaydence history purge failed", error);
      })
      .finally(() => {
        setPurgingHistory(false);
      });
  }

  return (
    <main
      className="app-shell"
      style={{ "--accent": lane.accent } as React.CSSProperties}
    >
      <aside className="sidebar" aria-label={`${snapshot.app_name} navigation`}>
        <div className="brand-lockup">
          <img className="brand-mark" src={markUrl} alt="" />
          <div>
            <strong>{snapshot.app_name}</strong>
            <span>Local voice platform</span>
          </div>
        </div>

        <nav className="nav-list">
          {navItems.map((item) => (
            <button
              className={item.label === "Dictate" ? "nav-item active" : "nav-item"}
              key={item.label}
              type="button"
            >
              <span>{item.label}</span>
              <small>{item.phase}</small>
            </button>
          ))}
        </nav>

        <div className="sidebar-card">
          <span className="status-dot" aria-hidden="true" />
          <div>
            <strong>Local only</strong>
            <p>
              No telemetry. {snapshot.settings.privacy.history_retention_days}-day local
              history. {privacyLabel}.
            </p>
          </div>
        </div>
      </aside>

      <section className="workspace" aria-label={`${snapshot.app_name} cockpit`}>
        <header className="topbar">
          <div>
            <p className="eyebrow">P1 recovery build</p>
            <h1>Dictation cockpit</h1>
            <span className="snapshot-source">{snapshotSource} config</span>
          </div>
          <div className="lane-switcher" aria-label="Operating system lane">
            {lanes.map((candidate) => (
              <button
                aria-pressed={candidate.id === lane.id}
                className={candidate.id === lane.id ? "lane active" : "lane"}
                key={candidate.id}
                onClick={() => setActiveLane(candidate.id)}
                type="button"
              >
                {candidate.label}
              </button>
            ))}
          </div>
        </header>

        <section className="metric-grid" aria-label="Current build evidence">
          {metrics.map((metric) => (
            <article className="metric-card" key={metric.label}>
              <span>{metric.label}</span>
              <strong>{metric.value}</strong>
              <p>{metric.detail}</p>
            </article>
          ))}
        </section>

        <section className="content-grid">
          <article className="panel recording-panel">
            <div className="panel-header">
              <div>
                <p className="eyebrow">Screen family 01</p>
                <h2>Main dictation</h2>
              </div>
              <span className="pill">Recording</span>
            </div>

            <div className="waveform" aria-label="Mock recording waveform">
              {Array.from({ length: 34 }, (_, index) => (
                <span key={index} style={{ height: `${20 + (index % 7) * 9}px` }} />
              ))}
            </div>

            <div className="recording-meta">
              <div>
                <span>Elapsed</span>
                <strong>00:12.4</strong>
              </div>
              <div>
                <span>Engine</span>
                <strong>{engineLabel}</strong>
              </div>
              <div>
                <span>Hotkey</span>
                <strong>{snapshot.settings.hotkey.primary_binding}</strong>
              </div>
              <div>
                <span>ASR runtime</span>
                <strong>{asrRuntimeStateLabel(asrRuntime)}</strong>
              </div>
            </div>

            <div className="transcript-card">
              <span>Recent clean transcript</span>
              <p>
                Refactor the auth module and then migrate the session guard into
                the shared middleware.
              </p>
            </div>

            <div className="history-strip" aria-label="Recent local history">
              <div className="history-strip-heading">
                <div>
                  <span>Local history</span>
                  <strong>{historySessions.length} recent sessions</strong>
                </div>
                <div className="history-header-actions">
                  <button
                    className="mini-action"
                    disabled={historyRefreshPending || purgingHistory}
                    onClick={refreshHistory}
                    type="button"
                  >
                    {historyRefreshPending ? "Refreshing" : "Refresh"}
                  </button>
                  <button
                    className="danger-action"
                    disabled={purgingHistory || historyRefreshPending || historySessions.length === 0}
                    onClick={purgeHistory}
                    type="button"
                  >
                    {purgingHistory ? "Purging" : "Purge All"}
                  </button>
                </div>
              </div>
              {historyPurgeOutcome ? (
                <p className="history-export-note">
                  Purged {historyPurgeOutcome.sessions_deleted} sessions,{" "}
                  {historyPurgeOutcome.audio_files_removed} audio files, and{" "}
                  {historyPurgeOutcome.export_files_removed} exports.
                </p>
              ) : null}
              {historyExportOutcome ? (
                <p className="history-export-note">
                  {historyExportOutcome.exported
                    ? `Exported to ${historyExportOutcome.text_path ?? historyExportOutcome.json_path}`
                    : "No matching local session to export."}
                </p>
              ) : null}
              {historyPlaybackIssue ? (
                <p className="history-export-note">{historyPlaybackIssue}</p>
              ) : null}
              {historySessions.length > 0 ? (
                <div className="history-list">
                  {historySessions.map((session) => (
                    <div className="history-row" key={session.id}>
                      <div className="history-row-main">
                        <strong>{session.target_app?.name ?? "Local session"}</strong>
                        <span>{historySummary(session)}</span>
                        {activeHistoryAudio?.sessionId === session.id ? (
                          <div className="history-audio-player">
                            <audio
                              aria-label={`Audio for ${session.target_app?.name ?? "local history session"}`}
                              controls
                              preload="metadata"
                            >
                              <source
                                src={activeHistoryAudio.src}
                                type={activeHistoryAudio.mimeType}
                              />
                            </audio>
                            <small>
                              {Math.max(1, Math.ceil(activeHistoryAudio.byteLength / 1024))} KB
                            </small>
                          </div>
                        ) : null}
                      </div>
                      <div className="history-row-actions">
                        <em>{historyStatus(session)}</em>
                        <button
                          aria-label={`Play ${session.target_app?.name ?? "local history session"} audio`}
                          className="mini-action"
                          disabled={
                            !session.audio_path ||
                            loadingHistoryAudioId !== null ||
                            deletingHistoryId !== null ||
                            purgingHistory
                          }
                          onClick={() => playHistoryAudio(session.id)}
                          type="button"
                        >
                          {loadingHistoryAudioId === session.id ? "Loading" : "Play"}
                        </button>
                        <button
                          aria-label={`Export ${session.target_app?.name ?? "local history session"}`}
                          className="mini-action"
                          disabled={
                            exportingHistoryId !== null ||
                            deletingHistoryId !== null ||
                            purgingHistory
                          }
                          onClick={() => exportHistorySession(session.id)}
                          type="button"
                        >
                          {exportingHistoryId === session.id ? "Exporting" : "Export"}
                        </button>
                        <button
                          aria-label={`Delete ${session.target_app?.name ?? "local history session"}`}
                          className="danger-action"
                          disabled={
                            deletingHistoryId !== null ||
                            exportingHistoryId !== null ||
                            purgingHistory
                          }
                          onClick={() => deleteHistorySession(session.id)}
                          type="button"
                        >
                          {deletingHistoryId === session.id ? "Deleting" : "Delete"}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="empty-history">No local sessions recorded yet.</p>
              )}
            </div>
          </article>

          <article className="panel">
            <div className="panel-header">
              <div>
                <p className="eyebrow">Cleanup dial</p>
                <h2>Output rules</h2>
              </div>
            </div>
            <div className="segmented" role="group" aria-label="Cleanup level">
              <button className={cleanupDefault === "raw" ? "selected" : ""} type="button">
                Raw
              </button>
              <button className={cleanupDefault === "light" ? "selected" : ""} type="button">
                Light
              </button>
              <button className={cleanupDefault === "full" ? "selected" : ""} type="button">
                Full
              </button>
            </div>
            <div className="segmented hotkey-mode-control" role="group" aria-label="Hotkey mode">
              <button
                className={snapshot.settings.hotkey.mode === "push_to_talk" ? "selected" : ""}
                disabled={hotkeyModePending !== null}
                onClick={() => setHotkeyMode("push_to_talk")}
                type="button"
              >
                Hold
              </button>
              <button
                className={snapshot.settings.hotkey.mode === "toggle" ? "selected" : ""}
                disabled={hotkeyModePending !== null}
                onClick={() => setHotkeyMode("toggle")}
                type="button"
              >
                Toggle
              </button>
            </div>
            {hotkeyModeIssue ? (
              <p className="hotkey-mode-note">{hotkeyModeIssue}</p>
            ) : null}
            <div className="hotkey-binding-grid" role="group" aria-label="Hotkey binding">
              {hotkeyBindingOptions.map((option) => (
                <button
                  aria-pressed={snapshot.settings.hotkey.primary_binding === option.id}
                  className={
                    snapshot.settings.hotkey.primary_binding === option.id
                      ? "hotkey-binding-option selected"
                      : "hotkey-binding-option"
                  }
                  disabled={hotkeyBindingPending !== null}
                  key={option.id}
                  onClick={() => setHotkeyBinding(option.id)}
                  type="button"
                >
                  <span>{option.label}</span>
                  <small>{option.detail}</small>
                </button>
              ))}
            </div>
            {hotkeyBindingIssue ? (
              <p className="hotkey-mode-note">{hotkeyBindingIssue}</p>
            ) : null}
            <ul className="rule-list">
              <li>{hotkeyMode} on {snapshot.settings.hotkey.primary_binding}</li>
              <li>Filler removal enabled</li>
              <li>Self-correction collapse enabled</li>
              <li>Full rewrite requires opt-in</li>
              <li>Secure fields always refuse injection</li>
            </ul>
          </article>

          <article className="panel">
            <div className="panel-header">
              <div>
                <p className="eyebrow">Screen family 03</p>
                <h2>{lane.label} injection</h2>
              </div>
              <span className="pill subtle">{lane.injection}</span>
            </div>
            <div className="capability-list">
              <div>
                <span>Primary path</span>
                <strong>{lane.injection}</strong>
              </div>
              <div>
                <span>Runtime permissions</span>
                <strong>{permissionSummaryText}</strong>
              </div>
              <div>
                <span>Unknown focus</span>
                <strong>{unknownFocus}</strong>
              </div>
            </div>
          </article>

          <article className="panel privacy-panel">
            <div className="panel-header">
              <div>
                <p className="eyebrow">Screen family 07</p>
                <h2>Privacy & context</h2>
              </div>
              <span className="pill subtle">Local only</span>
            </div>
            <div className="privacy-grid" aria-label="Privacy posture">
              {privacyItems.map((item) => (
                <div className="privacy-item" key={item.label}>
                  <span>{item.label}</span>
                  <strong>{item.value}</strong>
                  <small>{item.detail}</small>
                </div>
              ))}
            </div>
          </article>

          <article className="panel setup-panel">
            <div className="panel-header">
              <div>
                <p className="eyebrow">Screen family 10</p>
                <h2>First run</h2>
              </div>
              <div className="setup-header-actions">
                <button
                  className="mini-action"
                  disabled={setupRefreshPending}
                  onClick={refreshSetupSnapshot}
                  type="button"
                >
                  {setupRefreshPending ? "Refreshing" : "Refresh"}
                </button>
                <span className="pill">Free selected</span>
              </div>
            </div>
            {setupRefreshIssue ? <p className="hotkey-mode-note">{setupRefreshIssue}</p> : null}
            <div className={`next-step-card step-${nextStep.kind}`} aria-label="First-run next step">
              <div>
                <span>Next step</span>
                <strong>{nextStep.title}</strong>
                <p>{nextStep.detail}</p>
                <small>Proof: {nextStep.proof_requirement}</small>
              </div>
              <button
                className="secondary-action next-step-action"
                disabled={firstRunActionDisabled}
                onClick={() => void handleFirstRunAction()}
                type="button"
              >
                {nextStep.action_label}
              </button>
              {firstRunActionNote ? (
                <small className="next-step-note" role="status">
                  {firstRunActionNote}
                </small>
              ) : null}
            </div>
            <ol className="setup-list">
              {checklist.map((item) => (
                <li className={`setup-item status-${statusClassName(item.status)}`} key={item.label}>
                  <span>
                    {item.label}
                    {item.detail ? <small>{item.detail}</small> : null}
                  </span>
                  <strong>{item.status}</strong>
                </li>
              ))}
            </ol>
            <div className="setup-timing" aria-label="First-run setup timing">
              <span>60-second setup proof</span>
              <strong>{setupTimingLabel(setupTiming)}</strong>
              <small>{setupTimingDetail(setupTiming)}</small>
            </div>
            <div
              className={`asr-runtime-card status-${asrRuntime.state}`}
              aria-label="Selected ASR runtime status"
            >
              <span>Selected ASR runtime</span>
              <strong>{asrRuntimeStateLabel(asrRuntime)}</strong>
              <small>
                {asrRuntime.selected_model_id ?? "No model selected"}
                {asrRuntime.lane ? ` / ${asrRuntime.lane}` : ""}
                {asrRuntime.runtime ? ` / ${asrRuntime.runtime}` : ""}
              </small>
              <small>{asrRuntime.detail}</small>
              {asrRuntimeArtifact ? <small>Artifact: {asrRuntimeArtifact}</small> : null}
              <small>Proof: {asrRuntime.proof_requirement}</small>
            </div>
            <div className="proof-export-card" aria-label="First-run proof export">
              <div>
                <span>Local proof JSON</span>
                <strong>{firstRunProofExportLabel}</strong>
                <small>
                  {firstRunProofExport?.json_path ??
                    "Rust snapshot, next step, permissions, models, hotkey, dictation."}
                </small>
              </div>
              <button
                className="secondary-action proof-export-action"
                disabled={exportingFirstRunProof}
                onClick={exportFirstRunProofPlan}
                type="button"
              >
                {exportingFirstRunProof ? "Exporting" : "Export Proof"}
              </button>
              {firstRunProofIssue ? (
                <small className="proof-export-issue" role="status">
                  {firstRunProofIssue}
                </small>
              ) : null}
            </div>
            {permissionRequirements.length > 0 ? (
              <div className="permission-list" aria-label="OS permission requirements">
                <div className="permission-list-heading">
                  <span>OS permission requirements</span>
                  <strong>Runtime contract</strong>
                </div>
                {permissionRequirements.map((requirement) => (
                  <div
                    className={`permission-item status-${requirement.state}`}
                    key={requirement.id}
                  >
                    <div>
                      <strong>{requirement.label}</strong>
                      <span>{requirement.detail}</span>
                      <small>{requirement.action}</small>
                    </div>
                    <div className="permission-item-actions">
                      <em>{permissionStateLabel(requirement.state)}</em>
                      <button
                        className="mini-action permission-action"
                        disabled={permissionActionPendingId !== null}
                        onClick={() => showPermissionAction(requirement.id)}
                        type="button"
                      >
                        {permissionActionPendingId === requirement.id
                          ? "Loading"
                          : requirement.action_label}
                      </button>
                    </div>
                  </div>
                ))}
                {permissionActionOutcome ? (
                  <div className="permission-action-outcome" role="status">
                    <strong>{permissionActionOutcome.label}</strong>
                    <span>{permissionActionOutcome.manual_step}</span>
                    {permissionActionOutcome.settings_target &&
                    permissionActionOutcome.settings_open_label ? (
                      <div className="permission-settings-open">
                        <button
                          className="mini-action"
                          disabled={permissionSettingsPendingId !== null}
                          onClick={() =>
                            openPermissionSettings(permissionActionOutcome.requirement_id)
                          }
                          type="button"
                        >
                          {permissionSettingsPendingId === permissionActionOutcome.requirement_id
                            ? "Opening"
                            : permissionActionOutcome.settings_open_label}
                        </button>
                        <code>{permissionActionOutcome.settings_target}</code>
                      </div>
                    ) : (
                      <small>Manual-only: no stable OS settings panel for this proof.</small>
                    )}
                    <small>Proof: {permissionActionOutcome.proof_requirement}</small>
                    {permissionActionOutcome.proof_command ? (
                      <code>{permissionActionOutcome.proof_command}</code>
                    ) : null}
                    <small>Evidence: {permissionActionOutcome.expected_evidence}</small>
                    <small>Ready boundary: {permissionActionOutcome.ready_boundary}</small>
                    {permissionSettingsOutcome ? (
                      <small>
                        {permissionSettingsOutcome.opened
                          ? "Settings panel requested. Return here and run the proof before marking ready."
                          : "Use the manual step above; this requirement has no stable settings target."}
                      </small>
                    ) : null}
                    {permissionSettingsIssue ? (
                      <small className="proof-export-issue">{permissionSettingsIssue}</small>
                    ) : null}
                  </div>
                ) : null}
                {permissionActionIssue ? (
                  <p className="model-install-note">{permissionActionIssue}</p>
                ) : null}
              </div>
            ) : null}
            {snapshot.settings.first_run.hotkey_registration_error ? (
              <div className="setup-rebind">
                <span>Try another hotkey</span>
                <div className="hotkey-binding-grid compact" role="group" aria-label="Setup hotkey binding">
                  {hotkeyBindingOptions.map((option) => (
                    <button
                      aria-pressed={snapshot.settings.hotkey.primary_binding === option.id}
                      className={
                        snapshot.settings.hotkey.primary_binding === option.id
                          ? "hotkey-binding-option selected"
                          : "hotkey-binding-option"
                      }
                      disabled={hotkeyBindingPending !== null}
                      key={option.id}
                      onClick={() => setHotkeyBinding(option.id)}
                      type="button"
                    >
                      <span>{option.label}</span>
                    </button>
                  ))}
                </div>
              </div>
            ) : null}
            {asrCandidates.length > 0 ? (
              <div className="model-picker" aria-label="Local ASR model picker">
                <div className="model-picker-heading">
                  <span>Local ASR</span>
                  <strong>{snapshot.settings.first_run.recommended_asr_model_id ?? "none"}</strong>
                </div>
                {asrCandidates.map((model) => (
                  <button
                    aria-pressed={model.selected}
                    className={`model-option status-${model.state}${model.selected ? " selected" : ""}`}
                    key={model.id}
                    onClick={() => selectAsrModel(model.id)}
                    type="button"
                  >
                    <span>
                      <strong>{model.id}</strong>
                      <small>{model.recommendation ?? `${model.runtime} / ${model.min_hw}`}</small>
                      {model.download_available ? (
                        <small>
                          {model.download_size_mb ?? model.size_mb} MB / {model.download_source_count} sources
                        </small>
                      ) : null}
                    </span>
                    <em>{model.state}</em>
                  </button>
                ))}
              </div>
            ) : null}
            {requiredModels.length > 0 ? (
              <div className="model-status-list" aria-label="Required local models">
                {requiredModels.map((model) => (
                  <div className={`model-status status-${model.state}`} key={model.id}>
                    <div>
                      <strong>{model.id}</strong>
                      <span>{model.task}{model.lane ? ` / ${model.lane}` : ""}</span>
                    </div>
                    <div>
                      <em>{model.state}</em>
                      <small>{model.detail}</small>
                      {model.download_available ? (
                        <small>
                          {model.download_size_mb ?? 0} MB / {model.download_source_count} sources
                        </small>
                      ) : null}
                      {model.state !== "ready" ? (
                        <div className="model-status-actions">
                          <button
                            className="mini-action model-review-action"
                            disabled={modelPreflightPendingId !== null || modelRefreshPending}
                            onClick={() => reviewModelDownload(model.id)}
                            type="button"
                          >
                            {modelPreflightPendingId === model.id ? "Reviewing" : "Review"}
                          </button>
                          <button
                            className="mini-action model-install-action"
                            disabled={
                              !model.download_available ||
                              installingModelId !== null ||
                              modelRefreshPending
                            }
                            onClick={() => void installModelArtifact(model.id)}
                            type="button"
                          >
                            {installingModelId === model.id ? "Installing" : "Install"}
                          </button>
                        </div>
                      ) : null}
                    </div>
                  </div>
                ))}
              </div>
            ) : null}
            {modelDownloadPreflight ? (
              <div
                className={`model-preflight-card status-${modelDownloadPreflight.state}`}
                aria-label="Model download preflight"
              >
                <div className="model-preflight-heading">
                  <span>Download preflight</span>
                  <strong>{modelDownloadPreflight.available ? "Available" : "Blocked"}</strong>
                </div>
                <strong>{modelDownloadPreflight.model_id}</strong>
                <span>
                  {modelDownloadPreflight.task}
                  {modelDownloadPreflight.lane ? ` / ${modelDownloadPreflight.lane}` : ""} /{" "}
                  {modelDownloadPreflight.runtime}
                </span>
                <small>{modelDownloadPreflight.detail}</small>
                {modelDownloadPreflight.available ? (
                  <>
                    <small>
                      Destination: {modelDownloadPreflight.destination_path ?? "not available"}
                    </small>
                    <small>
                      Expected sha256: {modelDownloadPreflight.expected_sha256 ?? "not available"}
                    </small>
                    <small>
                      Sources: {modelDownloadPreflight.source_count}; License:{" "}
                      {modelDownloadPreflight.license}
                      {modelDownloadPreflight.license_review_required
                        ? " / review required"
                        : ""}
                    </small>
                    <small>{modelDownloadPreflight.sources[0] ?? "No source URL exposed"}</small>
                  </>
                ) : (
                  <small>{modelDownloadPreflight.blocked_reason}</small>
                )}
                <small>Action: {modelDownloadPreflight.operator_action}</small>
                <small>Proof: {modelDownloadPreflight.proof_requirement}</small>
              </div>
            ) : null}
            {modelDownloadPreflightIssue ? (
              <p className="model-install-note">{modelDownloadPreflightIssue}</p>
            ) : null}
            {modelInstallIssue ? (
              <p className="model-install-note">{modelInstallIssue}</p>
            ) : null}
            {showModelRecheck ? (
              <button
                className="secondary-action"
                disabled={modelRefreshPending}
                onClick={refreshModelReadiness}
                type="button"
              >
                {modelRefreshPending ? "Checking Models" : "Recheck Models"}
              </button>
            ) : null}
            <button
              className="primary-action"
              disabled={firstRunActionDisabled}
              onClick={() => void handleFirstRunAction()}
              type="button"
            >
              {nextStep.action_label}
            </button>
          </article>
        </section>
      </section>
    </main>
  );
}
