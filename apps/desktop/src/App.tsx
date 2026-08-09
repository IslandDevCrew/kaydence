import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import {
  type AppView,
  type CleanupDial,
  type OsLane,
} from "./components/CockpitChrome";
import { NavRail } from "./components/NavRail";
import {
  type CockpitStatusItem,
  type DictateHistoryItem,
  DictateView,
} from "./views/DictateView";
import { CleanupView, type CleanupLaneSpec } from "./views/CleanupView";
import {
  type PrivacyAuditItem,
  PrivacyView,
} from "./views/PrivacyView";
import { FirstRunView } from "./views/FirstRunView";

const markUrl = new URL(
  "../../../assets/brand/logos/kaydence-logo-option-1.png",
  import.meta.url,
).href;
const appIconUrl = new URL("../src-tauri/icons/128x128.png", import.meta.url).href;

type HotkeyMode = "push_to_talk" | "toggle";

interface LaneSpec extends CleanupLaneSpec {
  injection: string;
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
  | "asr_runtime"
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
      default_dial: CleanupDial;
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

interface FirstRunModelDownloadResult {
  model_id: string;
  path: string;
  size_bytes: number;
  snapshot: AppSnapshot;
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
      default_local_asr: "whisper_gpu",
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
      model_ready: true,
      model_readiness_error: null,
      required_models: [],
      asr_candidates: [],
      recommended_asr_model_id: null,
      selected_asr_model_id: "whisper-base-en-q5_1",
      asr_runtime: {
        state: "verified_artifact",
        selected_model_id: "whisper-base-en-q5_1",
        lane: "gpu",
        runtime: "whisper_cpp",
        artifact_path: "/preview/models/ggml-base.en-q5_1.bin",
        artifact_size_bytes: 59_721_011,
        adapter_ready: true,
        detail: "Ready-state fixture for the local whisper.cpp lane.",
        proof_requirement: "Native readiness remains backend-owned.",
      },
      next_step: {
        kind: "complete",
        target_id: null,
        title: "Ready to dictate",
        detail: "The preview fixture represents a completed local setup.",
        action_label: "Start dictating",
        proof_requirement: "The native app must prove this state independently.",
      },
      permission_requirements: [
        {
          id: "microphone",
          label: "Microphone",
          state: "ready",
          detail: "Ready-state fixture for local microphone capture.",
          action: "Grant Kaydence access in System Settings -> Privacy & Security -> Microphone.",
          action_label: "Show microphone step",
        },
        {
          id: "accessibility",
          label: "Accessibility",
          state: "ready",
          detail: "Ready-state fixture for native insertion and focus checks.",
          action: "Enable Kaydence in System Settings -> Privacy & Security -> Accessibility.",
          action_label: "Show accessibility step",
        },
        {
          id: "input_monitoring",
          label: "Input Monitoring",
          state: "ready",
          detail: "Ready-state fixture for the global hotkey path.",
          action: "Enable Kaydence in System Settings -> Privacy & Security -> Input Monitoring.",
          action_label: "Show input step",
        },
      ],
      microphone_permission_ready: true,
      input_permission_ready: true,
      hotkey_registered: true,
      hotkey_registration_error: null,
      setup_timing: {
        started_at_ms: null,
        completed_at_ms: null,
        elapsed_ms: null,
        target_ms: 60000,
        within_target: null,
      },
      first_dictation_completed: true,
    },
  },
};

const previewHistory: HistorySession[] = [
  {
    id: "preview-project-kickoff",
    started_ms: 1_783_725_300_000,
    target_app: { id: "com.todesktop.230313mzl4w4u92", name: "Cursor" },
    audio_path: null,
    raw_text: "This local first dictation cockpit helps me capture ideas anywhere and ship clean text everywhere. It predicts ahead, cleans up, and inserts exactly where I am working. Speak freely. Ship confidently.",
    clean_text: "This local-first dictation cockpit helps me capture ideas anywhere and ship clean text everywhere. It predicts ahead, cleans up, and inserts exactly where I am working. Speak freely. Ship confidently.",
    cleanup_dial: "light",
    injected_method: "native",
    held_reason: null,
    failure: null,
    event_count: 7,
  },
  {
    id: "preview-bug-triage",
    started_ms: 1_783_721_700_000,
    target_app: { id: "com.tinyspeck.slackmacgap", name: "Slack" },
    audio_path: null,
    raw_text: "Bug triage summary for the injection fallback.",
    clean_text: "Bug triage summary for the injection fallback.",
    cleanup_dial: "light",
    injected_method: "clipboard_restore",
    held_reason: null,
    failure: null,
    event_count: 7,
  },
  {
    id: "preview-design-system",
    started_ms: 1_783_639_200_000,
    target_app: { id: "md.obsidian", name: "Obsidian" },
    audio_path: null,
    raw_text: "Design system overview and screen-family notes.",
    clean_text: "Design system overview and screen-family notes.",
    cleanup_dial: "light",
    injected_method: "native",
    held_reason: null,
    failure: null,
    event_count: 7,
  },
];

