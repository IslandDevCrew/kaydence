import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";

const markUrl = new URL(
  "../../../assets/brand/logos/kaydence-logo-option-1.png",
  import.meta.url,
).href;

type OsLane = "mac" | "windows" | "linux";

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

interface FirstRunModelStatus {
  id: string;
  task: string;
  lane: string | null;
  runtime: string;
  file: string;
  state: "ready" | "missing" | "blocked";
  detail: string;
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
  selected: boolean;
  recommendation: string | null;
  license: string;
  license_review_required: boolean;
}

interface AppSnapshot {
  app_name: string;
  app_identifier: string;
  settings: {
    hotkey: {
      mode: "push_to_talk" | "toggle";
      primary_binding: string;
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

const previewSnapshot: AppSnapshot = {
  app_name: "Kaydence",
  app_identifier: "io.kaydence.app",
  settings: {
    hotkey: {
      mode: "push_to_talk",
      primary_binding: "RightAlt",
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

// Presentation only. The Rust backend owns all logic (root AGENTS §9).
export function App(): JSX.Element {
  const [activeLane, setActiveLane] = useState<OsLane>("mac");
  const [snapshot, setSnapshot] = useState<AppSnapshot>(previewSnapshot);
  const [snapshotSource, setSnapshotSource] = useState<"backend" | "preview">("preview");
  const lane = useMemo(
    () => lanes.find((candidate) => candidate.id === activeLane) ?? lanes[0],
    [activeLane],
  );
  const hotkeyMode =
    snapshot.settings.hotkey.mode === "push_to_talk" ? "Push-to-talk" : "Toggle";
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
                    </div>
                  </div>
                ))}
              </div>
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
