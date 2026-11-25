# MouseMacro

**Input studio for Windows** — remap mouse buttons, record shortcuts, and turn
button holds into gestures with a native input engine.

Built with Tauri v2, React 19, TypeScript, Tailwind CSS 4 and Rust. Everything
runs locally: no telemetry, no network calls, no background services.

> **Platform:** Windows 10 (1809+) / Windows 11, x64. This is a desktop app, not
> a web app — the UI cannot run in a browser because it depends on native input
> hooks.

---

## What works today

### Button remapping
- Physical buttons: Left, Right, Middle, Back (B4 / X1) and Forward (B5 / X2).
- Auto-detection of the physical button, or manual selection.
- Enable, edit and delete individual mappings; mappings apply instantly.

| Trigger | Fires when |
| --- | --- |
| Single click | Press and release |
| Double click | Two clicks inside the double-click window |
| Long press | Held past the long-press threshold |
| On release | Any release of the button |
| Button + button | First button held, second pressed (chord) |
| Button + wheel | Held while scrolling up / down / left / right |

### Actions
- Keyboard shortcut (e.g. `Ctrl + Z`)
- Single key
- Hold keys until the source button is released
- Mouse click / mouse button hold
- Scroll by a number of notches
- Disable the button (consume it silently) or explicit "no action"

### Choosing keys
Two independent paths, both usable offline:

1. **Choose** — a searchable key catalog (`page`, `f5`, `numpad`, `volume`, …)
   with modifier chips. Pure UI, no global hook involved.
2. **Record** — captures the next physical shortcut through a low-level keyboard
   hook that is installed only while recording, then removed from the input path.

If a game or secure window blocks the hook, path 1 always works and the app
explains the failure instead of silently stopping.

### Gesture engine guarantees
The engine is a pure state machine (`src-tauri/src/engine/`), tested without
touching Windows APIs:

- Gestures (button + button, button + wheel) always win over taps.
- Long press wins over double click and single click.
- A tap is suppressed once a gesture or long press has fired — no double action.
- Unmatched input is replayed to the OS instead of being swallowed.
- Synthetic holds are released on unmount, disable, reset and app exit.
- Injected events are tagged and ignored, so mappings cannot recurse.
- When the engine is disabled, every event passes straight through.

### Profiles
- Create, rename, duplicate and delete profiles.
- Seeded examples: Default, Productivity, Gaming, Design.
- Import/export a profile as JSON.
- Switch profiles from the sidebar or the tray menu.

### Desktop behaviour
- System tray icon: open, enable/disable, switch profile, exit.
- Launch on Windows startup (registry reconciled with config at boot).
- Start minimized, close-to-tray, light/dark/system themes.
- Optional relaunch as administrator for elevated applications.
- **Input diagnostics** in Settings: press Start, use the mouse, press Stop — the
  last mouse events are listed with axis, delta and whether they were filtered
  as synthetic. Also an opt-in to process input synthesized by other utilities
  (mouse vendor software, touchpad drivers, remote desktop).

### Safety
- Atomic config writes with a timestamped backup of unreadable files.
- Hook failures are surfaced in the UI, never silently ignored.
- Corrupt configuration cannot brick startup; defaults are restored.

---

## Requirements

| Tool | Version |
| --- | --- |
| Node.js | 20+ |
| Rust | 1.77.2+ (`rustup`) |
| Visual Studio Build Tools | 2019 or 2022, "Desktop development with C++" + Windows 10/11 SDK |
| WebView2 Runtime | Preinstalled on Windows 11 and updated Windows 10 |

## Run in development

```powershell
npm install
npm run tauri dev
```

On a plain Windows shell, `cargo` must be on `PATH` and the MSVC linker must be
available. Either use a *Developer PowerShell for VS*, or:

```powershell
cmd /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat" && set "PATH=%USERPROFILE%\.cargo\bin;%PATH%" && npx tauri dev'
```

Vite hot-reloads the frontend; Rust changes trigger a rebuild.

## Build an installable app

```powershell
npm run tauri build
```

Outputs:

```
src-tauri/target/release/mousemacro.exe                  portable, no install
src-tauri/target/release/bundle/nsis/*-setup.exe          per-user installer
src-tauri/target/release/bundle/msi/*.msi                 MSI package
```

For a fast local build without installers:

```powershell
npx tauri build --debug --no-bundle
```

