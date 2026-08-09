import {
  CockpitGlyph,
  type AppView,
  type CleanupDial,
  type OsLane,
  SegmentedDial,
} from "../components/CockpitChrome";
import "./CleanupView.css";

export interface CleanupLaneSpec {
  accent: string;
  fallbackDetail: string;
  fallbackMethod: string;
  gates: string[];
  id: OsLane;
  injection: string;
  label: string;
  primaryDetail: string;
  primaryMethod: string;
}

interface CleanupViewProps {
  appName: string;
  cleanupDial: CleanupDial;
  cleanupIssue: string | null;
  cleanupPending: boolean;
  deliveryMethod: "native" | "keystroke" | "clipboard_restore" | null;
  heldReason: "focus_changed" | "secure_field" | "no_target" | null;
  injectionFailure: string | null;
  lane: CleanupLaneSpec;
  lanes: CleanupLaneSpec[];
  markUrl: string;
  onCleanupChange: (dial: CleanupDial) => void;
  onLaneChange: (lane: OsLane) => void;
  onNavigate: (view: AppView) => void;
  permissionSummary: string;
  permissionsReady: boolean;
  previewFixture: boolean;
  targetApp: string | null;
  unknownFocus: string;
}

const rules = [
  "Smart punctuation",
  "Capitalize sentence start",
  "Remove filler words",
  "Collapse self-corrections",
  "Normalize whitespace",
  "Remove verbal clutter",
  "Apply custom dictionary",
];

const examples = [
  {
    label: "Punctuation",
    before: "this is a test lets see how it works",
    after: "This is a test. Let's see how it works.",
  },
  {
    label: "Filler removal",
    before: "um so basically like the point is uh this",
    after: "So basically, the point is this.",
  },
  {
    label: "Self-correction",
    before: "I want to go to the store no the market",
    after: "I want to go to the market.",
  },
];

function methodLabel(method: CleanupViewProps["deliveryMethod"]): string {
  if (method === "native") return "Native insertion proved";
  if (method === "keystroke") return "Keystroke fallback proved";
  if (method === "clipboard_restore") return "Clipboard restore proved";
  return "No delivery proof yet";
}

