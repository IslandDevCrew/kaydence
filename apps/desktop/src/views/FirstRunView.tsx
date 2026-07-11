import { useEffect, useState } from "react";
import {
  CockpitGlyph,
  type AppView,
  type CleanupDial,
  type OsLane,
} from "../components/CockpitChrome";
import "./FirstRunView.css";
import "./FirstRunEvidence.css";

type FirstRunModelState = "ready" | "missing" | "blocked";
type FirstRunPermissionState = "ready" | "needs_hardware" | "needs_review" | "blocked";
type FirstRunNextStepKind =
  | "setup"
  | "model_metadata"
  | "model_install"
  | "permission"
  | "hotkey"
  | "dictation"
  | "complete";
type FirstRunAsrRuntimeState = "pending" | "blocked" | "verified_artifact";
type EvidenceSection = "models" | "permissions" | "proof";

export interface FirstRunLaneSpec {
  accent: string;
  id: OsLane;
  label: string;
}

export interface FirstRunModelStatus {
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

export interface FirstRunAsrCandidate {
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

export interface FirstRunPermissionRequirement {
  id: string;
  label: string;
  state: FirstRunPermissionState;
  detail: string;
  action: string;
  action_label: string;
}

export interface FirstRunPermissionActionOutcome {
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

export interface FirstRunPermissionSettingsOpenOutcome {
  requirement_id: string;
  label: string;
  opened: boolean;
  settings_target: string | null;
  manual_step: string;
  proof_requirement: string;
  expected_evidence: string;
}

export interface FirstRunNextStep {
  kind: FirstRunNextStepKind;
  target_id: string | null;
  title: string;
  detail: string;
  action_label: string;
  proof_requirement: string;
}

export interface FirstRunSetupTiming {
  started_at_ms: number | null;
  completed_at_ms: number | null;
  elapsed_ms: number | null;
  target_ms: number;
  within_target: boolean | null;
}

export interface FirstRunProofExportOutcome {
  exported: boolean;
  json_path: string | null;
  item_count: number;
}

export interface FirstRunModelDownloadPreflight {
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

export interface FirstRunAsrRuntimeStatus {
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

export interface FirstRunStatus {
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
}

export interface FirstRunHotkeyOption {
  id: string;
  label: string;
  detail: string;
}

interface FirstRunViewProps {
  appName: string;
  cleanupDial: CleanupDial;
  cleanupDialIssue: string | null;
  cleanupDialPending: CleanupDial | null;
  exportingProof: boolean;
  firstRun: FirstRunStatus;
  firstRunActionDisabled: boolean;
  firstRunActionNote: string | null;
  hotkeyBinding: string;
  hotkeyBindingIssue: string | null;
  hotkeyBindingOptions: FirstRunHotkeyOption[];
  hotkeyBindingPending: string | null;
  hotkeyMode: "push_to_talk" | "toggle";
  hotkeyModeIssue: string | null;
  hotkeyModePending: "push_to_talk" | "toggle" | null;
  installingModelId: string | null;
  lane: FirstRunLaneSpec;
  lanes: FirstRunLaneSpec[];
  markUrl: string;
  modelDownloadPreflight: FirstRunModelDownloadPreflight | null;
  modelDownloadPreflightIssue: string | null;
  modelInstallIssue: string | null;
  modelPreflightPendingId: string | null;
  modelRefreshPending: boolean;
  onCleanupDialChange: (dial: CleanupDial) => void;
  onExportProof: () => void;
  onHotkeyBindingChange: (binding: string) => void;
  onHotkeyModeChange: (mode: "push_to_talk" | "toggle") => void;
  onInstallModel: (modelId: string) => void;
  onLaneChange: (lane: OsLane) => void;
  onModelChange: (modelId: string) => void;
  onNavigate: (view: AppView) => void;
  onOpenPermissionSettings: (requirementId: string) => void;
  onPrimaryAction: () => void;
  onRefreshModels: () => void;
  onRefreshSetup: () => void;
  onReviewModel: (modelId: string) => void;
  onShowPermission: (requirementId: string) => void;
  permissionActionIssue: string | null;
  permissionActionOutcome: FirstRunPermissionActionOutcome | null;
  permissionActionPendingId: string | null;
  permissionSettingsIssue: string | null;
  permissionSettingsOutcome: FirstRunPermissionSettingsOpenOutcome | null;
  permissionSettingsPendingId: string | null;
  previewFixture: boolean;
  proofExport: FirstRunProofExportOutcome | null;
  proofIssue: string | null;
  runtimeLane: OsLane;
  setupRefreshIssue: string | null;
  setupRefreshPending: boolean;
}

const setupSteps = [
  "Welcome",
  "ASR Engine",
  "Permissions",
  "Hotkey",
  "Cleanup Default",
  "Whisper-Ahead",
  "Privacy",
  "Choose Tier",
  "Finish",
];

const licenseCards = [
  { id: "free", name: "Free", price: "$0", term: "MIT Core", items: ["Local dictation", "Raw + Light cleanup", "History / one device"] },
  { id: "pro", name: "Pro", price: "$79", term: "One-time / major", items: ["Whisper-Ahead", "Profiles + context", "Voiceprint"] },
  { id: "captain", name: "Captain", price: "$189", term: "Lifetime", items: ["Everything in Pro", "Relay + Conductor", "Priority support"] },
  { id: "harbor", name: "Harbor", price: "Optional", term: "Hosted services", items: ["~$6/mo or $60/yr", "Every service self-hostable", "No feature gated"] },
];

const referencePermissions: Record<OsLane, FirstRunPermissionRequirement[]> = {
  mac: [
    { id: "microphone", label: "Microphone", state: "needs_review", detail: "Requires capture proof on the target Mac.", action: "Review microphone access on the target Mac.", action_label: "Reference" },
    { id: "accessibility", label: "Accessibility", state: "needs_review", detail: "Requires native insert and secure-field proof.", action: "Review Accessibility on the target Mac.", action_label: "Reference" },
    { id: "input_monitoring", label: "Input Monitoring", state: "needs_review", detail: "Requires a live global-hotkey proof.", action: "Review Input Monitoring on the target Mac.", action_label: "Reference" },
  ],
  windows: [
    { id: "microphone", label: "Microphone", state: "needs_review", detail: "Requires capture proof on the target PC.", action: "Review microphone access on the target PC.", action_label: "Reference" },
    { id: "uia_focus", label: "UI Automation", state: "needs_review", detail: "Requires focus and password-field proof.", action: "Review UI Automation on the target PC.", action_label: "Reference" },
    { id: "sendinput", label: "Keyboard fallback", state: "needs_review", detail: "Requires fallback typing proof after the focus gate.", action: "Review SendInput fallback on the target PC.", action_label: "Reference" },
  ],
  linux: [
    { id: "microphone", label: "Microphone", state: "needs_review", detail: "Requires PipeWire or PulseAudio capture proof.", action: "Review capture on the target Linux desktop.", action_label: "Reference" },
    { id: "accessibility_bus", label: "AT-SPI bus", state: "needs_review", detail: "Requires a live desktop-session focus proof.", action: "Review AT-SPI on the target Linux desktop.", action_label: "Reference" },
    { id: "uinput", label: "uinput path", state: "needs_hardware", detail: "Requires operator-in-the-loop field validation.", action: "Review uinput on the target Linux desktop.", action_label: "Reference" },
  ],
};

function permissionLabel(state: FirstRunPermissionState): string {
  if (state === "ready") return "Ready";
  if (state === "blocked") return "Blocked";
  if (state === "needs_hardware") return "Hardware proof";
  return "Review";
}

function runtimeLabel(runtime: FirstRunAsrRuntimeStatus): string {
  if (runtime.state === "verified_artifact") return "Verified artifact";
  if (runtime.state === "blocked") return "Blocked";
  return "Pending";
}

function timingLabel(timing: FirstRunSetupTiming): string {
  if (timing.completed_at_ms !== null && timing.elapsed_ms !== null) {
    return `${Math.round(timing.elapsed_ms / 1000)}s ${timing.within_target ? "pass" : "over target"}`;
  }
  if (timing.started_at_ms !== null) return "Timer active";
  return "Timer armed";
}

function activeStep(kind: FirstRunNextStepKind): number {
  if (kind === "model_metadata" || kind === "model_install") return 2;
  if (kind === "permission") return 3;
  if (kind === "hotkey") return 4;
  if (kind === "dictation" || kind === "complete") return 9;
  return 1;
}

function evidenceFor(kind: FirstRunNextStepKind): EvidenceSection {
  if (kind === "model_metadata" || kind === "model_install") return "models";
  if (kind === "permission") return "permissions";
  return "proof";
}

export function FirstRunView(props: FirstRunViewProps): JSX.Element {
  const [evidenceOpen, setEvidenceOpen] = useState(false);
  const [evidenceSection, setEvidenceSection] = useState<EvidenceSection>("proof");
  const currentStep = activeStep(props.firstRun.next_step.kind);
  const selectedRuntime = props.lane.id === props.runtimeLane;
  const displayedPermissions = selectedRuntime
    ? props.firstRun.permission_requirements
    : referencePermissions[props.lane.id];
  const permissionsReady = selectedRuntime && displayedPermissions.length > 0 &&
    displayedPermissions.every((item) => item.state === "ready");
  const selectedModel = props.firstRun.selected_asr_model_id ??
    props.firstRun.asr_runtime.selected_model_id ??
    "No model selected";
  const modelOptions = props.firstRun.asr_candidates.length
    ? props.firstRun.asr_candidates
    : [{ id: selectedModel, selected: true }];

  useEffect(() => {
    if (!evidenceOpen) return undefined;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setEvidenceOpen(false);
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [evidenceOpen]);

  const openEvidence = (section: EvidenceSection) => {
    setEvidenceSection(section);
    setEvidenceOpen(true);
  };

  const runPrimaryAction = () => {
    const kind = props.firstRun.next_step.kind;
    if (kind !== "dictation" && kind !== "complete") openEvidence(evidenceFor(kind));
    props.onPrimaryAction();
  };

  const stepDone = (step: number): boolean => {
    if (step === 1 || step === 5 || step === 7 || step === 8) return true;
    if (step === 2) return props.firstRun.model_ready;
    if (step === 3) return permissionsReady;
    if (step === 4) return props.firstRun.hotkey_registered;
    if (step === 9) return props.firstRun.first_dictation_completed;
    return false;
  };

  return (
    <section className="first-run-board" aria-label={`${props.appName} first run and license`}>
      <header className="first-run-titlebar">
        <div><img src={props.markUrl} alt="" /><strong>{props.appName}</strong></div>
        <div className="first-run-lanes" aria-label="Operating system lane">
          {props.lanes.map((lane) => (
            <button aria-pressed={lane.id === props.lane.id} key={lane.id} onClick={() => props.onLaneChange(lane.id)} type="button">{lane.label}</button>
          ))}
        </div>
      </header>

      <div className="first-run-window">
        <aside className="first-run-step-rail">
          <div className="first-run-brand"><img src={props.markUrl} alt="" /><strong>{props.appName}</strong></div>
          <ol>
            {setupSteps.map((label, index) => {
              const step = index + 1;
              const later = step === 6;
              return (
                <li className={`${step === currentStep ? "active " : ""}${stepDone(step) ? "done " : ""}${later ? "later" : ""}`} key={label}>
                  <span>{stepDone(step) ? <CockpitGlyph name="check" /> : step}</span><strong>{label}</strong>{later ? <small>P3</small> : null}
                </li>
              );
            })}
          </ol>
          <div className="first-run-local-note"><CockpitGlyph name="privacy" /><strong>100% local by default</strong><small>Your voice stays on your devices.</small></div>
        </aside>

        <div className="first-run-workspace">
          <header className="first-run-heading">
            <div><span>Screen family 10</span><h1>First Run Setup &amp; License</h1><p>Configure {props.appName} for this {props.lane.label} device.</p></div>
            <button disabled={props.setupRefreshPending || !selectedRuntime} onClick={props.onRefreshSetup} type="button">{selectedRuntime ? (props.setupRefreshPending ? "Refreshing" : "Refresh proof") : "Reference lane"}</button>
          </header>

          <ol className="first-run-progress" aria-label="First-run progress">
            {setupSteps.map((label, index) => {
              const step = index + 1;
              return <li aria-current={step === currentStep ? "step" : undefined} className={`${step === currentStep ? "active " : ""}${stepDone(step) ? "done" : ""}`} key={label}><span>{step}</span></li>;
            })}
          </ol>

          <section className={`first-run-next step-${props.firstRun.next_step.kind}`} aria-label="First-run next step">
            <span>Next step</span><strong>{props.firstRun.next_step.title}</strong><p>{props.firstRun.next_step.detail}</p><button onClick={() => openEvidence(evidenceFor(props.firstRun.next_step.kind))} type="button">Evidence</button>
          </section>

          <section className="first-run-controls" aria-label="First-run configuration">
            <div className="first-run-control-row">
              <span className="first-run-control-icon"><CockpitGlyph name="cpu" /></span>
              <div><strong>Choose Local ASR Engine</strong><small>Offline speech-to-text engine</small></div>
              <select aria-label="Local ASR engine" disabled={props.modelRefreshPending || !selectedRuntime} onChange={(event) => props.onModelChange(event.target.value)} value={selectedModel}>
                {modelOptions.map((model) => <option key={model.id} value={model.id}>{model.id}</option>)}
              </select>
              <button aria-label="Review models" className="first-run-status-button" onClick={() => openEvidence("models")} type="button"><span className={props.firstRun.model_ready ? "ready" : "issue"}>{props.firstRun.model_ready ? "Ready" : "Review"}</span></button>
            </div>

            <div className="first-run-control-row">
              <span className="first-run-control-icon"><CockpitGlyph name="privacy" /></span>
              <div><strong>Grant OS Permissions</strong><small>{displayedPermissions.map((item) => item.label).join(", ") || "Runtime requirements"}</small></div>
              <button className="first-run-inline-action" onClick={() => openEvidence("permissions")} type="button">Review</button>
              <span className={permissionsReady ? "first-run-state ready" : "first-run-state pending"}>{selectedRuntime ? (permissionsReady ? "Ready" : "Proof") : "Reference"}</span>
            </div>

            <div className="first-run-control-row">
              <span className="first-run-control-icon"><CockpitGlyph name="keyboard" /></span>
              <div><strong>Set Global Dictation Hotkey</strong><small>{props.hotkeyMode === "push_to_talk" ? "Hold to dictate" : "Toggle dictation"}</small></div>
              <div className="first-run-hotkey-control">
                <select aria-label="Global hotkey" disabled={props.hotkeyBindingPending !== null || !selectedRuntime} onChange={(event) => props.onHotkeyBindingChange(event.target.value)} value={props.hotkeyBinding}>
                  {props.hotkeyBindingOptions.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}
                </select>
                <div role="group" aria-label="Hotkey mode">
                  <button aria-pressed={props.hotkeyMode === "push_to_talk"} disabled={props.hotkeyModePending !== null || !selectedRuntime} onClick={() => props.onHotkeyModeChange("push_to_talk")} type="button">Hold</button>
                  <button aria-pressed={props.hotkeyMode === "toggle"} disabled={props.hotkeyModePending !== null || !selectedRuntime} onClick={() => props.onHotkeyModeChange("toggle")} type="button">Toggle</button>
                </div>
              </div>
              <span className={props.firstRun.hotkey_registered && selectedRuntime ? "first-run-state ready" : "first-run-state pending"}>{selectedRuntime ? (props.firstRun.hotkey_registered ? "Ready" : "Proof") : "Reference"}</span>
            </div>

            <div className="first-run-control-row">
              <span className="first-run-control-icon"><CockpitGlyph name="cleanup" /></span>
              <div><strong>Cleanup Default</strong><small>Raw, Light, or explicit Full</small></div>
              <div className="first-run-cleanup-control" role="group" aria-label="Cleanup default">
                {(["raw", "light", "full"] as const).map((dial) => <button aria-pressed={props.cleanupDial === dial} disabled={props.cleanupDialPending !== null} key={dial} onClick={() => props.onCleanupDialChange(dial)} type="button">{dial}</button>)}
              </div>
              <span className="first-run-state ready">Ready</span>
            </div>

            <div className="first-run-control-row is-future">
              <span className="first-run-control-icon"><CockpitGlyph name="waveform" /></span>
              <div><strong>Enable Whisper-Ahead</strong><small>Live previews while you speak</small></div>
              <label className="first-run-switch"><input aria-label="Whisper-Ahead is planned for P3" checked={false} disabled readOnly type="checkbox" /><span /></label>
              <span className="first-run-state future">P3</span>
            </div>

            <div className="first-run-control-row">
              <span className="first-run-control-icon"><CockpitGlyph name="privacy" /></span>
              <div><strong>Privacy Confirmation</strong><small>Local processing, visible persistence</small></div>
              <button className="first-run-confirmed" disabled type="button">Confirmed</button>
              <span className="first-run-state ready">Ready</span>
            </div>

            <div className="first-run-control-row">
              <span className="first-run-control-icon"><CockpitGlyph name="target" /></span>
              <div><strong>Select Your Tier</strong><small>Desktop features are pay-once</small></div>
              <strong className="first-run-free-label">Free ($0 MIT)</strong>
              <span className="first-run-state ready">Selected</span>
            </div>
          </section>

          <section className="first-run-license" aria-label="License and service choices">
            {licenseCards.map((card) => (
              <label className={`first-run-license-card${card.id === "free" ? " selected" : ""}`} key={card.id}>
                <span><strong>{card.name}</strong><b>{card.price}</b><small>{card.term}</small></span>
                <ul>{card.items.map((item) => <li key={item}>{item}</li>)}</ul>
                <input checked={card.id === "free"} disabled={card.id !== "free"} name="license-tier" readOnly type="radio" />
              </label>
            ))}
          </section>

          <p className="first-run-pricing-note"><CockpitGlyph name="check" /> Desktop capability is one-time. Harbor covers optional hosted services and every service remains self-hostable.</p>

          <footer className="first-run-footer">
            <button onClick={() => props.onNavigate("Dictate")} type="button">Cancel</button>
            <div><small>{selectedRuntime ? (props.previewFixture ? "Fixture state" : timingLabel(props.firstRun.setup_timing)) : `${props.lane.label} reference lane`}</small><strong>{selectedRuntime ? (props.firstRunActionNote ?? props.setupRefreshIssue ?? props.firstRun.next_step.proof_requirement) : `Runtime proof remains on ${props.runtimeLane === "mac" ? "macOS" : props.runtimeLane === "windows" ? "Windows" : "Linux"}.`}</strong></div>
            <button className="primary" disabled={props.firstRunActionDisabled || !selectedRuntime} onClick={runPrimaryAction} type="button">{selectedRuntime ? props.firstRun.next_step.action_label : "Runtime only"}</button>
          </footer>
        </div>
      </div>

      {evidenceOpen ? (
        <div className="first-run-dialog-backdrop" onClick={() => setEvidenceOpen(false)}>
          <section aria-labelledby="first-run-evidence-title" aria-modal="true" className="first-run-dialog" onClick={(event) => event.stopPropagation()} role="dialog">
            <header><div><span>Local setup evidence</span><h2 id="first-run-evidence-title">First Run Proof</h2></div><button autoFocus aria-label="Close setup evidence" onClick={() => setEvidenceOpen(false)} type="button">Close</button></header>
            <nav aria-label="Setup evidence sections">
              {(["models", "permissions", "proof"] as const).map((section) => <button aria-pressed={evidenceSection === section} key={section} onClick={() => setEvidenceSection(section)} type="button">{section}</button>)}
            </nav>

            {evidenceSection === "models" ? (
              <div className="first-run-evidence-body">
                <section className="first-run-evidence-summary"><span>Selected runtime</span><strong>{runtimeLabel(props.firstRun.asr_runtime)}</strong><p>{props.firstRun.asr_runtime.detail}</p><small>Proof: {props.firstRun.asr_runtime.proof_requirement}</small></section>
                {props.firstRun.asr_candidates.map((model) => <button aria-pressed={model.selected} className={`first-run-model-option state-${model.state}`} key={model.id} onClick={() => props.onModelChange(model.id)} type="button"><span><strong>{model.id}</strong><small>{model.recommendation ?? `${model.runtime} / ${model.min_hw}`}</small></span><em>{model.state}</em></button>)}
                {props.firstRun.required_models.map((model) => (
                  <div className={`first-run-model-row state-${model.state}`} key={model.id}><div><strong>{model.id}</strong><small>{model.task}{model.lane ? ` / ${model.lane}` : ""} - {model.detail}</small></div><em>{model.state}</em>{model.state !== "ready" ? <><button disabled={props.modelPreflightPendingId !== null} onClick={() => props.onReviewModel(model.id)} type="button">{props.modelPreflightPendingId === model.id ? "Reviewing" : "Review"}</button><button disabled={!model.download_available || props.installingModelId !== null} onClick={() => props.onInstallModel(model.id)} type="button">{props.installingModelId === model.id ? "Installing" : "Install"}</button></> : null}</div>
                ))}
                {props.modelDownloadPreflight ? <section className={`first-run-preflight state-${props.modelDownloadPreflight.state}`}><strong>{props.modelDownloadPreflight.model_id}: {props.modelDownloadPreflight.available ? "Reviewed source available" : "Blocked"}</strong><p>{props.modelDownloadPreflight.detail}</p><small>{props.modelDownloadPreflight.blocked_reason ?? props.modelDownloadPreflight.operator_action}</small><code>{props.modelDownloadPreflight.expected_sha256 ?? "No reviewed sha256"}</code></section> : null}
                {props.modelDownloadPreflightIssue || props.modelInstallIssue ? <p className="first-run-evidence-issue">{props.modelDownloadPreflightIssue ?? props.modelInstallIssue}</p> : null}
                <button disabled={props.modelRefreshPending} onClick={props.onRefreshModels} type="button">{props.modelRefreshPending ? "Checking models" : "Recheck models"}</button>
              </div>
            ) : null}

            {evidenceSection === "permissions" ? (
              <div className="first-run-evidence-body">
                {displayedPermissions.map((permission) => <div className={`first-run-permission-row state-${permission.state}`} key={permission.id}><span><strong>{permission.label}</strong><small>{permission.detail}</small></span><em>{permissionLabel(permission.state)}</em><button disabled={props.permissionActionPendingId !== null || !selectedRuntime} onClick={() => props.onShowPermission(permission.id)} type="button">{selectedRuntime && props.permissionActionPendingId === permission.id ? "Loading" : permission.action_label}</button></div>)}
                {props.permissionActionOutcome ? <section className="first-run-permission-proof"><strong>{props.permissionActionOutcome.label}</strong><p>{props.permissionActionOutcome.manual_step}</p><small>Proof: {props.permissionActionOutcome.proof_requirement}</small><small>Evidence: {props.permissionActionOutcome.expected_evidence}</small><small>Ready boundary: {props.permissionActionOutcome.ready_boundary}</small>{props.permissionActionOutcome.proof_command ? <code>{props.permissionActionOutcome.proof_command}</code> : null}{props.permissionActionOutcome.settings_target && props.permissionActionOutcome.settings_open_label ? <button disabled={props.permissionSettingsPendingId !== null} onClick={() => props.onOpenPermissionSettings(props.permissionActionOutcome!.requirement_id)} type="button">{props.permissionSettingsPendingId ? "Opening" : props.permissionActionOutcome.settings_open_label}</button> : null}</section> : null}
                {props.permissionSettingsOutcome ? <p className="first-run-evidence-note">{props.permissionSettingsOutcome.opened ? "Settings panel requested. Return and refresh proof before readiness changes." : props.permissionSettingsOutcome.manual_step}</p> : null}
                {props.permissionActionIssue || props.permissionSettingsIssue ? <p className="first-run-evidence-issue">{props.permissionActionIssue ?? props.permissionSettingsIssue}</p> : null}
              </div>
            ) : null}

            {evidenceSection === "proof" ? (
              <div className="first-run-evidence-body">
                <section className="first-run-evidence-summary"><span>Backend next step</span><strong>{props.firstRun.next_step.title}</strong><p>{props.firstRun.next_step.detail}</p><small>Proof: {props.firstRun.next_step.proof_requirement}</small></section>
                <section className="first-run-proof-grid"><div><span>60-second target</span><strong>{timingLabel(props.firstRun.setup_timing)}</strong><small>{props.firstRun.setup_timing.target_ms / 1000}s target</small></div><div><span>First dictation</span><strong>{props.firstRun.first_dictation_completed ? "Persisted" : "Pending"}</strong><small>Requires a real Injected event</small></div><div><span>Local proof JSON</span><strong>{props.proofExport?.exported ? `${props.proofExport.item_count} items` : "Ready to export"}</strong><small>{props.proofExport?.json_path ?? "No cloud upload"}</small></div></section>
                <button disabled={props.exportingProof} onClick={props.onExportProof} type="button">{props.exportingProof ? "Exporting" : "Export local proof"}</button>
                {props.proofIssue ? <p className="first-run-evidence-issue">{props.proofIssue}</p> : null}
                {props.cleanupDialIssue || props.hotkeyModeIssue || props.hotkeyBindingIssue ? <p className="first-run-evidence-issue">{props.cleanupDialIssue ?? props.hotkeyModeIssue ?? props.hotkeyBindingIssue}</p> : null}
              </div>
            ) : null}
          </section>
        </div>
      ) : null}
    </section>
  );
}
