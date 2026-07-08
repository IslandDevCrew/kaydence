//! Global hotkey capture coordination (push-to-talk + toggle). The OS hotkey
//! *registration* (tauri global-shortcut plugin) is a dependency add gated by a
//! future ADR; this module implements the platform-agnostic **capture state
//! machine** that every registration path feeds — the production answer to
//! Pitfall P1 (hotkey races truncating short utterances) adopted from Handy's
//! `TranscriptionCoordinator` (ADR-0004 study): a single serialized owner with a
//! debounce, min-capture floor, and tail buffer. Modelled as a pure state
//! machine (time injected as `u64` ms) so it is unit-tested without threads or a
//! real clock.
//!
//! Illegal transitions are unrepresentable ([`CaptureState`]). No stage imports
//! another's internals — communicate only via `SessionEvent`.
#![allow(dead_code)]

/// Which activation gesture the hotkey uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyMode {
    /// Hold to capture; release (plus tail) finalizes.
    PushToTalk,
    /// Press to start; press again to finalize. Releases are ignored.
    Toggle,
}

/// Capture timing rules. Defaults from PRD P0-1 + Pitfall P1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureConfig {
    /// Captures shorter than this are discarded as accidental taps.
    pub min_capture_ms: u64,
    /// Keep capturing this long after the stop gesture so trailing audio is not
    /// clipped.
    pub tail_buffer_ms: u64,
    /// Presses within this window of the last press are treated as key-bounce /
    /// auto-repeat and ignored (the anti-double-start guard).
    pub debounce_ms: u64,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            min_capture_ms: 250,
            tail_buffer_ms: 300,
            debounce_ms: 30,
        }
    }
}

/// The capture lifecycle. Only these states exist, so a "double start" or a
/// "stop while idle" cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    /// Not capturing.
    Idle,
    /// Actively capturing since `started_ms`; `last_press_ms` powers debounce.
    Capturing { started_ms: u64, last_press_ms: u64 },
    /// Stop gesture seen; still capturing the tail until `ends_ms`.
    Finalizing { started_ms: u64, ends_ms: u64 },
}

/// An input edge fed to the coordinator (from the OS hotkey layer or a timer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Press {
        at_ms: u64,
    },
    Release {
        at_ms: u64,
    },
    /// A timer tick that drives the tail-buffer close.
    Tick {
        at_ms: u64,
    },
}

/// The effect the runtime should perform in response to a signal. The runtime
/// owns audio/WAL; the coordinator only decides *when* to start/stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nothing to do (state may still have changed internally).
    None,
    /// Begin capture → audio thread on, WAL open (audio persists before ASR).
    StartCapture,
    /// Tail elapsed → stop capture and hand the session downstream.
    FinalizeCapture,
    /// Capture was too short (< `min_capture_ms`) → drop it as an accidental tap.
    DiscardCapture,
}

/// The single serialized owner of capture state. One instance processes every
/// hotkey edge in order — the Pitfall-P1 guarantee that rapid presses cannot
/// start overlapping captures.
pub struct CaptureCoordinator {
    mode: HotkeyMode,
    cfg: CaptureConfig,
    state: CaptureState,
}

impl CaptureCoordinator {
    pub fn new(mode: HotkeyMode, cfg: CaptureConfig) -> Self {
        Self {
            mode,
            cfg,
            state: CaptureState::Idle,
        }
    }

    pub fn state(&self) -> CaptureState {
        self.state
    }

    /// Abort the current session after a runtime start failure. Normal stop
    /// paths should still flow through [`Signal`] so timing invariants apply.
    pub fn reset(&mut self) {
        self.state = CaptureState::Idle;
    }

    /// Process one signal; returns the runtime action and updates state.
    pub fn step(&mut self, sig: Signal) -> Action {
        use CaptureState::*;
        use HotkeyMode::*;
        use Signal::*;
        match (self.mode, self.state, sig) {
            // Start from idle.
            (_, Idle, Press { at_ms }) => {
                self.state = Capturing {
                    started_ms: at_ms,
                    last_press_ms: at_ms,
                };
                Action::StartCapture
            }
            // Key-bounce / auto-repeat while capturing: ignore, but coalesce the
            // press time so a genuine later press is measured from here.
            (
                _,
                Capturing {
                    started_ms,
                    last_press_ms,
                },
                Press { at_ms },
            ) if at_ms.saturating_sub(last_press_ms) < self.cfg.debounce_ms => {
                self.state = Capturing {
                    started_ms,
                    last_press_ms: at_ms,
                };
                Action::None
            }
            // Toggle: a real second press (past debounce) stops.
            (Toggle, Capturing { started_ms, .. }, Press { at_ms }) => {
                self.begin_finalize(started_ms, at_ms)
            }
            // Push-to-talk: extra presses while held are ignored.
            (PushToTalk, Capturing { .. }, Press { .. }) => Action::None,
            // Push-to-talk release: min-capture check, then finalize with tail.
            (PushToTalk, Capturing { started_ms, .. }, Release { at_ms }) => {
                self.begin_finalize(started_ms, at_ms)
            }
            // Toggle ignores releases entirely.
            (Toggle, _, Release { .. }) => Action::None,
            // Tail window elapsed → finalize.
            (_, Finalizing { ends_ms, .. }, Tick { at_ms }) if at_ms >= ends_ms => {
                self.state = Idle;
                Action::FinalizeCapture
            }
            // Any other edge during finalizing (incl. an early re-press) is held
            // off until this session closes — keeps sessions from overlapping.
            (_, Finalizing { .. }, _) => Action::None,
            // Idle release / stray ticks.
            (_, _, _) => Action::None,
        }
    }

