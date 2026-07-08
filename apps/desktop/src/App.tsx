import { useMemo, useState } from "react";

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
  status: "Ready" | "Needs hardware" | "Next";
}

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
  { label: "Crash recovery", value: "28/28", detail: "local tests green" },
  { label: "Network audit", value: "0", detail: "unreviewed call sites" },
  { label: "Remote", value: "404", detail: "local-first recovery" },
];

const checklist: ChecklistItem[] = [
  { label: "Choose local ASR engine", status: "Next" },
  { label: "Grant OS permissions", status: "Needs hardware" },
  { label: "Set global hotkey", status: "Next" },
  { label: "Confirm privacy defaults", status: "Ready" },
  { label: "Start first dictation", status: "Next" },
];

// Presentation only. The Rust backend owns all logic (root AGENTS §9).
export function App(): JSX.Element {
  const [activeLane, setActiveLane] = useState<OsLane>("mac");
  const lane = useMemo(
    () => lanes.find((candidate) => candidate.id === activeLane) ?? lanes[0],
    [activeLane],
  );

  return (
    <main
      className="app-shell"
      style={{ "--accent": lane.accent } as React.CSSProperties}
    >
      <aside className="sidebar" aria-label="Kaydence navigation">
        <div className="brand-lockup">
          <img className="brand-mark" src={markUrl} alt="" />
          <div>
            <strong>Kaydence</strong>
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
            <p>No telemetry. No accounts. No cloud sync for core features.</p>
          </div>
        </div>
      </aside>

      <section className="workspace" aria-label="Kaydence cockpit">
        <header className="topbar">
          <div>
            <p className="eyebrow">P1 recovery build</p>
            <h1>Dictation cockpit</h1>
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
                <strong>Parakeet CPU</strong>
              </div>
              <div>
                <span>Target</span>
                <strong>Cursor</strong>
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
              <button type="button">Raw</button>
              <button className="selected" type="button">Light</button>
              <button type="button">Full</button>
            </div>
            <ul className="rule-list">
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
                <span>Fallback</span>
                <strong>Clipboard restore</strong>
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
                <li key={item.label}>
                  <span>{item.label}</span>
                  <strong>{item.status}</strong>
                </li>
              ))}
            </ol>
            <button className="primary-action" type="button">
              Start Dictating
            </button>
          </article>
        </section>
      </section>
    </main>
  );
}
