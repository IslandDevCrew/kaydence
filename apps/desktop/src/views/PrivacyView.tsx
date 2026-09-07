import { useState } from "react";
import { Modal } from "../components/Modal";
import {
  CockpitGlyph,
  type AppView,
  type OsLane,
} from "../components/CockpitChrome";
import "./PrivacyView.css";

export interface PrivacyLaneSpec {
  accent: string;
  id: OsLane;
  label: string;
}

export type PrivacyPermissionState =
  | "ready"
  | "needs_hardware"
  | "needs_review"
  | "blocked"
  | "not_requested";

export interface PrivacyPermissionItem {
  detail: string;
  id: string;
  label: string;
  state: PrivacyPermissionState;
}

export interface PrivacyAuditItem {
  detail: string;
  id: string;
  outcome: string;
  timestamp: string;
  title: string;
}

interface PrivacyViewProps {
  appName: string;
  auditItems: PrivacyAuditItem[];
  contextEnabled: boolean;
  engineLabel: string;
  historyRetentionDays: number;
  lane: PrivacyLaneSpec;
  lanes: PrivacyLaneSpec[];
  markUrl: string;
  ocrEnabled: boolean;
  onLaneChange: (lane: OsLane) => void;
  onNavigate: (view: AppView) => void;
  operational: boolean;
  permissionRequirements: PrivacyPermissionItem[];
  previewFixture: boolean;
  runtimeLane: OsLane;
}

const referencePermissions: Record<OsLane, PrivacyPermissionItem[]> = {
  mac: [
    {
      id: "microphone",
      label: "Microphone",
      state: "needs_review",
      detail: "Requires capture proof on the target Mac.",
    },
    {
      id: "accessibility",
      label: "Accessibility",
      state: "needs_review",
      detail: "Requires native insert and secure-field proof.",
    },
    {
      id: "input_monitoring",
      label: "Input Monitoring",
      state: "needs_review",
      detail: "Requires a live global-hotkey proof.",
    },
  ],
  windows: [
    {
      id: "microphone",
      label: "Microphone",
      state: "needs_review",
      detail: "Requires capture proof on the target PC.",
    },
    {
      id: "uia_focus",
      label: "UI Automation",
      state: "needs_review",
      detail: "Requires focus and password-field proof.",
    },
    {
      id: "sendinput",
      label: "Keyboard fallback",
      state: "needs_review",
      detail: "Requires fallback typing proof after the focus gate.",
    },
  ],
  linux: [
    {
      id: "microphone",
      label: "Microphone",
      state: "needs_review",
      detail: "Requires PipeWire or PulseAudio capture proof.",
    },
    {
      id: "accessibility_bus",
      label: "AT-SPI bus",
      state: "needs_review",
      detail: "Requires a live desktop-session focus proof.",
    },
    {
      id: "uinput",
      label: "uinput path",
      state: "needs_hardware",
      detail: "Requires operator-in-the-loop field validation.",
    },
  ],
};

function permissionStateLabel(state: PrivacyPermissionState): string {
  switch (state) {
    case "ready":
      return "Ready";
    case "needs_hardware":
      return "Hardware proof";
    case "needs_review":
      return "Review";
    case "blocked":
      return "Blocked";
    case "not_requested":
      return "Not requested";
  }
}

