# Changelog

All notable changes to MouseMacro are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
semantic versioning.

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
