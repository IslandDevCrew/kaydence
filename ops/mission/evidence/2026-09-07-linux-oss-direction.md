# Kaydence: Linux OSS direction, not an architecture replacement

Research snapshot: 2026-09-07. Primary-source reading only; no fetched code executed, dependencies installed, repository files changed, or runtime proof claimed. Kaydence source snapshot: `4b6bc462394108ccec7a4752c92a2770d74536b8`; mission state was read first. Its P1-G3 and reopened P1-G4 remain pending; this note closes neither. [K-state]

## Identity and provenance

| Reference | Exact inspected revision | What it contributes |
|---|---|---|
| `peteonrails/voxtype` | `e32510690636d1e082f2f02a5094efda4aa95fa0` (default branch `dev`, commit dated 2026-09-02) | The local-first Voxtype associated with voxtype.io; its README documents Linux compositor integration. MIT, copyright Peter Jackson. This is the **inferred intended reference**, because the user supplied a name, not a URL. [V-commit] [V-readme] [V-license] |
| `cjpais/Handy` | `bc7facea3a777869182203cfcf5c90f7a98efd99` (commit dated 2026-09-05) | Linux-capable Tauri comparator already selected for study—not a hard fork—by Kaydence ADR-0004. MIT, copyright CJ Pais. [H-commit] [H-readme] [H-license] [K-ADR4] |
| `atheerium/voxtype` | `c81000210fd4f17a2ca0849f6c92500df403e443` | A different MIT project with the same name. Its README describes ffmpeg capture and Groq-hosted transcription by default. Do not mistake its privacy positioning for Kaydence's offline default. Included only to resolve the naming collision, not as the implementation model. [A-readme] [A-license] |

**Recommendation / inference:** study the first two projects' failure handling and test cases; keep Kaydence's pipeline and approved product contracts. This extends ADR-0004 rather than reopening it. Source-code MIT notices are not evidence of model-weight licenses or binary provenance; review those artifacts separately before any adoption. Kaydence already requires ADR/operator review for new dependencies, OS surfaces, network surfaces, and model-registry changes. [V-license] [H-license] [K-ADR4] [K-agents]

## What the primary sources actually establish

| Area | Source fact | Kaydence transfer / boundary (inference) |
|---|---|---|
| Hotkeys | Voxtype documents compositor start/stop bindings, with toggle where key release is unavailable. Its built-in Linux listener uses evdev and requires input-device access; it handles hotplug and excludes injection-created virtual keyboards. [V-readme] [V-hotkey] | Separate registration, actual press/release proof, and recording readiness. Reconnect and synthetic-event feedback tests are useful. Do not silently add the user to `input`, install a privileged daemon, or replace approved hotkey plumbing. |
| Capture | Voxtype runs CPAL on a dedicated thread, uses channels, preserves a stateful resampler across callbacks, and flushes its tail on Stop. Stream failure is recorded explicitly, but the stop path can still return samples captured before failure. [V-capture] | Test dropped-device and final-syllable behavior; do not assume “nonempty audio” means a complete recording. Kaydence already has a lock-free callback/drain/WAL architecture; preserve that and its durable-audio ordering instead of copying Voxtype's callback locking/buffering implementation. [K-audio] |
| Wayland output | The Voxtype output chain orders wtype, eitype, dotool, ydotool, and clipboard drivers. Its eitype adapter targets EI-capable compositors; Handy explicitly excludes wtype on GNOME/KDE in its automatic Linux paste path. Voxtype's wtype availability probe checks executable presence, not compositor capability. [V-output] [V-eitype] [V-wtype] [H-clipboard] | Use a measured compositor/capability matrix, not “Linux = supported.” Installed command, portal permission, accessible target, and actual delivery are distinct states. Native macOS/Windows paths remain independent behind the shared contract. |
| Modifier races | Voxtype waits for modifier release and skips keystroke methods on timeout. However, unreadable `/dev/input` disables its guard and returns success immediately. [V-output] [V-modifiers] | A bounded guard and an explained hold are useful; disabled observation must stay **unknown**, not become “safe.” Any change to Kaydence's accepted Unknown-field policy needs the existing critical-path process, not a silent safety-policy rewrite. [K-ADR13] |
| Delivery truth | Voxtype `TextOutput` returns `Result<()>`; its reviewed wtype/eitype adapters treat successful command exit as success. The reviewed interface does not carry a secure-field verdict or capture-bound target identity. [V-output] [V-wtype] [V-eitype] | **[UNVERIFIED]** End-to-end secure-field protection across Voxtype as a whole was not established by this bounded review. Neither exit zero nor clipboard copy proves focused-field delivery. Keep Kaydence's Held/Error/Injected semantics and revalidate its focus/security checks. [K-core] |
| Linux safety already learned | Kaydence ADR-0013 amendments record AT-SPI insertion acknowledging without changing GNOME fields, while AT-SPI secure detection and explicit uinput insertion were proven on an accessible client. The July proof explicitly leaves opaque clients, Unicode beyond ASCII, and portal/libei work open. [K-ADR13] | This is not an invitation to discard the existing ladder. Preserve disclosed Lenient/Strict behavior and the real distinction between detector and insertion mechanism. Unknown must never be labelled verified in Privacy or Cleanup. |
| Clipboard and Raw | Handy restores the prior text/image or clears an originally empty clipboard even after paste-key failure. Voxtype normalizes curly quotes before its output chain, and clipboard fallback counts as output success. [H-clipboard] [V-output] | Borrow restoration-on-error test ideas, not the text-normalization policy. Kaydence Raw and clipboard-restoration contracts are stronger and must remain intact; the current snapshot/restore abstraction is text-only. [K-core] [K-agents] |
| Tauri-specific hazards | Handy documents Linux phantom recordings/interrupted dictations from using SIGUSR1, which WebKitGTK also uses; its current guidance removes that binding. It also documents X11 overlay focus interfering with paste. Voxtype's README still describes SIGUSR1/SIGUSR2 record control. [H-readme] [V-readme] | Do **not** transplant Voxtype's signal transport into Tauri. Add a lifecycle/focus regression scenario around visible, closed, and reopened cockpit states; do not infer native WebView safety from Chromium screenshots. |
| Hermetic capability tests | Voxtype's session detector accepts an injected environment lookup for tests, avoiding process-global environment mutation. Its production fallback assumes Wayland when no relevant variables exist. [V-session] | Adopt the hermetic-test pattern, not the unknown-session assumption. Kaydence should report unsupported/unverified capability truth instead of presenting a guessed green badge. |