const lanes: LaneSpec[] = [
  {
    id: "mac",
    label: "macOS",
    accent: "#14a7a1",
    injection: "AX native insert",
    primaryMethod: "AX API (Accessibility)",
    primaryDetail: "Selected-text insertion with a secure-field refusal boundary.",
    fallbackMethod: "CGEvent + clipboard restore",
    fallbackDetail: "Unicode events, then snapshot, paste, and restore when needed.",
    gates: ["Accessibility", "Secure Input", "Latest Delivery", "Round Trip"],
  },
  {
    id: "windows",
    label: "Windows",
    // Cobalt (Windows 11 system-accent family). Was #2f72f2, which collided
    // byte-for-byte with the reserved --blue (Whisper-Ahead prediction text
    // only, never a per-OS accent) — see docs/design/DESIGN_LANGUAGE_V2_LOCK.md.
    accent: "#0067c0",
    injection: "UI Automation + SendInput",
    primaryMethod: "UI Automation (ValuePattern)",
    primaryDetail: "ValuePattern insertion when the focused control exposes a writable value.",
    fallbackMethod: "SendInput + clipboard restore",
    fallbackDetail: "Unicode input, then a restored clipboard path when direct input is unavailable.",
    gates: ["UI Automation", "Secure Desktop", "Latest Delivery", "Round Trip"],
  },
  {
    id: "linux",
    label: "Linux",
    accent: "#1f9d63",
    injection: "AT-SPI detect + uinput",
    primaryMethod: "AT-SPI detection + native best effort",
    primaryDetail: "Detects editable accessibility targets without claiming universal compositor proof.",
    fallbackMethod: "uinput / portal / X11 ladder",
    fallbackDetail: "Runtime capabilities select the safest available Wayland or X11 delivery path.",
    gates: ["AT-SPI / X11", "Secure Input", "Latest Delivery", "Round Trip"],
  },
];

function detectOsLane(): OsLane {
  const userAgent = navigator.userAgent.toLowerCase();
  if (userAgent.includes("win")) return "windows";
  if (userAgent.includes("linux")) return "linux";
  return "mac";
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

function historyTimestamp(startedMs: number | null): string {
  if (startedMs === null) {
    return "Saved";
  }
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(startedMs));
}

function privacyAuditItem(session: HistorySession): PrivacyAuditItem {
  const detail = `${session.target_app?.name ?? "Local target"} / ${session.event_count} local events`;
  if (session.failure) {
    return {
      id: session.id,
      timestamp: historyTimestamp(session.started_ms),
      title: `${session.failure.stage} issue recorded`,
      detail,
      outcome: "Review",
    };
  }
  if (session.held_reason) {
    return {
      id: session.id,
      timestamp: historyTimestamp(session.started_ms),
      title: "Delivery held safely",
      detail,
      outcome: session.held_reason.replaceAll("_", " "),
    };
  }
  if (session.injected_method) {
    return {
      id: session.id,
      timestamp: historyTimestamp(session.started_ms),
      title: "Local dictation completed",
      detail,
      outcome: session.injected_method.replaceAll("_", " "),
    };
  }
  return {
    id: session.id,
    timestamp: historyTimestamp(session.started_ms),
    title: session.audio_path ? "Local audio retained" : "Session metadata recorded",
    detail,
    outcome: session.audio_path ? "WAL ready" : "Local only",
  };
}

