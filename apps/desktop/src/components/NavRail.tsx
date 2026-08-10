import { type AppView, CockpitGlyph, type CockpitIcon } from "./CockpitChrome";

/**
 * LOCKED nav invariant — docs/design/DESIGN_LANGUAGE_V2_LOCK.md ("One NAV
 * constant"). Order and copy are fixed: Dictate, Whisper-Ahead, Cleanup,
 * Relay, Voiceprint, Conductor, Privacy, Dictionary, Analytics, Setup.
 * Do not reorder, rename, or fork this list per screen. Items without a
 * `view` are roadmap placeholders (rendered disabled with a phase badge)
 * because their screens have not shipped yet.
 */
interface NavRailEntry {
  icon: CockpitIcon;
  label: string;
  phase?: string;
  view?: AppView;
}

export const NAV_RAIL_ITEMS: readonly NavRailEntry[] = [
  { icon: "waveform", label: "Dictate", view: "Dictate" },
  { icon: "waveform", label: "Whisper-Ahead", phase: "P3" },
  { icon: "cleanup", label: "Cleanup", view: "Cleanup" },
  { icon: "square", label: "Relay", phase: "P4" },
  { icon: "waveform", label: "Voiceprint", phase: "P4" },
  { icon: "cpu", label: "Conductor", phase: "P4" },
  { icon: "privacy", label: "Privacy", view: "Privacy" },
  { icon: "database", label: "Dictionary", phase: "P2" },
  { icon: "history", label: "Analytics", phase: "P3" },
  { icon: "settings", label: "Setup", view: "Setup" },
];

export interface NavRailProps {
  /** Currently active screen, so the matching item can render as selected. */
  activeView: AppView;
  appName: string;
  /**
   * Icon-rail footprint (126px, matches the existing Cleanup/Privacy shells).
   * Defaults to true — every screen using this component today renders the
   * compact treatment. Pass `false` to opt into the full 236px label rail
   * from the locked reference if a future screen wants it.
   */
  compact?: boolean;
  footerTitle: string;
  markUrl: string;
  /** Screen-switch callback — same shape as the `onNavigate` prop already
   * threaded through DictateView/CleanupView/PrivacyView/FirstRunView. */
  onNavigate: (view: AppView) => void;
  privacySummary: string;
}

export function NavRail({
  activeView,
  appName,
  compact = true,
  footerTitle,
  markUrl,
  onNavigate,
  privacySummary,
}: NavRailProps): JSX.Element {
  return (
    <aside className={compact ? "sidebar compact-sidebar" : "sidebar"} aria-label={`${appName} navigation`}>
      <div className="brand-lockup">
        <img className="brand-mark" src={markUrl} alt="" />
        <div><strong>{appName}</strong><span>Local voice platform</span></div>
      </div>
      <nav className="nav-list">
        {NAV_RAIL_ITEMS.map((item) => (
          <button
            aria-current={item.view === activeView ? "page" : undefined}
            className={item.view === activeView ? "nav-item active" : "nav-item"}
            disabled={!item.view}
            key={item.label}
            onClick={() => item.view && onNavigate(item.view)}
            type="button"
          >
            <span className="nav-item-label">
              <CockpitGlyph name={item.icon} />
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
