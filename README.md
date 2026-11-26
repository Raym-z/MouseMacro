# MouseMacro

**Input studio for Windows** — remap mouse buttons and keys, record shortcuts,
and turn button holds into gestures with a native input engine.

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
| Wheel | Scroll up / down / left / right without a button |

### Layers
- Hold a button to activate a second mapping set for the other buttons (and
  wheel directions defined in the layer).
- Buttons the layer does not define keep their normal profile mapping.
- The activator keeps its own tap, double click and long press from the profile.
- One layer per activator, up to 20 layers per profile.

### Application rules (Exceptions)
- Per-executable behaviour: pass input through untouched, or switch to another
  profile while that app is in the foreground.
- Add from the list of open windows or type an executable name.
- The foreground watcher also releases synthetic holds when you switch apps, so
  nothing sticks in the window you left.
- Rules match the executable name, not a full path.

### Actions
- Keyboard shortcut (e.g. `Ctrl + Z`)
- Single key
- Hold keys until the source button is released
- Mouse click / mouse button hold
- Scroll by a number of notches
- **Macro** — ordered steps with tap/hold/release keys, waits, mouse clicks and
  holds, scrolling and typed text; repeat with an interval or toggle to stop
- **Launch** a program, file, folder or URL
- **Type text** (Unicode-safe, including non-Latin characters)
- Media keys (play/pause, next, previous, volume, mute)
- Window actions (minimize, maximize, restore, close, snap left/right)
- Sticky **toggle keys** and **drag lock**
- **Multiple actions per mapping** — add 2–8 actions to one trigger; they run
  in order when it fires, and hold actions stay held until you release. For
  example, Middle button → *Hold keys* `H` + *Hold mouse button* Left pans in
  Photoshop with the hand tool.
- Disable the button (consume it silently) or explicit "no action"

### Keyboard
- **Opt-in** — installs an always-on low-level keyboard hook only while it is
  enabled and acknowledged; off by default.
- **Remap** a key or combination to another key with hold semantics (Caps Lock →
  Ctrl, hold-to-walk), or **run any action** from it.
- Matching is **exact**: a plain `F12` mapping does not fire while Ctrl is held,
  and `Ctrl + F12` is a separate mapping.
- Auto-repeat does not retrigger a mapping; the matching key-up releases a held
  remap.
- **Per-app rules** can disable keyboard remapping for games and secure apps.
- **Privacy** — no text is stored and no keystroke history is kept; unmatched
  keys pass through untouched.
- The **emergency stop** always takes precedence.

### Scroll
- Invert vertical and/or horizontal scrolling.
- Swap axes when a driver reports the wrong wheel direction.
- Send 1–5 notches per physical notch, with optional experimental acceleration.
- Per-application overrides, so a game can keep normal scrolling while the
  desktop is inverted.
- Only physical wheel events are transformed; MouseMacro's own injected scroll
  is tagged and never remapped.
- **Standalone wheel triggers** — map the wheel itself (up, down, horizontal
  left/right, including thumb/tilt wheels) in Button mappings. Precedence is
  button + wheel gesture → layer wheel override → standalone wheel → pass
  through. A per-mapping 80 ms cooldown stops a fast wheel from firing
  repeatedly, and *also scroll* is off by default.

### Choosing keys
Two independent paths, both usable offline:

1. **Choose** — a searchable key catalog (`page`, `f5`, `numpad`, `volume`, …)
   with modifier chips. Pure UI, no global hook involved.
2. **Record** — captures the next shortcut through the shared capture session.
   The log pairs presses into gestures, keeps them after Stop, and lets you
   click any earlier gesture to use it instead. Input from vendor software,
   remote sessions and on-screen keyboards is recorded too. A session ends by
   itself after 20 seconds.

If a game or secure window blocks the global hook, the focused window records
the key as a fallback, and path 1 always works.

### Gesture engine guarantees
The engine is a pure state machine (`src-tauri/src/engine/`), tested without
touching Windows APIs:

- Gestures (button + button, button + wheel) always win over taps.
- Wheel precedence is fixed: button + wheel gesture, layer override, standalone
  wheel, then pass-through.
- Keyboard matching is exact and repeat-safe; a held remap releases on key-up.
- Long press wins over double click and single click.
- A tap is suppressed once a gesture or long press has fired — no double action.
- Unmatched input is replayed to the OS instead of being swallowed.
- Synthetic holds are released on unmount, disable, reset and app exit.
- Injected events are tagged and ignored, so mappings cannot recurse.
- When the engine is disabled, every event passes straight through.

