import {
  BottomNav,
  CockpitGlyph,
  type AppView,
  type CleanupDial,
  type CockpitIcon,
  type OsLane,
  PanelHeading,
  SegmentedDial,
  StatusCard,
  StatusFooter,
} from "../components/CockpitChrome";
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
  activeLane: OsLane;
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
  laneOptions: Array<{ id: OsLane; label: string }>;
  markUrl: string;
  microphone: string;
  onCleanupChange: (dial: CleanupDial) => void;
  onClearLatest: (id: string) => void;
  onDeleteHistory: (id: string) => void;
  onExportHistory: (id: string) => void;
  onLaneChange: (lane: OsLane) => void;
  onNavigate: (view: AppView) => void;
  onPlayHistory: (id: string) => void;
  onPurgeHistory: () => void;
  onRecordToggle?: () => void;
  onRefreshHistory: () => void;
  operational: boolean;
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
  return (
    <section className="dictate-board" aria-label={`${props.appName} dictation cockpit`}>
      <header className="cockpit-titlebar">
        <div><img src={props.markUrl} alt="" /><strong>{props.appName}</strong></div>
        <div className="cockpit-lane-switcher" aria-label="Operating system lane">
          {props.laneOptions.map((lane) => (
            <button
              aria-pressed={lane.id === props.activeLane}
              key={lane.id}
              onClick={() => props.onLaneChange(lane.id)}
              type="button"
            >{lane.label}</button>
          ))}
        </div>
      </header>

      <section className={`cockpit-record-strip${props.recording ? " is-recording" : ""}`}>
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

      <div className="cockpit-main-grid">
        <div className="cockpit-status-column">
          {props.statusItems.map((item) => <StatusCard key={item.label} {...item} />)}
          <BottomNav activeView="Dictate" onSelect={props.onNavigate} />
        </div>

        <div className="cockpit-center-column">
          <section className="cockpit-transcript-panel">
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

          <section className="cockpit-history-panel">
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
        </div>

        <aside className="cockpit-control-column">
          <section className="cockpit-control-panel">
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
          <section className="cockpit-control-panel cockpit-output-panel">
            <PanelHeading>Output</PanelHeading>
            <button onClick={() => props.onNavigate("Cleanup")} type="button">
              <span>{props.outputDestination}</span><small>{props.outputMethod}</small>
            </button>
            <label><span>Auto-paste ready</span><input checked={props.autoPasteReady} disabled readOnly type="checkbox" /></label>
          </section>
        </aside>
      </div>

      <StatusFooter appName={props.appName} microphone={props.microphone} operational={props.operational} version="v0.1.0" />
    </section>
  );
}