## Three bounded experiments — proposed, not executed

1. **Capability truth matrix — P1-G4 / P1-P0-3 / P1-P0-8.** First use existing snapshot fixtures to verify that Cleanup, Privacy, and Setup distinguish registered hotkey, observed input, microphone proof, active injection method, unknown field, and real delivery. Then run the same small non-sensitive Unicode sentence plus an empty password-field refusal case on each available authorized native lane. For Linux record compositor/session, accessibility visibility, selected rung, permission outcome, and actual focused-field result; missing GNOME/KDE/wlroots/X11 cells remain missing, not extrapolated. Acceptance: no green readiness from executable presence, clipboard copy, fixture data, or unknown security state. Map to existing backend facts; any missing contract is a separately owned backend packet. [K-state] [K-ADR13] [V-wtype] [V-eitype]
2. **Capture-to-UI liveness — P1-G1/G2/G4/G5.** Reuse Kaydence's short-utterance/WAL/event tests and add a bounded proof sequence: start, rapid repeat, release, disconnect/reconnect microphone, and reopen cockpit during a session. Check that displayed recording/elapsed/error states reflect backend events and that the final 0.3-second clip is not truncated. Acceptance: no overlapping session, explicit partial/failure state, persisted recoverable audio, one terminal outcome, and truthful UI after reopening. Voxtype's resampler-tail/device-failure handling and Handy's coordinator/focus history motivate the cases; they do not prove Kaydence's outcomes. [K-state] [K-audio] [V-capture] [H-shortcut] [H-readme]
3. **One exact-build physical proof packet per owned reference device — P1-G3 / P1-P0-8.** Measure hotkey-release through actual focused-field text, packaged model-resident memory, and visible/closed/reopened idle footprint; separately time a genuinely fresh first-run journey. Save SHA, flags, model hash, OS/compositor, PID/WebView roster, raw timestamps/results, and failures. Keep the existing ≤700 ms GPU / ≤1200 ms CPU, <80 MB no-model, ≤250 MB resident, and 60-second setup criteria unchanged; only current measured evidence can close them. Neither Voxtype's README speed claims nor hosted CI substitutes for this packet. Permission/device interaction requires the mission's single-owner lane; do not manipulate another agent's running instance. [K-state] [K-agents]

## Decision and limits

**Recommendation:** finish truthful P1 surfaces and real native integration before expanding the model roster or adopting another project's daemon. The useful reuse is primarily failure taxonomy, capability probing, and regression cases; Kaydence already has the right abstraction boundaries for cross-platform differences. This is an architectural inference from the cited code and ADRs, not a guarantee of implementation effort or completion date. [K-core] [K-ADR4] [K-ADR13]