export function CleanupView(props: CleanupViewProps): JSX.Element {
  const cleanupEnabled = props.cleanupDial !== "raw";
  const healthState = props.injectionFailure
    ? "issue"
    : props.deliveryMethod
      ? "ready"
      : "pending";
  const healthLabel = props.injectionFailure
    ? "Issue"
    : props.deliveryMethod
      ? "Good"
      : props.heldReason
        ? "Held safely"
        : "Pending proof";
  const gateStates = [
    props.permissionsReady,
    true,
    Boolean(props.deliveryMethod || props.heldReason),
    Boolean(props.deliveryMethod),
  ];

  return (
    <section className="cleanup-board" aria-label={`${props.appName} cleanup and injection`}>
      <header className="cleanup-titlebar">
        <div><img src={props.markUrl} alt="" /><strong>{props.appName}</strong></div>
        <div className="cleanup-lane-switcher" aria-label="Operating system lane">
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

      <div className="cleanup-heading">
        <div><span>Screen family 03</span><h1>Cleanup &amp; Injection</h1></div>
        <span>{props.previewFixture ? "Preview fixture" : "Backend state"}</span>
      </div>

      <div className="cleanup-main-grid">
        <article className="cleanup-panel cleanup-rules-panel">
          <section className="cleanup-dial-section">
            <h2>Cleanup Dial &amp; Output Rules</h2>
            <SegmentedDial
              disabled={props.cleanupPending}
              onChange={props.onCleanupChange}
              value={props.cleanupDial}
            />
            <p>
              {props.cleanupDial === "raw"
                ? "Raw preserves recognized text exactly."
                : props.cleanupDial === "light"
                  ? "Light applies the deterministic daily cleanup floor."
                  : "Full is explicit opt-in and currently uses the verified Light rule floor."}
            </p>
            {props.cleanupIssue ? <small className="cleanup-issue">{props.cleanupIssue}</small> : null}
          </section>

          <section className="cleanup-rule-section">
            <h3>Output Rules</h3>
            <div className="cleanup-rule-list">
              {rules.map((rule) => {
                const enabled = cleanupEnabled && rule !== "Remove verbal clutter";
                return (
                  <label key={rule}>
                    <input checked={enabled} disabled readOnly type="checkbox" />
                    <span>{rule}</span>
                    <small>{enabled ? "Profile rule" : "Not applied"}</small>
                  </label>
                );
              })}
            </div>
          </section>

          <section className="cleanup-example-section">
            <h3>Before / After Examples</h3>
            <div className="cleanup-examples">
              {examples.map((example) => (
                <div key={example.label}>
                  <strong>{example.label}</strong>
                  <p><span>Before</span>{example.before}</p>
                  <p><span>After</span>{cleanupEnabled ? example.after : example.before}</p>
                </div>
              ))}
            </div>
          </section>

          <section className="cleanup-destination-section">
            <div><h3>Output Destination (Per App)</h3><small>{props.targetApp ? "Latest bound target" : "Awaiting first captured target"}</small></div>
            <button disabled type="button" title="Per-app profile editing lands with the P2 profile unit">
              <CockpitGlyph name="target" />
              <span>{props.targetApp ?? "No captured app yet"}</span>
            </button>
            <button disabled type="button" title="Per-app profile editing is not available in this build">Configure</button>
          </section>
        </article>

        <aside className="cleanup-panel cleanup-injection-panel">
          <section className="cleanup-method-section">
            <div><h2>Injection Method &amp; Status</h2><span className={props.permissionsReady ? "ready" : "pending"}>{props.permissionsReady ? "Enabled" : "Setup required"}</span></div>
            <strong>{props.lane.label} Injection</strong>
            <p>{props.lane.primaryMethod}</p>
            <small>{props.lane.primaryDetail}</small>
            <dl><div><dt>Status</dt><dd>{props.permissionsReady ? "Active" : "Permission proof pending"}</dd></div><div><dt>Permissions</dt><dd>{props.permissionSummary}</dd></div></dl>
            <button onClick={() => props.onNavigate("Setup")} type="button">Check permissions</button>
          </section>

          <section className="cleanup-fallback-section">
            <strong>Fallback: {props.lane.fallbackMethod}</strong>
            <p>{props.lane.fallbackDetail}</p>
            <small>{props.unknownFocus}</small>
          </section>

          <section className="cleanup-health-section">
            <div><strong>Injection Health</strong><span className={healthState}>{healthLabel}</span></div>
            <p>{props.injectionFailure ?? methodLabel(props.deliveryMethod)}</p>
          </section>

          <section className="cleanup-latency-section">
            <h3>Latency Budget</h3>
            <div className="cleanup-budget-grid">
              <div><span>Capture</span><strong>&le; 50ms</strong></div>
              <div><span>Partial</span><strong>&le; 300ms</strong></div>
              <div><span>CPU inject</span><strong>&le; 1200ms</strong></div>
            </div>
            <div className="cleanup-current-p95"><span>Reference p95</span><strong>{props.previewFixture ? "Fixture 7 / 120 / 5ms" : "Pending P1-G3"}</strong></div>
          </section>

          <section className="cleanup-gates-section">
            <h3>Test Gates</h3>
            {props.lane.gates.slice(0, 4).map((gate, index) => (
              <div key={gate}>
                <span className={gateStates[index] ? "ready" : "pending"}>{gateStates[index] ? <CockpitGlyph name="check" /> : null}</span>
                <strong>{gate}</strong>
                <small>{gateStates[index] ? "OK" : "Pending"}</small>
              </div>
            ))}
            <div className="cleanup-status-legend" aria-label="Status legend">
              <span><span className="dot dot--good" aria-hidden="true" />Good</span>
              <span><span className="dot dot--accent" aria-hidden="true" />Available</span>
              <span><span className="dot dot--idle" aria-hidden="true" />Idle</span>
              <span><span className="dot dot--danger" aria-hidden="true" />Issue</span>
            </div>
          </section>
        </aside>
      </div>

      <footer className="cleanup-actions">
        <button disabled={props.cleanupPending} onClick={() => props.onCleanupChange("light")} type="button">Reset to Defaults</button>
        <span>{props.cleanupPending ? "Saving changes" : props.cleanupIssue ? "Save blocked" : "Changes saved"}</span>
        <button disabled={props.cleanupPending} onClick={() => props.onCleanupChange(props.cleanupDial)} type="button">Save Changes</button>
      </footer>
    </section>
  );
}