    /// Enter the tail-buffer window, or discard if the capture was too short.
    fn begin_finalize(&mut self, started_ms: u64, at_ms: u64) -> Action {
        if at_ms.saturating_sub(started_ms) < self.cfg.min_capture_ms {
            self.state = CaptureState::Idle;
            Action::DiscardCapture
        } else {
            self.state = CaptureState::Finalizing {
                started_ms,
                ends_ms: at_ms + self.cfg.tail_buffer_ms,
            };
            Action::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ptt() -> CaptureCoordinator {
        CaptureCoordinator::new(HotkeyMode::PushToTalk, CaptureConfig::default())
    }
    fn toggle() -> CaptureCoordinator {
        CaptureCoordinator::new(HotkeyMode::Toggle, CaptureConfig::default())
    }

    #[test]
    fn ptt_starts_on_press_and_finalizes_after_tail() {
        let mut c = ptt();
        assert_eq!(c.step(Signal::Press { at_ms: 0 }), Action::StartCapture);
        assert_eq!(c.step(Signal::Release { at_ms: 400 }), Action::None); // >= min → tail
        assert!(matches!(
            c.state(),
            CaptureState::Finalizing { ends_ms: 700, .. }
        ));
        assert_eq!(c.step(Signal::Tick { at_ms: 699 }), Action::None); // tail not up
        assert_eq!(c.step(Signal::Tick { at_ms: 700 }), Action::FinalizeCapture);
        assert_eq!(c.state(), CaptureState::Idle);
    }

    #[test]
    fn debounce_prevents_double_start() {
        // The Pitfall-P1 race: a bounced/repeated press must not start a 2nd capture.
        let mut c = ptt();
        assert_eq!(c.step(Signal::Press { at_ms: 0 }), Action::StartCapture);
        assert_eq!(c.step(Signal::Press { at_ms: 10 }), Action::None); // within 30ms → ignored
        assert!(matches!(
            c.state(),
            CaptureState::Capturing { started_ms: 0, .. }
        ));
    }

    #[test]
    fn short_tap_is_discarded() {
        let mut c = ptt();
        c.step(Signal::Press { at_ms: 0 });
        // Held only 100ms (< 250 floor) → accidental tap, dropped.
        assert_eq!(
            c.step(Signal::Release { at_ms: 100 }),
            Action::DiscardCapture
        );
        assert_eq!(c.state(), CaptureState::Idle);
    }

    #[test]
    fn tail_buffer_keeps_capturing_after_release() {
        let mut c = ptt();
        c.step(Signal::Press { at_ms: 0 });
        c.step(Signal::Release { at_ms: 300 }); // finalizing until 600
        assert!(matches!(
            c.state(),
            CaptureState::Finalizing { ends_ms: 600, .. }
        ));
        // still capturing through the tail — no finalize before the window closes
        assert_eq!(c.step(Signal::Tick { at_ms: 599 }), Action::None);
        assert_eq!(c.step(Signal::Tick { at_ms: 600 }), Action::FinalizeCapture);
    }

    #[test]
    fn toggle_starts_and_stops_on_presses_ignoring_release() {
        let mut c = toggle();
        assert_eq!(c.step(Signal::Press { at_ms: 0 }), Action::StartCapture);
        assert_eq!(c.step(Signal::Release { at_ms: 50 }), Action::None); // release ignored
        assert!(matches!(c.state(), CaptureState::Capturing { .. }));
        // second real press (past debounce, past min) → tail then finalize
        assert_eq!(c.step(Signal::Press { at_ms: 500 }), Action::None);
        assert_eq!(c.step(Signal::Tick { at_ms: 800 }), Action::FinalizeCapture);
    }

    #[test]
    fn early_repress_during_finalize_does_not_overlap() {
        let mut c = ptt();
        c.step(Signal::Press { at_ms: 0 });
        c.step(Signal::Release { at_ms: 300 }); // finalizing until 600
                                                // a fresh press at 350 must not start a second capture mid-finalize
        assert_eq!(c.step(Signal::Press { at_ms: 350 }), Action::None);
        assert!(matches!(c.state(), CaptureState::Finalizing { .. }));
    }

    #[test]
    fn runtime_can_reset_after_start_failure() {
        let mut c = ptt();
        assert_eq!(c.step(Signal::Press { at_ms: 0 }), Action::StartCapture);

        c.reset();

        assert_eq!(c.state(), CaptureState::Idle);
        assert_eq!(c.step(Signal::Press { at_ms: 500 }), Action::StartCapture);
    }
}
