import type { ReactNode } from "react";

export type AppView = "Dictate" | "Cleanup" | "Privacy" | "Setup";
export type CleanupDial = "raw" | "light" | "full";
export type OsLane = "mac" | "windows" | "linux";

/**
 * Cockpit layout preset (design-relock 2026-08-09, Decision 1): the three
 * gauntlet-round-2 compositions ship as selectable Setup options instead of
 * picking one winner. `miccapsule` is the confirmed operator default.
 * Chrome/composition differs per preset; underlying data/state does not.
 */
export type CockpitLayoutPreset = "pillbar" | "miccapsule" | "stackedpanel";

export const DEFAULT_COCKPIT_LAYOUT_PRESET: CockpitLayoutPreset = "miccapsule";

export const COCKPIT_LAYOUT_PRESET_OPTIONS: ReadonlyArray<{
  id: CockpitLayoutPreset;
  label: string;
}> = [
  { id: "pillbar", label: "Pill Bar" },
  { id: "miccapsule", label: "Mic Capsule" },
  { id: "stackedpanel", label: "Stacked Panel" },
];
export type CockpitIcon =
  | "waveform"
  | "history"
  | "cleanup"
  | "privacy"
  | "settings"
  | "cpu"
  | "target"
  | "keyboard"
  | "database"
  | "square"
  | "check";

export interface NavRailItem {
  icon?: CockpitIcon;
  label: string;
  phase?: string;
  view?: AppView;
}

const glyphContents: Record<CockpitIcon, JSX.Element> = {
  waveform: <><path d="M2 10v3"/><path d="M6 6v11"/><path d="M10 3v18"/><path d="M14 8v7"/><path d="M18 5v13"/><path d="M22 10v3"/></>,
  history: <><path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/></>,
  cleanup: <><path d="m15 4 5 5L7 22H2v-5Z"/><path d="m14 6 4 4"/><path d="M6 3v4"/><path d="M4 5h4"/><path d="M19 16v4"/><path d="M17 18h4"/></>,
  privacy: <><path d="M20 13c0 5-3.5 7.5-8 9-4.5-1.5-8-4-8-9V5l8-3 8 3Z"/><path d="m9 12 2 2 4-4"/></>,
  settings: <><path d="M4 21v-7"/><path d="M4 10V3"/><path d="M12 21v-9"/><path d="M12 8V3"/><path d="M20 21v-5"/><path d="M20 12V3"/><path d="M2 14h4"/><path d="M10 8h4"/><path d="M18 16h4"/></>,
  cpu: <><rect width="16" height="16" x="4" y="4" rx="2"/><rect width="6" height="6" x="9" y="9" rx="1"/><path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3"/></>,
  target: <><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/></>,
  keyboard: <><rect width="20" height="16" x="2" y="4" rx="2"/><path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M7 16h10"/></>,
  database: <><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14c0 1.7 4 3 9 3s9-1.3 9-3V5M3 12c0 1.7 4 3 9 3s9-1.3 9-3"/></>,
  square: <rect width="14" height="14" x="5" y="5" rx="2" fill="currentColor" stroke="none" />,
  check: <path d="m5 12 4 4L19 6" />,
};

export function CockpitGlyph({ name }: { name: CockpitIcon }): JSX.Element {
  return (
    <svg
      aria-hidden="true"
      className="cockpit-glyph"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={name === "check" ? 2.5 : 2}
      viewBox="0 0 24 24"
    >
      {glyphContents[name]}
    </svg>
  );
}

export function NavRail({
  activeView,
  appName,
  compact = false,
  footerTitle = "Local only",
  items,
  markUrl,
  onSelect,
  privacySummary,
}: {
  activeView: AppView;
  appName: string;
  compact?: boolean;
  footerTitle?: string;
  items: NavRailItem[];
  markUrl: string;
  onSelect: (view: AppView) => void;
  privacySummary: string;
}): JSX.Element {
  return (
    <aside className={compact ? "sidebar compact-sidebar" : "sidebar"} aria-label={`${appName} navigation`}>
      <div className="brand-lockup">
        <img className="brand-mark" src={markUrl} alt="" />
        <div><strong>{appName}</strong><span>Local voice platform</span></div>
      </div>
      <nav className="nav-list">
        {items.map((item) => (
          <button
            aria-current={item.view === activeView ? "page" : undefined}
            className={item.view === activeView ? "nav-item active" : "nav-item"}
            disabled={!item.view}
            key={item.label}
            onClick={() => item.view && onSelect(item.view)}
            type="button"
          >
            <span className="nav-item-label">
              {item.icon ? <CockpitGlyph name={item.icon} /> : null}
              <span>{item.label}</span>
            </span>
            {item.phase ? <small>{item.phase}</small> : null}
          </button>
        ))}
      </nav>
      <div className="sidebar-card">
        <span className="status-dot" aria-hidden="true" />
        <div><strong>{footerTitle}</strong><p>{privacySummary}</p></div>
      </div>
    </aside>
  );
}

