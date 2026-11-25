# Changelog

All notable changes to MouseMacro are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
semantic versioning.

## [1.4.0] — 2026-11-25

Keyboard remapping, standalone wheel triggers and a rebuilt recorder.

### Added

- **Keyboard remapping (opt-in)** — turn a key or key combination into another
  key, or run any MouseMacro action. *Remap* keeps hold semantics (Caps Lock →
  Ctrl), while *Run an action* fires once on key down. Matching is exact: a
  plain `F12` mapping does not fire while Ctrl is held. Auto-repeat and the
  matching key-up are handled, so held remaps release cleanly.
- **Keyboard page** — profile selector, mapping list with search, duplicate and
  shadow warnings, a dedicated *Keyboard* toggle with the same opt-in
  acknowledgement as the other listening features, and per-app keyboard rules
  in Exceptions.
- **Standalone wheel triggers** — map the wheel itself to an action in all four
  directions, including horizontal tilt left/right, without a button. Wheel
  mappings have their own group in Button mappings and no pseudo-button.
- **Wheel precedence and options** — button + wheel gesture → layer wheel
  override → standalone wheel → pass through. An 80 ms cooldown per mapping
  keeps a fast wheel from firing repeatedly; *also scroll* is off by default.
- **Recording rebuilt around sessions** — Record/Stop with a live, scrollable
  gesture log. Down/up pairs collapse into one row (“Left click”, “Wheel
  left”), and a keyboard combination is captured as one entry (“Ctrl + Shift +
  S”). Any row can be clicked to use that gesture instead; repeats collapse.
  The log stays visible after Stop, and a session ends by itself after 20 s.
- **Input from other tools while recording** — vendor utilities, remote
  sessions and on-screen keyboards deliver input as injected events; recording
  now accepts it. MouseMacro's own tagged output is still ignored.
- **Recording safety nets** — the keyboard hook is reinstalled for every
  recording, the active session is polled so a missed end event cannot leave a
  stale “Listening” state, and the focused window records the key if the hook
  misses it.
- **Horizontal wheel diagnostics** — Settings now hints that `left`/`right`
  wheel rows mean a thumb/tilt wheel.

### Changed

- Configuration schema is now **v4**. Existing v3 files migrate automatically;
  keyboard settings, keyboard mappings and wheel mappings receive defaults.
- A mapping's `button` is optional, so a wheel mapping carries no pseudo-button.
- The keyboard hook is installed at startup only while keyboard remapping is
  enabled and acknowledged; recording installs it on demand.
- The mapping editor and keyboard editor share one action editor, so every
  action type is available from both.
- Emergency stop always wins over keyboard remapping, layers and wheel triggers.

### Fixed

- Keyboard combinations were recorded without their modifiers when the hook
  swallowed the modifier keys. Held modifiers now come from the captured
  stream instead of the OS key state.
- Starting a new recording replayed the previous session's last gesture into
  the field.
- Events delivered just after Stop updated the log but not the mapping field.
- Space or Enter pressed right after Record could activate Stop before the
  session was listening.
- A keyboard hook that Windows had silently dropped could leave recording
  listening forever; recording now installs a fresh hook each time.

### Tests

- 116 Rust tests, including the keyboard matcher, wheel precedence and cooldown,
  capture sessions and suppression, schema v4 migration, and the existing
  engine, layer, exception, macro and scroll coverage.

## [1.3.0] — 2026-11-25

Macros, richer actions and real scroll control.

### Added

- **Macros** — ordered action sequences with tap/hold/release keys, waits, mouse
  clicks and holds, scrolling and typed text. Macros support a repeat count with
  an interval, or toggle mode where pressing the same button again stops the run.
  Steps can be reordered, duplicated, delayed, and tested from the editor.
- **New action types** — launch a program, file, folder or URL; type Unicode text
  into the focused window; media keys; minimize/maximize/restore/close and
  snap-left/right window actions; sticky toggle keys; and a drag lock.
- **Scroll tab** — invert vertical or horizontal scrolling, swap axes for drivers
  that report the wrong one, send 1–5 notches per physical notch, and optional
  time-based acceleration. Per-application overrides keep a game on normal
  scrolling while the desktop stays inverted.
- **Import safety** — profiles containing launch actions show every target for
  confirmation before they are saved.

### Changed

- Configuration schema is now **v3**. Existing v2 files migrate automatically;
  wheel handling, macros and the new action types receive defaults.
- Macros run on their own thread, are cancelled by the emergency stop, the editor
  Stop button, a toggle press or an app switch, and release every key and button
  they hold.