// Presentation only. The Rust backend owns all logic (root AGENTS §9).
export function App(): JSX.Element {
  const runtimeLane = useMemo(detectOsLane, []);
  const [activeLane, setActiveLane] = useState<OsLane>(runtimeLane);
  const [activeView, setActiveView] = useState<AppView>("Dictate");
  const [previewRecording, setPreviewRecording] = useState(true);
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
  const [downloadingModelId, setDownloadingModelId] = useState<string | null>(null);
  const [modelDownloadResult, setModelDownloadResult] =
    useState<FirstRunModelDownloadResult | null>(null);
  const [modelDownloadIssue, setModelDownloadIssue] = useState<string | null>(null);
  const [installingModelId, setInstallingModelId] = useState<string | null>(null);
  const [modelInstallIssue, setModelInstallIssue] = useState<string | null>(null);
  const [modelPreflightPendingId, setModelPreflightPendingId] = useState<string | null>(null);
  const [modelDownloadPreflight, setModelDownloadPreflight] =
    useState<FirstRunModelDownloadPreflight | null>(null);
  const [modelDownloadPreflightIssue, setModelDownloadPreflightIssue] =
    useState<string | null>(null);
  const [hotkeyModePending, setHotkeyModePending] = useState<HotkeyMode | null>(null);
  const [hotkeyModeIssue, setHotkeyModeIssue] = useState<string | null>(null);
  const [cleanupDialPending, setCleanupDialPending] = useState<CleanupDial | null>(null);
  const [cleanupDialIssue, setCleanupDialIssue] = useState<string | null>(null);
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
  const engineLabel =
    selectedAsr?.id ??
    (snapshot.settings.engine.default_local_asr === "parakeet_cpu"
      ? "Parakeet CPU"
      : "Whisper GPU");
  const unknownFocus =
    snapshot.settings.injection.unknown_focus_policy === "warn_and_allow"
      ? "Warn on opaque focus"
      : "Refuse opaque focus";
  const permissionRequirements = snapshot.settings.first_run.permission_requirements;
  const permissionSummaryText = permissionSummary(permissionRequirements);
  const nextStep = snapshot.settings.first_run.next_step;
  const firstRunActionDisabled =
    nextStep.kind === "setup" ||
    ((nextStep.kind === "model_install" || nextStep.kind === "permission") &&
      nextStep.target_id === null) ||
    downloadingModelId !== null ||
    installingModelId !== null ||
    permissionActionPendingId !== null ||
    setupRefreshPending ||
    modelRefreshPending ||
    modelPreflightPendingId !== null;
  const latestHistory = historySessions[0];
  const latestTarget = latestHistory?.target_app ?? null;
  const latestInjection = historySessions.find(
    (session) =>
      session.injected_method !== null ||
      session.held_reason !== null ||
      session.failure?.stage === "inject",
  );
  const injectionFailure =
    latestInjection?.failure?.stage === "inject" ? latestInjection.failure.error : null;
  const cockpitTranscript = latestHistory?.clean_text ?? latestHistory?.raw_text ?? "";
  const captureFailure = historySessions.find((session) => session.failure?.stage === "capture");
  const permissionsReady = permissionRequirementsReady(permissionRequirements);
  const cockpitOperational =
    asrRuntime.adapter_ready &&
    snapshot.settings.first_run.hotkey_registered &&
    permissionsReady;
  const cockpitStatuses: CockpitStatusItem[] = [
    {
      label: "Engine",
      value: asrRuntime.adapter_ready ? "Ready" : asrRuntime.state === "blocked" ? "Blocked" : "Pending",
      detail: engineLabel,
      icon: "cpu",
      state: asrRuntime.adapter_ready ? "ready" : asrRuntime.state === "blocked" ? "issue" : "pending",
    },
    {
      label: "Target App",
      value: latestTarget?.name ?? "Awaiting focus",
      detail: latestTarget ? "Bound at capture start" : "Runtime source pending",
      icon: "target",
      state: latestTarget ? "ready" : "pending",
    },
    {
      label: "Global Hotkey",
      value: snapshot.settings.hotkey.primary_binding,
      detail: `${hotkeyMode} / ${snapshot.settings.first_run.hotkey_registered ? "registered" : "not proven"}`,
      icon: "keyboard",
      state: snapshot.settings.first_run.hotkey_registration_error
        ? "issue"
        : snapshot.settings.first_run.hotkey_registered
          ? "ready"
          : "pending",
    },
    {
      label: "Privacy",
      value: "Local only",
      detail: `No telemetry / ${snapshot.settings.privacy.history_retention_days}-day history`,
      icon: "privacy",
      state: "ready",
    },
    {
      label: "WAL Recovery",
      value: captureFailure ? "Review needed" : historySessions.length ? "Healthy" : "Pending proof",
      detail: captureFailure?.failure?.error ?? `${historySessions.length} local sessions tracked`,
      icon: "database",
      state: captureFailure ? "issue" : historySessions.length ? "ready" : "pending",
    },
  ];
  const cockpitHistory: DictateHistoryItem[] = historySessions.map((session) => ({
    id: session.id,
    title: `${session.target_app?.name ?? "Local"} dictation`,
    summary: historySummary(session),
    status: historyStatus(session),
    timestamp: historyTimestamp(session.started_ms),
    hasAudio: session.audio_path !== null,
    audio:
      activeHistoryAudio?.sessionId === session.id
        ? {
            src: activeHistoryAudio.src,
            mimeType: activeHistoryAudio.mimeType,
            byteLength: activeHistoryAudio.byteLength,
          }
        : undefined,
  }));
  const privacyAuditItems = historySessions.map(privacyAuditItem);
  const historyExportNote = historyExportOutcome
    ? historyExportOutcome.exported
      ? `Exported to ${historyExportOutcome.text_path ?? historyExportOutcome.json_path}`
      : "No matching local session to export."
    : null;
  const historyPurgeNote = historyPurgeOutcome
    ? `Purged ${historyPurgeOutcome.sessions_deleted} sessions and ${historyPurgeOutcome.audio_files_removed} audio files.`
    : null;
  const pendingHistoryAction =
    deletingHistoryId !== null ||
    exportingHistoryId !== null ||
    loadingHistoryAudioId !== null ||
    purgingHistory;

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
    if (snapshotSource === "preview") {
      setSnapshot((current) => ({
        ...current,
        settings: {
          ...current.settings,
          hotkey: { ...current.settings.hotkey, mode },
        },
      }));
      setHotkeyModePending(null);
      return;
    }
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

  function setCleanupDial(dial: CleanupDial) {
    setCleanupDialPending(dial);
    setCleanupDialIssue(null);
    if (snapshotSource === "preview") {
      setSnapshot((current) => ({
        ...current,
        settings: {
          ...current.settings,
          cleanup: { ...current.settings.cleanup, default_dial: dial },
        },
      }));
      setCleanupDialPending(null);
      return;
    }
    void invoke<AppSnapshot>("set_cleanup_dial", { dial })
      .then((nextSnapshot) => {
        setSnapshot(nextSnapshot);
        setSnapshotSource("backend");
      })
      .catch((error) => {
        console.error("Kaydence cleanup dial update failed", error);
        setCleanupDialIssue("Cleanup dial can be changed when capture is idle.");
      })
      .finally(() => {
        setCleanupDialPending(null);
      });
  }

  function setHotkeyBinding(binding: string) {
    setHotkeyBindingPending(binding);
    setHotkeyBindingIssue(null);
    if (snapshotSource === "preview") {
      setSnapshot((current) => ({
        ...current,
        settings: {
          ...current.settings,
          hotkey: { ...current.settings.hotkey, primary_binding: binding },
        },
      }));
      setHotkeyBindingPending(null);
      return;
    }
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
    setModelDownloadIssue(null);
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
    setModelDownloadIssue(null);

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

  async function downloadModelArtifact(modelId: string) {
    if (
      modelDownloadPreflight?.model_id !== modelId ||
      !modelDownloadPreflight.available
    ) {
      setModelDownloadIssue("Review the pinned source and checksum before downloading.");
      return;
    }

    setDownloadingModelId(modelId);
    setModelDownloadIssue(null);
    setModelDownloadResult(null);
    setModelInstallIssue(null);
    try {
      const outcome = await invoke<FirstRunModelDownloadResult>("first_run_model_download", {
        modelId,
      });
      setSnapshot(outcome.snapshot);
      setSnapshotSource("backend");
      setModelDownloadResult(outcome);
    } catch (error) {
      console.error("Kaydence model download failed", error);
      const unavailable = String(error).includes("unavailable in this build");
      setModelDownloadIssue(
        unavailable
          ? "Automatic download is off in this build. Install the reviewed local artifact instead."
          : "The model could not be downloaded and verified. Retry or install the reviewed local artifact.",
      );
    } finally {
      setDownloadingModelId(null);
    }
  }

  async function handleFirstRunAction() {
    setFirstRunActionNote(null);

    if (nextStep.kind === "dictation" || nextStep.kind === "complete") {
      setActiveView("Dictate");
      return;
    }

    if (snapshotSource === "preview") {
      setFirstRunActionNote(nextStep.proof_requirement);
      return;
    }

    if (nextStep.kind === "permission" && nextStep.target_id) {
      showPermissionAction(nextStep.target_id);
      return;
    }

    if (nextStep.kind === "asr_runtime") {
      setFirstRunActionNote(nextStep.proof_requirement);
      refreshModelReadiness();
      return;
    }

    if (nextStep.kind === "model_install" && nextStep.target_id) {
      reviewModelDownload(nextStep.target_id);
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

  if (activeView === "Dictate") {
    return (
      <main
        className="app-shell dictate-shell"
        style={{ "--accent": lane.accent } as React.CSSProperties}
      >
        <NavRail
          activeView="Dictate"
          appName={snapshot.app_name}
          footerTitle="Engine: Local"
          markUrl={appIconUrl}
          onNavigate={setActiveView}
          privacySummary={`Model: ${engineLabel}`}
        />
        <DictateView
          activeLane={activeLane}
          appName={snapshot.app_name}
          autoPasteReady={permissionsReady}
          cleanupDial={cleanupDefault}
          cleanupIssue={cleanupDialIssue}
          cleanupPending={cleanupDialPending !== null}
          elapsed={snapshotSource === "preview" && previewRecording ? "00:01.24" : "--:--"}
          historyExportNote={historyExportNote}
          historyItems={cockpitHistory}
          historyPlaybackIssue={historyPlaybackIssue}
          historyPurgeNote={historyPurgeNote}
          historyRefreshPending={historyRefreshPending}
          laneOptions={lanes.map(({ id, label }) => ({ id, label }))}
          markUrl={markUrl}
          microphone={
            snapshot.settings.first_run.microphone_permission_ready
              ? "Microphone proof ready"
              : "Awaiting microphone proof"
          }
          onCleanupChange={setCleanupDial}
          onClearLatest={deleteHistorySession}
          onDeleteHistory={deleteHistorySession}
          onExportHistory={exportHistorySession}
          onLaneChange={setActiveLane}
          onNavigate={setActiveView}
          onPlayHistory={playHistoryAudio}
          onPurgeHistory={purgeHistory}
          onRecordToggle={
            snapshotSource === "preview"
              ? () => setPreviewRecording((recording) => !recording)
              : undefined
          }
          onRefreshHistory={refreshHistory}
          operational={cockpitOperational}
          outputDestination={latestTarget ? `Insert at ${latestTarget.name}` : "Insert at cursor"}
          outputMethod={lane.injection}
          pendingHistoryAction={pendingHistoryAction}
          recording={snapshotSource === "preview" && previewRecording}
          statusItems={cockpitStatuses}
          transcript={cockpitTranscript}
        />
      </main>
    );
  }

  if (activeView === "Cleanup") {
    return (
      <main
        className="app-shell cleanup-shell"
        style={{ "--accent": lane.accent } as React.CSSProperties}
      >
        <NavRail
          activeView="Cleanup"
          appName={snapshot.app_name}
          footerTitle="Engine: Local"
          markUrl={appIconUrl}
          onNavigate={setActiveView}
          privacySummary={`Model: ${engineLabel}`}
        />
        <CleanupView
          appName={snapshot.app_name}
          cleanupDial={cleanupDefault}
          cleanupIssue={cleanupDialIssue}
          cleanupPending={cleanupDialPending !== null}
          deliveryMethod={latestInjection?.injected_method ?? null}
          heldReason={latestInjection?.held_reason ?? null}
          injectionFailure={injectionFailure}
          lane={lane}
          lanes={lanes}
          markUrl={appIconUrl}
          onCleanupChange={setCleanupDial}
          onLaneChange={setActiveLane}
          onNavigate={setActiveView}
          permissionSummary={permissionSummaryText}
          permissionsReady={permissionsReady}
          previewFixture={snapshotSource === "preview"}
          targetApp={latestTarget?.name ?? null}
          unknownFocus={unknownFocus}
        />
      </main>
    );
  }

  if (activeView === "Privacy") {
    return (
      <main
        className="app-shell privacy-shell"
        style={{ "--accent": lane.accent } as React.CSSProperties}
      >
        <NavRail
          activeView="Privacy"
          appName={snapshot.app_name}
          footerTitle="Private by design"
          markUrl={appIconUrl}
          onNavigate={setActiveView}
          privacySummary={`${snapshot.settings.privacy.history_retention_days}-day local history`}
        />
        <PrivacyView
          appName={snapshot.app_name}
          auditItems={privacyAuditItems}
          contextEnabled={snapshot.settings.privacy.local_context_enabled}
          engineLabel={engineLabel}
          historyRetentionDays={snapshot.settings.privacy.history_retention_days}
          lane={lane}
          lanes={lanes}
          markUrl={appIconUrl}
          ocrEnabled={snapshot.settings.privacy.local_ocr_enabled}
          onLaneChange={setActiveLane}
          onNavigate={setActiveView}
          operational={cockpitOperational}
          permissionRequirements={permissionRequirements}
          previewFixture={snapshotSource === "preview"}
          runtimeLane={runtimeLane}
        />
      </main>
    );
  }

  if (activeView === "Setup") {
    return (
      <main
        className="app-shell first-run-shell"
        style={{ "--accent": lane.accent } as React.CSSProperties}
      >
        <FirstRunView
          appName={snapshot.app_name}
          cleanupDial={cleanupDefault}
          cleanupDialIssue={cleanupDialIssue}
          cleanupDialPending={cleanupDialPending}
          exportingProof={exportingFirstRunProof}
          firstRun={snapshot.settings.first_run}
          firstRunActionDisabled={firstRunActionDisabled}
          firstRunActionNote={firstRunActionNote}
          hotkeyBinding={snapshot.settings.hotkey.primary_binding}
          hotkeyBindingIssue={hotkeyBindingIssue}
          hotkeyBindingOptions={hotkeyBindingOptions}
          hotkeyBindingPending={hotkeyBindingPending}
          hotkeyMode={snapshot.settings.hotkey.mode}
          hotkeyModeIssue={hotkeyModeIssue}
          hotkeyModePending={hotkeyModePending}
          downloadingModelId={downloadingModelId}
          installingModelId={installingModelId}
          lane={lane}
          lanes={lanes}
          markUrl={appIconUrl}
          modelDownloadPreflight={modelDownloadPreflight}
          modelDownloadPreflightIssue={modelDownloadPreflightIssue}
          modelDownloadIssue={modelDownloadIssue}
          modelDownloadResult={modelDownloadResult}
          modelInstallIssue={modelInstallIssue}
          modelPreflightPendingId={modelPreflightPendingId}
          modelRefreshPending={modelRefreshPending}
          onCleanupDialChange={setCleanupDial}
          onExportProof={exportFirstRunProofPlan}
          onHotkeyBindingChange={setHotkeyBinding}
          onHotkeyModeChange={setHotkeyMode}
          onDownloadModel={(modelId) => void downloadModelArtifact(modelId)}
          onInstallModel={(modelId) => void installModelArtifact(modelId)}
          onLaneChange={setActiveLane}
          onModelChange={selectAsrModel}
          onNavigate={setActiveView}
          onOpenPermissionSettings={openPermissionSettings}
          onPrimaryAction={() => void handleFirstRunAction()}
          onRefreshModels={refreshModelReadiness}
          onRefreshSetup={refreshSetupSnapshot}
          onReviewModel={reviewModelDownload}
          onShowPermission={showPermissionAction}
          permissionActionIssue={permissionActionIssue}
          permissionActionOutcome={permissionActionOutcome}
          permissionActionPendingId={permissionActionPendingId}
          permissionSettingsIssue={permissionSettingsIssue}
          permissionSettingsOutcome={permissionSettingsOutcome}
          permissionSettingsPendingId={permissionSettingsPendingId}
          previewFixture={snapshotSource === "preview"}
          proofExport={firstRunProofExport}
          proofIssue={firstRunProofIssue}
          runtimeLane={runtimeLane}
          setupRefreshIssue={setupRefreshIssue}
          setupRefreshPending={setupRefreshPending}
        />
      </main>
    );
  }

  throw new Error("Unsupported Kaydence view");
}
