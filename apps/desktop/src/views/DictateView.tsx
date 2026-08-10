import {
  CockpitGlyph,
  type AppView,
  type CleanupDial,
  type CockpitIcon,
  type CockpitLayoutPreset,
  PanelHeading,
  SegmentedDial,
  StatusFooter,
} from "../components/CockpitChrome";
import { APP_VERSION } from "../version";
import "./DictateView.css";

export interface CockpitStatusItem {
  detail: string;
  icon: CockpitIcon;
  label: string;
  state: "ready" | "pending" | "issue";
  value: string;
}

export interface DictateHistoryItem {
  audio?: { byteLength: number; mimeType: string; src: string };
  hasAudio: boolean;
  id: string;
  status: string;
  summary: string;
  timestamp: string;
  title: string;
}

interface DictateViewProps {
  appName: string;
  autoPasteReady: boolean;
  cleanupDial: CleanupDial;
  cleanupIssue: string | null;
  cleanupPending: boolean;
  elapsed: string;
  historyExportNote: string | null;
  historyItems: DictateHistoryItem[];
  historyPlaybackIssue: string | null;
  historyPurgeNote: string | null;
  historyRefreshPending: boolean;
  /**
   * Which Cockpit chrome/composition to render (design-relock 2026-08-09,
   * Decision 1: Pill Bar / Mic Capsule / Stacked Panel, selectable in Setup).
   * Branches the topband flank arrangement below (see `topband` in the
   * component body) and is also mirrored onto the root section's
   * `data-cockpit-layout` attribute for CSS hooks. The status bar, transcript
   * panel, and all underlying data/handlers stay identical across presets —
   * only chrome/composition differs.
   */
  layoutPreset: CockpitLayoutPreset;
  markUrl: string;
  microphone: string;
  onCleanupChange: (dial: CleanupDial) => void;
  onClearLatest: (id: string) => void;
  onDeleteHistory: (id: string) => void;
  onExportHistory: (id: string) => void;
  onNavigate: (view: AppView) => void;
  onPlayHistory: (id: string) => void;
  onPurgeHistory: () => void;
  onRecordToggle?: () => void;
  onRefreshHistory: () => void;
  operational: boolean;
  /** Human-readable runtime OS label ("macOS"/"Windows"/"Linux"), appended
   * to the version string in the status footer. */
  osLabel: string;
  outputDestination: string;
  outputMethod: string;
  pendingHistoryAction: boolean;
  recording: boolean;
  statusItems: CockpitStatusItem[];
  transcript: string;
}

const cleanupCopy: Record<CleanupDial, string> = {
  raw: "No cleanup. Preserve recognized text exactly.",
  light: "Balanced cleanup for daily dictation.",
  full: "Opt-in rewrite using the selected profile.",
};

const waveformHeights = Array.from(
  { length: 58 },
  (_, index) => 8 + ((index * 11 + index * index * 3) % 30),
);