### Mapping workflow
- Search by button, trigger, action or label.
- Duplicate a mapping, enable/disable all, and test an action from the editor.
- Duplicate button + trigger pairs are marked **shadowed** because the engine
  always uses the first enabled match.
- Each mapping shows when it last fired.

### Profiles
- Create, rename, duplicate and delete profiles.
- Seeded examples: Default, Productivity, Gaming, Design.
- Import/export a profile as JSON, or export everything at once.
- Switch profiles from the sidebar or the tray menu.

### Backup and restore
- Export/import all settings, profiles, layers and application rules as one
  versioned JSON file.
- Ten rotating backups are written before every save; restore one click, and
  the current state is backed up first.
- Imported files never change Windows startup integration.

### Desktop behaviour
- **Emergency stop**: global hotkey (default `Ctrl + Alt + Shift + F12`) and a
  tray item that disable mappings and release every synthetic key or button.
  Resume from the tray, sidebar or Settings.
- First-run setup: button detection, starter profile and startup choices.
- System tray icon: open, enable/disable, switch profile, stop all input, exit.
- Launch on Windows startup (registry reconciled with config at boot).
- Start minimized, close-to-tray, light/dark/system themes.
- **Search settings** from the Settings header; results jump to the setting,
  open its card and highlight it.
- Optional relaunch as administrator for elevated applications.
- **Input diagnostics** in Settings: press Start, use the mouse, press Stop — the
  last mouse events are listed with axis, delta and whether they were filtered
  as synthetic. Also an opt-in to process input synthesized by other utilities
  (mouse vendor software, touchpad drivers, remote desktop), and a **Reinstall**
  button that restores the low-level mouse hook if Windows dropped it.

### Safety
- Atomic config writes with rotating timestamped backups and corrupt-file
  recovery.
- Hook and hotkey failures are surfaced in the UI, never silently ignored.
- Corrupt configuration cannot brick startup; defaults are restored.
- An app switch or emergency stop releases every synthetic hold, including
  drag locks.

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
npm run test:rust   # 125 engine/config/hook/capture/service tests
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
  keyboard hook has its own thread, installed while keyboard remapping is
  enabled and for every recording session.
- `platform/capture.rs` — recording sessions: buffered events, gesture
  selection, suppression of the recorded click and a 20 s watchdog.
- `platform/service.rs` — adapts raw events to engine input, runs the timer for
  long press / double click deadlines, and executes effects.
- `engine/` — UI-independent rules, state and timing. Deterministic and fully
  unit-tested.
- `platform/simulate.rs` — `SendInput` shortcuts, clicks, holds, Unicode text,
  wheel scrolling, media keys, window actions and shell launches, tagged with
  `INJECTION_TAG` so the hook ignores them.
- `platform/service.rs` — adapts raw events to engine input, applies per-app
  wheel transforms, runs the timer for long press / double click deadlines, and
  executes effects including cancellable macros.
- `state.rs` — configuration, validation, snapshots and window control.
- `src/` — React UI: Dashboard, Button mappings, Layers, Profiles, Exceptions,
  Keyboard, Scroll, Settings, About.

## Configuration

Stored as JSON at `%APPDATA%\com.mousemacro.app\config.json` and written
atomically. Shape:

```json
{
  "schemaVersion": 6,
  "settings": {
    "enabled": true,
    "launchAtStartup": false,
    "startMinimized": false,
    "closeToTray": true,
    "theme": "system",
    "longPressMs": 300,
    "doubleClickMs": 200,
    "rulePriority": ["gesture", "longPress", "doubleClick", "release", "singleClick"],
    "panicHotkey": ["ctrl", "alt", "shift", "f12"],
    "onboarded": true,
    "keyboard": {
      "enabled": false,
      "acknowledged": false
    },
    "scroll": {
      "invertVertical": false,
      "invertHorizontal": false,
      "swapAxes": false,
      "speed": 1,
      "acceleration": false
    }
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
        },
        {
          "id": "…",
          "name": "Zoom in",
          "button": null,
          "trigger": { "type": "wheel", "direction": "up" },
          "action": { "type": "shortcut", "keys": ["ctrl", "equal"] },
          "enabled": true,
          "wheel": { "cooldownMs": 80, "alsoScroll": false }
        },
        {
          "id": "…",
          "name": "Pan",
          "button": "middle",
          "trigger": { "type": "singleClick" },
          "action": {
            "type": "combined",
            "actions": [
              { "type": "holdKeys", "keys": ["h"] },
              { "type": "mouseHold", "button": "left" }
            ]
          },
          "enabled": true
        }
      ],
      "keyMappings": [
        {
          "id": "…",
          "name": "Caps Lock to Ctrl",
          "source": ["capslock"],
          "behaviour": { "mode": "remap", "target": ["ctrl"] },
          "enabled": true
        }
      ],
      "layers": [
        {
          "id": "…",
          "name": "Navigation",
          "activator": "x1",
          "enabled": true,
          "mappings": []
        }
      ]
    }
  ],
  "activeProfileId": "…",
  "exceptions": [
    {
      "id": "…",
      "process": "game.exe",
      "mode": "disable",
      "profileId": null,
      "enabled": true,
      "keyboardDisabled": false
    }
  ]
}
```