export function PrivacyView(props: PrivacyViewProps): JSX.Element {
  const [constitutionOpen, setConstitutionOpen] = useState(false);
  const selectedRuntime = props.lane.id === props.runtimeLane;
  const permissionRows = selectedRuntime
    ? props.permissionRequirements
    : referencePermissions[props.lane.id];
  const displayedPermissions: PrivacyPermissionItem[] = [
    ...permissionRows,
    {
      id: "screen_capture",
      label: "Screen capture",
      state: "not_requested",
      detail: "Kaydence does not request cloud or OS screen recording.",
    },
  ];
  const auditRows = props.auditItems.slice(0, 4);

  const readerRows = [
    {
      id: "tier-one",
      label: "Tier 1 Accessibility (Text Read)",
      detail: "Visible text through OS accessibility APIs only.",
      status: props.contextEnabled ? "Opted in" : "Off - P3 opt-in",
      state: props.contextEnabled ? "ready" : "pending",
    },
    {
      id: "tier-two",
      label: "Tier 2 On-Device OCR",
      detail: "Non-selectable text; never sent to a cloud service.",
      status: props.ocrEnabled ? "Opted in" : "Off - P3",
      state: props.ocrEnabled ? "ready" : "pending",
    },
    {
      id: "secure-fields",
      label: "Secure-Field Detection",
      detail: "Password and secure targets are held, not injected.",
      status: "Protected",
      state: "ready",
    },
    {
      id: "memory-buffer",
      label: "In-Memory Context Buffer",
      detail: "Cleared with the active task; no cloud persistence.",
      status: props.contextEnabled ? "Memory only" : "Inactive",
      state: props.contextEnabled ? "ready" : "pending",
    },
    {
      id: "network-audit",
      label: "Network Audit",
      detail: "Unreviewed network call sites fail the source gate.",
      status: "Source-level",
      state: "ready",
    },
    {
      id: "audit-policy",
      label: "Audit Policy",
      detail: "This view uses local event metadata, never transcript text.",
      status: "Metadata only",
      state: "ready",
    },
  ];
  const contextRows = readerRows.filter((row) => row.id !== "network-audit");
  const networkAuditRow = readerRows.find((row) => row.id === "network-audit") ?? null;

  return (
    <section className="privacy-board" aria-label={`${props.appName} privacy and context`}>
      <header className="privacy-titlebar">
        <div><img src={props.markUrl} alt="" /><strong>{props.appName}</strong></div>
        <div className="privacy-lane-switcher" aria-label="Operating system lane">
          {props.lanes.map((lane) => (
            <button
              aria-pressed={lane.id === props.lane.id}
              key={lane.id}
              onClick={() => props.onLaneChange(lane.id)}
              type="button"
            >{lane.label}</button>
          ))}
        </div>
      </header>

      <div className="privacy-heading">
        <span className="privacy-heading-icon"><CockpitGlyph name="privacy" /></span>
        <div><span>Screen family 07</span><h1>Privacy &amp; Context</h1><small>Your voice. Your device. Your control.</small></div>
        <button
          aria-expanded={constitutionOpen}
          onClick={() => setConstitutionOpen(true)}
          type="button"
        >Privacy Constitution</button>
      </div>

      <section className="privacy-constitution" aria-label="Privacy constitution summary">
        <header><strong>Privacy Constitution</strong><span>Always on</span></header>
        <div>
          <article>
            <CockpitGlyph name="check" /><strong>No Transmission</strong><small>Processing stays on this device.</small>
            <em className="privacy-enforced"><span className="privacy-enforced-dot" aria-hidden="true" />Enforced</em>
          </article>
          <article>
            <CockpitGlyph name="database" /><strong>Local History Only</strong><small>Audio and transcripts stay local. {props.historyRetentionDays}-day retention policy.</small>
            <em className="privacy-enforced"><span className="privacy-enforced-dot" aria-hidden="true" />Enforced</em>
          </article>
          <article>
            <CockpitGlyph name="privacy" /><strong>Secure Fields Blocked</strong><small>Protected targets are refused.</small>
            <em className="privacy-enforced"><span className="privacy-enforced-dot" aria-hidden="true" />Enforced</em>
          </article>
          <article>
            <CockpitGlyph name="cpu" /><strong>Local-Only Models</strong><small>{props.engineLabel}</small>
            <em className="privacy-enforced"><span className="privacy-enforced-dot" aria-hidden="true" />Enforced</em>
          </article>
        </div>
      </section>

      <div className="privacy-main-grid">
        <article className="privacy-panel privacy-reader-panel">
          <header><div><h2>Context Reader</h2><small>Backend settings and policy state</small></div><span>{props.contextEnabled ? "Opted in" : "Default off"}</span></header>
          <div className="privacy-reader-list">
            {contextRows.map((row) => (
              <div className="privacy-reader-row" key={row.id}>
                <span className={`privacy-state-dot state-${row.state}`} aria-hidden="true" />
                <div><strong>{row.label}</strong><small>{row.detail}</small></div>
                <em className={`state-${row.state}`}>{row.status}</em>
                {row.id === "memory-buffer" ? (
                  <div className="privacy-buffer" aria-hidden="true">
                    <span className="privacy-buffer-label">Live buffer</span>
                    <span className="privacy-buffer-mask">•••••• •••• ••• •••••••• ••••• •• ••• •••••• ••••••</span>
                  </div>
                ) : null}
              </div>
            ))}
          </div>
        </article>

        <div className="privacy-side-col">
          <aside className="privacy-panel privacy-permissions-panel">
            <header>
              <div><h2>Permissions</h2><small>{props.lane.label}{selectedRuntime ? " runtime" : " reference lane"}</small></div>
              <span>{selectedRuntime ? (props.previewFixture ? "Fixture" : "Backend") : "Reference"}</span>
            </header>
            <div className="privacy-permission-list">
              {displayedPermissions.map((permission) => (
                <div className={`privacy-permission-row state-${permission.state}`} key={permission.id}>
                  <span><CockpitGlyph name={permission.state === "ready" ? "check" : "privacy"} /></span>
                  <div><strong>{permission.label}</strong><small>{permission.detail}</small></div>
                  <em>{permissionStateLabel(permission.state)}</em>
                </div>
              ))}
            </div>
            <div className="privacy-permission-legend" aria-hidden="true">
              <span><span className="privacy-legend-dot state-ready" />Ready</span>
              <span><span className="privacy-legend-dot state-needs_review" />Needs review</span>
              <span><span className="privacy-legend-dot state-blocked" />Blocked</span>
              <span><span className="privacy-legend-dot state-not_requested" />Not requested</span>
            </div>
            <button onClick={() => props.onNavigate("Setup")} type="button">Review in Setup</button>
          </aside>

          {networkAuditRow ? (
            <aside className="privacy-panel privacy-network-panel" aria-label="Network audit">
              <header><div><h2>Network Audit</h2><small>{networkAuditRow.detail}</small></div><span>{networkAuditRow.status}</span></header>
              <div className="privacy-network-hero">
                <span className="privacy-shield-ring" aria-hidden="true"><CockpitGlyph name="privacy" /></span>
                <div className="privacy-network-count"><strong>Source</strong><span>allowlist audit</span></div>
              </div>
              <small className="privacy-network-note">Live connections not measured here.</small>
            </aside>
          ) : null}
        </div>
      </div>

      <section className="privacy-panel privacy-audit-panel">
        <header><div><h2>Audit Log</h2><small>Recent local event metadata</small></div><button onClick={() => props.onNavigate("Dictate")} type="button">View History</button></header>
        <div className="privacy-audit-list">
          {auditRows.length ? auditRows.map((item) => (
            <div className="privacy-audit-row" key={item.id}>
              <time>{item.timestamp}</time>
              <div><strong>{item.title}</strong><small>{item.detail}</small></div>
              <em>{item.outcome}</em>
            </div>
          )) : (
            <p>No local session metadata yet.</p>
          )}
        </div>
      </section>

      <footer className="privacy-footer">
        <span className="privacy-footer-dot" aria-hidden="true" /><strong>Model: {props.engineLabel}</strong>
        <span><CockpitGlyph name="privacy" />Local only</span>
        <span className={props.operational ? "ready" : "pending"}>{props.operational ? "Runtime ready" : "Setup proof pending"}</span>
      </footer>

      {constitutionOpen ? (
        <Modal className="privacy-dialog-backdrop" labelledBy="privacy-constitution-title" onDismiss={() => setConstitutionOpen(false)}>
          <section
            className="privacy-dialog"
          >
            <header><div><span>Kaydence Core</span><h2 id="privacy-constitution-title">Privacy Constitution</h2></div><button aria-label="Close privacy constitution" onClick={() => setConstitutionOpen(false)} type="button">Close</button></header>
            <dl>
              <div><dt>No transmission by default</dt><dd>No telemetry, account stream, or unreviewed egress leaves the machine.</dd></div>
              <div><dt>Local persistence is visible</dt><dd>Audio and transcripts are retained locally. History refresh applies your {props.historyRetentionDays}-day retention policy; export, delete, and purge remain available.</dd></div>
              <div><dt>Context is explicit opt-in</dt><dd>Accessibility context and OCR remain off in P1. Future P3 context is memory-bound and secure-field aware.</dd></div>
              <div><dt>Secure fields fail closed</dt><dd>Known password and secure targets are held instead of receiving dictation.</dd></div>
            </dl>
          </section>
        </Modal>
      ) : null}
    </section>
  );
}