- Physical wheel events are only transformed when the setting differs from
  pass-through; MouseMacro's own injected scroll is never remapped.

### Tests

- 87 Rust tests, including scroll transforms, per-app scroll precedence, macro
  step sanitation and the existing engine/layer/exception coverage.

## [1.2.0] — 2026-11-25

Mappings become context-aware, repeatable and safer to leave running.

### Added

- **Application exceptions** — per-executable rules that disable mappings or
  switch to another profile while that app is in the foreground. The foreground
  watcher runs four times per second, and releasing a hold on app switch
  prevents stuck keys.
- **Layers** — hold a button to activate a second mapping set for the other
  buttons and wheel. Unlisted buttons still fall back to the profile, and the
  activator keeps its own tap/long-press behaviour.
- **Emergency stop** — global hotkey (default `Ctrl + Alt + Shift + F12`) and a
  tray item release every synthetic key/button and pause mappings without
  changing saved settings. Resume from the tray or the Safety card.
- **Mapping workflow** — search, duplicate, enable/disable all, shadow warnings
  for duplicate button+trigger pairs, per-mapping last-fired times, and a Test
  button that runs an action once from the editor.
- **Backup and restore** — export/import all settings, profiles, layers and
  exceptions as one versioned JSON file. Ten rotating timestamped backups are
  kept, with one-click restore that backs up the current state first.
- **First-run setup** — guided three-step wizard: overview, button detection,
  then profile and startup choices.
- **Application picker** — add a rule for any visible window, or type an
  executable name.

### Changed

- Configuration schema is now **v2**. Existing files migrate automatically with
  a backup; the first-run wizard is suppressed for existing installs.
- The tray can stop all input, and the sidebar shows the effective profile and
  the app that selected it.
- Imported full configurations never change Windows startup integration; the
  registry stays the source of truth.

### Tests

- 77 Rust tests, including layer resolution/fallback, exception precedence,
  panic precedence, backup restoration and hotkey combo parsing.

## [1.1.0] — 2026-11-25

Adds in-app input diagnostics, support for vendor-software input, and reloading
hand-edited configuration.

### Added

- **Input diagnostics** in Settings: start and stop a recording session, then
  inspect the last mouse events with device axis, wheel delta, and whether an
  event was filtered as synthetic or matched a rule. Starting clears the buffer;
  stopping freezes the snapshot for inspection.
- **Process input from other input tools** setting for events synthesized by
  mouse vendor software, touchpad drivers or remote desktop. MouseMacro's own
  synthetic input is always ignored through `INJECTION_TAG`, so this cannot
  cause feedback loops.
- **Reload from disk** action for `config.json`, picking up hand edits without a
  restart and normalising the file.

### Changed

- The keyboard-hook diagnostics are now part of the product instead of a hidden
  development command surface.
- Documentation: diagnostics workflow, horizontal-wheel behaviour, and
  troubleshooting entries.

### Tests

- 50 Rust tests (3 new for the injected-input decision logic).

## [1.0.0] — 2026-11-25

First public release.

### Added

- Mouse button remapping for Left, Right, Middle, B4 (X1) and B5 (X2).
- Triggers: single click, double click, long press, release, button + button
  chords, and button + wheel gestures on both axes.
- Actions: keyboard shortcuts, single keys, held keys, mouse click/hold, scroll
  by notches, disable, and explicit "no action".
- Pure Rust gesture engine with fixed gesture/long-press precedence, configurable
  double-click/release/single-click order, click replay for unmatched input, and
  guaranteed release of synthetic holds.
- Searchable key picker with modifier chips and the full Windows key catalog —
  works without the global keyboard hook. Global recording remains available as
  an optional fallback.
- Profiles with seeded examples (Default, Productivity, Gaming, Design), plus
  per-profile JSON import/export.
- System tray menu: open, enable/disable, switch profile, exit.
- Launch on Windows startup, start minimized, close to tray, light/dark/system
  themes.
- Optional relaunch as administrator for elevated applications.
- GUI-subsystem executables: no console window in debug or release builds.
  Attach logs with `--console` or `MOUSEMACRO_CONSOLE=1`.
- Installer, MSI and portable builds; atomic config writes; corrupt-file backup;
  registry/autostart reconciliation.

### Known limitations

- Windows 10 (1809+)/11 x64 only.
- Hardware DPI control is not claimed: no manufacturer-specific integration.
- Scroll, Mouse, Keyboard and Exceptions tabs from the extended roadmap are not
  part of this release.
