import { useCallback, useState } from "react";
import {
  DEFAULT_COCKPIT_LAYOUT_PRESET,
  type CockpitLayoutPreset,
} from "../components/CockpitChrome";

const STORAGE_KEY = "kaydence.cockpit.layoutPreset";

const VALID_PRESETS: readonly CockpitLayoutPreset[] = ["pillbar", "miccapsule", "stackedpanel"];

function isCockpitLayoutPreset(value: string | null): value is CockpitLayoutPreset {
  return value !== null && (VALID_PRESETS as readonly string[]).includes(value);
}

function readStoredLayoutPreset(): CockpitLayoutPreset {
  if (typeof window === "undefined") return DEFAULT_COCKPIT_LAYOUT_PRESET;
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    return isCockpitLayoutPreset(stored) ? stored : DEFAULT_COCKPIT_LAYOUT_PRESET;
  } catch {
    // localStorage may be unavailable (disabled storage, privacy mode). Fall
    // back to the default preset rather than throwing during render.
    return DEFAULT_COCKPIT_LAYOUT_PRESET;
  }
}

/**
 * Reads and persists the operator's chosen Cockpit layout preset.
 *
 * Persistence: `localStorage`, keyed by `STORAGE_KEY`. This codebase has no
 * existing settings-persistence pattern (no Tauri store plugin, no settings
 * context) — most durable settings shown in Setup (hotkey mode, cleanup
 * default, etc.) round-trip through the Rust backend via `invoke`, which is
 * out of scope for a pure-presentation concern like "which chrome layout am I
 * looking at". `localStorage` is the simplest mechanism that survives an app
 * restart without adding a new backend command.
 */
export function useLayoutPreset(): [CockpitLayoutPreset, (preset: CockpitLayoutPreset) => void] {
  const [preset, setPresetState] = useState<CockpitLayoutPreset>(() => readStoredLayoutPreset());

  const setPreset = useCallback((next: CockpitLayoutPreset) => {
    setPresetState(next);
    if (typeof window === "undefined") return;
    try {
      window.localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Best-effort persistence only; the in-memory state above still updates.
    }
  }, []);

  return [preset, setPreset];
}