export function SegmentedDial({
  disabled,
  onChange,
  value,
}: {
  disabled: boolean;
  onChange: (dial: CleanupDial) => void;
  value: CleanupDial;
}): JSX.Element {
  return (
    <div className="cockpit-dial" role="group" aria-label="Cleanup level">
      {(["raw", "light", "full"] as const).map((dial) => (
        <button
          aria-pressed={value === dial}
          disabled={disabled}
          key={dial}
          onClick={() => onChange(dial)}
          type="button"
        >{dial}</button>
      ))}
    </div>
  );
}

export function BottomNav({
  activeView,
  onSelect,
}: {
  activeView: AppView;
  onSelect: (view: AppView) => void;
}): JSX.Element {
  const items: Array<{ icon: CockpitIcon; label: string; view?: AppView }> = [
    { icon: "waveform", label: "Dictate", view: "Dictate" },
    { icon: "history", label: "History" },
    { icon: "cleanup", label: "Cleanup", view: "Cleanup" },
    { icon: "privacy", label: "Privacy", view: "Privacy" },
    { icon: "settings", label: "Setup", view: "Setup" },
  ];
  return (
    <nav className="cockpit-bottom-nav" aria-label="Cockpit views">
      {items.map((item) => (
        <button
          aria-label={item.label}
          aria-current={item.view === activeView ? "page" : undefined}
          disabled={!item.view}
          key={item.label}
          onClick={() => item.view && onSelect(item.view)}
          type="button"
        ><CockpitGlyph name={item.icon} /></button>
      ))}
    </nav>
  );
}

/**
 * Status-bar pill datum. Structurally identical to (but declared without
 * importing, to avoid a views -> components -> views cycle) DictateView's
 * `CockpitStatusItem` — Engine / Target App / Global Hotkey / Privacy / WAL
 * Recovery, per design-relock Decision 1 (L1: status demoted from a column
 * card into clickable footer pills, common to all three Cockpit layouts).
 */
export interface StatusPillItem {
  detail: string;
  icon: CockpitIcon;
  label: string;
  state: "ready" | "pending" | "issue";
  value: string;
}

/**
 * Common Cockpit status bar (design-relock Decisions 2-6, applied identically
 * across Pill Bar / Mic Capsule / Stacked Panel): a brand block with the
 * version underneath on the left; an "all systems operational" indicator, a
 * decorative mic-level meter centered in the negative space, and the five
 * status items as clickable pill popovers (native <details>/<summary> — no
 * extra UI state needed) on the right.
 */
export function StatusFooter({
  appName,
  microphone,
  onNavigateSetup,
  operational,
  statusItems,
  version,
}: {
  appName: string;
  microphone: string;
  onNavigateSetup: () => void;
  operational: boolean;
  statusItems: StatusPillItem[];
  version: string;
}): JSX.Element {
  return (
    <footer className="cockpit-statusbar">
      <div className="cockpit-sb-left">
        <strong>{appName} Free</strong>
        <span>{version}</span>
      </div>
      <div className="cockpit-sb-right">
        <strong className={operational ? "operational" : "attention"}>
          {operational ? "All systems operational" : "Setup action required"}
        </strong>
        <span aria-label={`Mic level — ${microphone}`} className="cockpit-mic-meter" title={`Mic level — ${microphone}`}>
          {Array.from({ length: 10 }, (_, index) => <i key={index} />)}
        </span>
        <div className="cockpit-sb-pills">
          {statusItems.map((item) => (
            <details className="cockpit-sb-pill" key={item.label}>
              <summary>
                <i className={`cockpit-sb-pill-dot state-${item.state}`} aria-hidden="true" />
                {item.label}
              </summary>
              <div className="cockpit-sb-pop">
                <strong>{item.value}</strong>
                <small>{item.detail}</small>
                <button onClick={onNavigateSetup} type="button">Change in Setup</button>
              </div>
            </details>
          ))}
        </div>
      </div>
    </footer>
  );
}

export function PanelHeading({ children }: { children: ReactNode }): JSX.Element {
  return <h2 className="cockpit-panel-heading">{children}</h2>;
}