export function DictateView(props: DictateViewProps): JSX.Element {
  const words = props.transcript.trim() ? props.transcript.trim().split(/\s+/).length : 0;

  // L2/L3 — Capture is always the centered/primary control; only its flank
  // neighbors (Cleanup+Output vs. History) and their left/right order change
  // per layout preset. Built once so all three compositions render the same
  // recording state, handlers, and waveform data (data layer never forks).
  const captureNode = (
    <section
      aria-label="Capture"
      className={`cockpit-capture-panel${props.recording ? " is-recording" : ""}`}
      key="capture"
    >
      <PanelHeading>Capture</PanelHeading>
      <button
        aria-label={
          props.recording
            ? "Stop preview recording"
            : props.onRecordToggle
              ? "Start preview recording"
              : "Manual recording unavailable"
        }
        className="record-control"
        disabled={!props.onRecordToggle}
        onClick={props.onRecordToggle}
        type="button"
      ><CockpitGlyph name="square" /></button>
      <div className="record-state"><strong>{props.recording ? "Recording" : "Ready"}</strong><span>{props.elapsed}</span></div>
      <div className="cockpit-waveform" aria-label={props.recording ? "Preview recording waveform" : "Microphone idle"}>
        {waveformHeights.map((height, index) => <span key={index} style={{ height: `${height}px` }} />)}
      </div>
    </section>
  );

  const cleanupNode = (
    <section aria-label="Cleanup" className="cockpit-control-panel" key="cleanup">
      <PanelHeading>Cleanup</PanelHeading>
      <SegmentedDial disabled={props.cleanupPending} onChange={props.onCleanupChange} value={props.cleanupDial} />
      <p>{cleanupCopy[props.cleanupDial]}</p>
      <ul>
        {[
          "Remove filler words",
          "Fix capitalization",
          "Smart punctuation",
          "Normalize spacing",
          "Basic formatting",
        ].map((rule) => <li key={rule}><CockpitGlyph name="check" />{rule}</li>)}
      </ul>
      {props.cleanupIssue ? <small className="cockpit-note issue">{props.cleanupIssue}</small> : null}
      <button onClick={() => props.onNavigate("Cleanup")} type="button">Configure cleanups</button>
    </section>
  );

  const outputNode = (
    <section aria-label="Output" className="cockpit-control-panel cockpit-output-panel" key="output">
      <PanelHeading>Output</PanelHeading>
      <button onClick={() => props.onNavigate("Cleanup")} type="button">
        <span>{props.outputDestination}</span><small>{props.outputMethod}</small>
      </button>
      <label><span>Auto-paste ready</span><input checked={props.autoPasteReady} disabled readOnly type="checkbox" /></label>
    </section>
  );

  const historyNode = (
    <section aria-label="History" className="cockpit-history-panel" key="history">
      <div className="cockpit-panel-header"><PanelHeading>History</PanelHeading>
        <button disabled={props.historyRefreshPending} onClick={props.onRefreshHistory} type="button">
          {props.historyRefreshPending ? "Refreshing" : "Refresh"}
        </button>
      </div>
      {props.historyItems.length ? props.historyItems.slice(0, 3).map((item) => (
        <details className="cockpit-history-row" key={item.id}>
          <summary><span><strong>{item.title}</strong><small>{item.status}</small></span><time>{item.timestamp}</time></summary>
          <p>{item.summary}</p>
          {item.audio ? <audio controls preload="metadata"><source src={item.audio.src} type={item.audio.mimeType} /></audio> : null}
          <div>
            <button disabled={!item.hasAudio || props.pendingHistoryAction} onClick={() => props.onPlayHistory(item.id)} type="button">Play</button>
            <button disabled={props.pendingHistoryAction} onClick={() => props.onExportHistory(item.id)} type="button">Export</button>
            <button disabled={props.pendingHistoryAction} onClick={() => props.onDeleteHistory(item.id)} type="button">Delete</button>
          </div>
        </details>
      )) : <p className="cockpit-empty-state">No local sessions yet.</p>}
      {props.historyExportNote ? <small className="cockpit-note">{props.historyExportNote}</small> : null}
      {props.historyPurgeNote ? <small className="cockpit-note">{props.historyPurgeNote}</small> : null}
      {props.historyPlaybackIssue ? <small className="cockpit-note issue">{props.historyPlaybackIssue}</small> : null}
      <button className="cockpit-purge" disabled={!props.historyItems.length || props.pendingHistoryAction} onClick={props.onPurgeHistory} type="button">Purge local history</button>
    </section>
  );

  // L4 — full-width, bottom-half, dominant transcript surface (shared by all
  // three layouts; only the topband arrangement above it differs).
  const transcriptNode = (
    <section aria-label="Recent clean transcript" className="cockpit-transcript-panel cockpit-transcript-full">
      <PanelHeading>Recent clean transcript</PanelHeading>
      <p>{props.transcript || "No local transcript yet."}</p>
      <div><span>{words} words / {props.transcript.length} chars</span>
        <button
          disabled={props.historyItems.length === 0}
          onClick={() => props.historyItems[0] && props.onClearLatest(props.historyItems[0].id)}
          type="button"
        >Clear</button>
      </div>
    </section>
  );

  // L2/L3 — the differing part: which panels flank the centered Capture, and
  // on which side. Pill Bar: History (left) / Cleanup+Output (right). Mic
  // Capsule (default): Cleanup+Output (left) / History (right). Stacked
  // Panel: Capture widens instead of a third column, Cleanup+History stack
  // as one right-hand flank.
  let topband: JSX.Element;
  if (props.layoutPreset === "pillbar") {
    topband = (
      <div className="cockpit-topband">
        <div className="cockpit-flank">{historyNode}</div>
        {captureNode}
        <div className="cockpit-flank">{cleanupNode}{outputNode}</div>
      </div>
    );
  } else if (props.layoutPreset === "stackedpanel") {
    topband = (
      <div className="cockpit-topband">
        {captureNode}
        <div className="cockpit-flank cockpit-flank-stack">{cleanupNode}{historyNode}</div>
      </div>
    );
  } else {
    topband = (
      <div className="cockpit-topband">
        <div className="cockpit-flank">{cleanupNode}{outputNode}</div>
        {captureNode}
        <div className="cockpit-flank">{historyNode}</div>
      </div>
    );
  }

  return (
    <section
      aria-label={`${props.appName} Dictate cockpit`}
      className="dictate-board"
      data-cockpit-layout={props.layoutPreset}
    >
      <header className="cockpit-titlebar">
        <strong>Dictate</strong>
      </header>

      {topband}
      {transcriptNode}

      <StatusFooter
        appName={props.appName}
        microphone={props.microphone}
        onNavigateSetup={() => props.onNavigate("Setup")}
        operational={props.operational}
        statusItems={props.statusItems}
        version={`${APP_VERSION} · ${props.osLabel}`}
      />
    </section>
  );
}