Both debug and release binaries are GUI-subsystem executables — no console
window. Pass `--console` (or set `MOUSEMACRO_CONSOLE=1`) when launching from a
terminal to attach logs.

## Tests

```powershell
npm run test:rust   # 50 engine/config/hook tests
npm run build       # TypeScript typecheck + production frontend build
```

---

## How it works

```
low-level mouse hook ──► EngineService ──► pure engine ──► effects
                              │                              │
                              └──── capture events ──► UI    └─► SendInput
```

- `platform/hooks.rs` — `WH_MOUSE_LL` on a dedicated message-loop thread; the
  keyboard hook is installed lazily on its own thread only for recording.
- `platform/service.rs` — adapts raw events to engine input, runs the timer for
  long press / double click deadlines, and executes effects.
- `engine/` — UI-independent rules, state and timing. Deterministic and fully
  unit-tested.
- `platform/simulate.rs` — `SendInput` shortcuts, clicks, holds and wheel
  scrolling, tagged with `INJECTION_TAG` so the hook ignores them.
- `state.rs` — configuration, validation, snapshots and window control.
- `src/` — React UI: Dashboard, Button mappings (sheet editor), Profiles,
  Settings, About.

## Configuration

Stored as JSON at `%APPDATA%\com.mousemacro.app\config.json` and written
atomically. Shape:

```json
{
  "schemaVersion": 1,
  "settings": {
    "enabled": true,
    "launchAtStartup": false,
    "startMinimized": false,
    "closeToTray": true,
    "theme": "system",
    "longPressMs": 400,
    "doubleClickMs": 280,
    "rulePriority": ["gesture", "longPress", "doubleClick", "release", "singleClick"]
  },
  "profiles": [
    {
      "id": "…",
      "name": "Default",
      "description": "Everyday browsing and desktop use",
      "mappings": [
        {
          "id": "…",
          "name": "Undo",
          "button": "x2",
          "trigger": { "type": "singleClick" },
          "action": { "type": "shortcut", "keys": ["ctrl", "z"] },
          "enabled": true
        }
      ]
    }
  ],
  "activeProfileId": "…"
}
```

Gesture and long-press precedence are fixed; the order of double click, release
and single click is configurable in Settings.

## Project layout

```
src/                     React UI
  components/            Sheet editor, key picker, reusable controls
  hooks/useCapture.ts    backend capture bridge
  lib/                   types, formatting, key catalog, API client
  pages/                 Dashboard, Mappings, Profiles, Settings, About
  state/                 app context (snapshot, view, run helpers)
src-tauri/src/
  config/                model, defaults, atomic store, import/export
  engine/                pure rule engine + key tables (47 tests)
  platform/              hooks, service, SendInput, elevation
  commands.rs            Tauri command surface
  tray.rs                tray menu
  lib.rs                 setup, plugins, window sizing, shutdown cleanup
```

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| No window after launch | It starts in the tray (`startMinimized`). Use the tray icon, or run the exe again to show the existing instance. Disable *Start minimized* in Settings. |
| Closing the window doesn't quit | *Close to tray* is on. Use tray → Exit. |
| Shortcut recording sees nothing | Some games/secure windows block keyboard hooks. Use **Choose** in the action section — no hook required. |
| Scroll gestures do nothing | Settings → **Input diagnostics**: if wheel rows say `synthetic`, enable *Process input from other input tools*. If they say `horizontal`, the device is a thumb/tilt wheel — map `left`/`right` triggers. |
| Hand-edited config.json has no effect | Settings → **Reload from disk** (or restart). The running app keeps its in-memory copy. |
| Shortcuts don't work inside a game | If the game runs elevated, relaunch MouseMacro via *Settings → Run as administrator*. |
| SmartScreen on another PC | The build is unsigned: *More info → Run anyway*. |

## Roadmap

The next milestone expands the app from button remapping into a full input
customisation suite with six tabs:

1. **Scroll** — system scroll lines, reverse/smooth scrolling, app-specific overrides.
2. **Mouse** — Windows pointer speed, acceleration, primary button, double-click speed, sensitivity profiles.
3. **Buttons** — the mapping engine above plus macros.
4. **Keyboard** — key remapping, repeat delay/rate, key disabling, per-app rules.
5. **Exceptions** — allow/block lists per executable, per-app profiles.
6. **Settings** — schema-driven controls, full config import/export, update info.

OS-level changes will snapshot and restore original Windows values, with explicit
warnings and no silent modification.