Gesture and long-press precedence are fixed; the order of double click, release
and single click is configurable in Settings.

## Project layout

```
src/                     React UI
  components/            Sheet editor, key picker, capture log, controls
  hooks/                 recording session bridge, diagnostics monitor
  lib/                   types, formatting, key catalog, API client
  pages/                 Dashboard, Mappings, Layers, Profiles, Exceptions,
                         Keyboard, Scroll, Settings, About
  state/                 app context (snapshot, view, run helpers)
src-tauri/src/
  config/                model, defaults, atomic store, backups, import/export
  engine/                pure rule engine, layers, key tables
  platform/              hooks, service, foreground watcher, hotkey, SendInput,
                         elevation
  commands.rs            Tauri command surface
  tray.rs                tray menu
  lib.rs                 setup, plugins, window sizing, shutdown cleanup
```

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| No window after launch | It starts in the tray (`startMinimized`). Use the tray icon, or run the exe again to show the existing instance. Disable *Start minimized* in Settings. |
| Closing the window doesn't quit | *Close to tray* is on. Use tray → Exit. |
| Shortcut recording sees nothing | Keep the MouseMacro window focused while recording; the focused window records the key if the global hook is blocked. In games or secure windows use **Choose** — no hook required. |
| A keyboard remap does nothing | Check that **Keyboard** is on and acknowledged, that the combination matches exactly (`F12` is not `Ctrl + F12`), and that Exceptions does not disable the keyboard for the current app. |
| A recording stops by itself | A recording session ends after 20 seconds. Press Record again. |
| Scroll gestures do nothing | Settings → **Input diagnostics**: if wheel rows say `synthetic`, enable *Process input from other input tools*. If they say `horizontal`, the device is a thumb/tilt wheel — map `left`/`right` triggers. |
| Hand-edited config.json has no effect | Settings → **Reload from disk** (or restart). The running app keeps its in-memory copy. |
| Shortcuts don't work inside a game | If the game runs elevated, relaunch MouseMacro via *Settings → Run as administrator*. |
| An application rule does not apply | Check the executable name in **Exceptions**; elevated apps need MouseMacro elevated too. The status chip shows the detected foreground app. |
| A layer does nothing | The activator must be held; each button can own only one layer; check that the layer and its mappings are enabled. |
| A mapping is marked *Shadowed* | Another enabled mapping has the same button and trigger. Disable, edit or remove one; the engine uses the first match. |
| Mappings suddenly stopped | The emergency hotkey (`Ctrl + Alt + Shift + F12`) or tray → *Stop all input* was used. Resume from the sidebar, tray or **Settings → Safety**. |
| Need an older configuration | **Settings → Backup and restore** → choose a timestamped backup → Restore. |
| SmartScreen on another PC | The build is unsigned: *More info → Run anyway*. |

## Roadmap

1.6 added Settings search, faster gesture defaults and layout polish. The
remaining input-suite work is:

1. **Pointer and keyboard timing** (1.7) — Windows pointer speed and
   acceleration, primary button, double-click time, and keyboard repeat
   delay/rate, with snapshot/restore.
2. **Keyboard follow-ups** (1.7) — scan-code mode for non-US layouts, F12 as a
   mouse modifier, and an opt-in keyboard diagnostics list.

OS-level changes will snapshot and restore original Windows values, with explicit
warnings and no silent modification.

## Changelog

Release notes for every version live in [CHANGELOG.md](CHANGELOG.md).
