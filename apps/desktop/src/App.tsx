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
  permission: string;
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

interface ChecklistItem {
  label: string;
  status: "Ready" | "Needs hardware" | "Next" | "Issue";
  detail?: string;
}

interface AppRef {
  id: string;
  name: string;
}

interface FirstRunModelStatus {
  id: string;
  task: string;
  lane: string | null;
  runtime: string;
  file: string;
  state: "ready" | "missing" | "blocked";
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
  state: "ready" | "missing" | "blocked";
  detail: string;
  download_available: boolean;
  download_size_mb: number | null;
  download_source_count: number;
  selected: boolean;
  recommendation: string | null;
  license: string;
  license_review_required: boolean;
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
      microphone_permission_ready: boolean;
      input_permission_ready: boolean;
      hotkey_registered: boolean;
      hotkey_registration_error: string | null;
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
      microphone_permission_ready: false,
      input_permission_ready: false,
      hotkey_registered: false,
      hotkey_registration_error: null,
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
    permission: "Microphone, Accessibility, Input Monitoring",
  },
  {
    id: "windows",
    label: "Windows",
    accent: "#2f72f2",
    injection: "UI Automation + SendInput",
    permission: "Microphone privacy setting",
  },
  {
    id: "linux",
    label: "Linux",
    accent: "#1f9d63",
    injection: "AT-SPI detect + uinput",
    permission: "Input group or portal consent",
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
  { label: "Remote", value: "404", detail: "local-first recovery" },
];

function firstRunChecklist(snapshot: AppSnapshot): ChecklistItem[] {
  const firstRun = snapshot.settings.first_run;
  const permissionsReady =
    firstRun.microphone_permission_ready && firstRun.input_permission_ready;

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
  const [modelRefreshPending, setModelRefreshPending] = useState(false);
  const [installingModelId, setInstallingModelId] = useState<string | null>(null);
  const [modelInstallIssue, setModelInstallIssue] = useState<string | null>(null);
  const [hotkeyModePending, setHotkeyModePending] = useState<HotkeyMode | null>(null);
  const [hotkeyModeIssue, setHotkeyModeIssue] = useState<string | null>(null);
  const [hotkeyBindingPending, setHotkeyBindingPending] = useState<string | null>(null);
  const [hotkeyBindingIssue, setHotkeyBindingIssue] = useState<string | null>(null);
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
  const firstRunReady =
    snapshot.settings.first_run.model_ready &&
    snapshot.settings.first_run.microphone_permission_ready &&
    snapshot.settings.first_run.input_permission_ready &&
    snapshot.settings.first_run.hotkey_registered;
  const requiredModels = snapshot.settings.first_run.required_models;
  const showModelRecheck =
    !snapshot.settings.first_run.model_ready &&
    (requiredModels.length > 0 ||
      snapshot.settings.first_run.model_readiness_error !== null);

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

  async function installModelArtifact(modelId: string) {
    setInstallingModelId(modelId);
    setModelInstallIssue(null);
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
                <span>Permission</span>
                <strong>{lane.permission}</strong>
              </div>
              <div>
                <span>Unknown focus</span>
                <strong>{unknownFocus}</strong>
              </div>
            </div>
          </article>

          <article className="panel setup-panel">
            <div className="panel-header">
              <div>
                <p className="eyebrow">Screen family 10</p>
                <h2>First run</h2>
              </div>
              <span className="pill">Free selected</span>
            </div>
            <ol className="setup-list">
              {checklist.map((item) => (
                <li className={`setup-item status-${item.status.toLowerCase()}`} key={item.label}>
                  <span>
                    {item.label}
                    {item.detail ? <small>{item.detail}</small> : null}
                  </span>
                  <strong>{item.status}</strong>
                </li>
              ))}
            </ol>
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
                      ) : null}
                    </div>
                  </div>
                ))}
              </div>
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
            <button className="primary-action" disabled={!firstRunReady} type="button">
              {firstRunReady ? "Start Dictating" : "Resolve Setup"}
            </button>
          </article>
        </section>
      </section>
    </main>
  );
}