No new crate, binary, privileged permission, model, shell-hook surface, or copied implementation is proposed for immediate adoption. Preserve source pins and MIT notices if any future snippet is approved; Kaydence requires `ATTRIBUTIONS.md` alongside vendored material. ADR-0016 provenance/default-lane approval and the separate ADR-0017 shipped-download gate remain operator decisions; the approval of CI/auto-merge does not settle them. [K-ADR4] [K-agents] [K-state]

**[UNVERIFIED]** Runtime behavior, security completeness, performance, and packaged compatibility of all three external projects were not tested. All experiments above remain proposals. Repository source was read at pinned revisions; release-artifact correspondence, dependency licenses, and model licenses were not audited.

## Primary-source URLs

[V-commit]: https://github.com/peteonrails/voxtype/commit/e32510690636d1e082f2f02a5094efda4aa95fa0
[V-readme]: https://github.com/peteonrails/voxtype/blob/e32510690636d1e082f2f02a5094efda4aa95fa0/README.md
[V-license]: https://github.com/peteonrails/voxtype/blob/e32510690636d1e082f2f02a5094efda4aa95fa0/LICENSE
[V-hotkey]: https://github.com/peteonrails/voxtype/blob/e32510690636d1e082f2f02a5094efda4aa95fa0/src/hotkey/evdev_listener.rs
[V-capture]: https://github.com/peteonrails/voxtype/blob/e32510690636d1e082f2f02a5094efda4aa95fa0/src/audio/cpal_capture.rs
[V-output]: https://github.com/peteonrails/voxtype/blob/e32510690636d1e082f2f02a5094efda4aa95fa0/src/output/mod.rs
[V-eitype]: https://github.com/peteonrails/voxtype/blob/e32510690636d1e082f2f02a5094efda4aa95fa0/src/output/eitype.rs
[V-wtype]: https://github.com/peteonrails/voxtype/blob/e32510690636d1e082f2f02a5094efda4aa95fa0/src/output/wtype.rs
[V-modifiers]: https://github.com/peteonrails/voxtype/blob/e32510690636d1e082f2f02a5094efda4aa95fa0/src/output/modifier_guard.rs
[V-session]: https://github.com/peteonrails/voxtype/blob/e32510690636d1e082f2f02a5094efda4aa95fa0/src/output/session.rs
[H-commit]: https://github.com/cjpais/Handy/commit/bc7facea3a777869182203cfcf5c90f7a98efd99
[H-readme]: https://github.com/cjpais/Handy/blob/bc7facea3a777869182203cfcf5c90f7a98efd99/README.md
[H-license]: https://github.com/cjpais/Handy/blob/bc7facea3a777869182203cfcf5c90f7a98efd99/LICENSE
[H-clipboard]: https://github.com/cjpais/Handy/blob/bc7facea3a777869182203cfcf5c90f7a98efd99/src-tauri/src/clipboard.rs
[H-shortcut]: https://github.com/cjpais/Handy/blob/bc7facea3a777869182203cfcf5c90f7a98efd99/src-tauri/src/shortcut/handler.rs
[A-readme]: https://github.com/atheerium/voxtype/blob/c81000210fd4f17a2ca0849f6c92500df403e443/README.md
[A-license]: https://github.com/atheerium/voxtype/blob/c81000210fd4f17a2ca0849f6c92500df403e443/LICENSE
[K-state]: https://github.com/IslandDevCrew/kaydence/blob/4b6bc462394108ccec7a4752c92a2770d74536b8/ops/mission/state.json
[K-core]: https://github.com/IslandDevCrew/kaydence/blob/4b6bc462394108ccec7a4752c92a2770d74536b8/apps/desktop/src-tauri/src/inject/mod.rs
[K-audio]: https://github.com/IslandDevCrew/kaydence/blob/4b6bc462394108ccec7a4752c92a2770d74536b8/apps/desktop/src-tauri/src/audio/mod.rs
[K-ADR4]: https://github.com/IslandDevCrew/kaydence/blob/4b6bc462394108ccec7a4752c92a2770d74536b8/docs/decisions/0004-study-handy-not-fork.md
[K-ADR13]: https://github.com/IslandDevCrew/kaydence/blob/4b6bc462394108ccec7a4752c92a2770d74536b8/docs/decisions/0013-linux-injection-strategy.md
[K-agents]: https://github.com/IslandDevCrew/kaydence/blob/4b6bc462394108ccec7a4752c92a2770d74536b8/AGENTS.md
