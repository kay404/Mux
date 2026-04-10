<h1 align="center">Mux</h1>
<p align="center">
  <b>Mux between your projects</b><br>
  <a href="#install">Install</a> •
  <a href="#features">Features</a> •
  <a href="#how-it-works">How It Works</a> •
  <a href="#development">Development</a><br>
  English | <a href="README.zh-CN.md">简体中文</a>
</p>

---

A macOS menu bar app for switching between developer tool windows. One click to bring any project to the front.

You have 6+ VSCode windows open. Cmd+Tab shows 30 windows across all apps. Finding the right project takes too long. Mux fixes that.

![macOS](https://img.shields.io/badge/platform-macOS-lightgrey)
![Tauri](https://img.shields.io/badge/built%20with-Tauri%20v2-blue)
![License](https://img.shields.io/badge/license-MIT-green)

## Features

- Auto-detects running VSCode, VSCode Insiders, and Cursor
- Shows project name + full file path for each open window
- Click to bring any project window to the front (two-step focus: app activate + window raise)
- Real app icons from each IDE's .app bundle
- Keyboard navigation (Arrow keys, Enter, Escape)
- Dark theme with macOS vibrancy blur
- 5-second background refresh keeps the list current (only polls when popover is visible)
- First-launch Accessibility permission flow
- No configuration needed

## Screenshot

```
┌─────────────────────────────────┐
│  [icon] Visual Studio Code      │
│  ● my-project    ~/work/my-p..  │
│    api-server    ~/work/api-..  │
│    frontend      ~/dev/front..  │
│                                 │
│  [icon] Cursor                  │
│    dashboard     ~/work/dash..  │
└─────────────────────────────────┘
```

## Requirements

- macOS (Sonoma or later recommended)
- Accessibility permission (one-time grant on first launch)

## Install

### Build from source

```bash
git clone https://github.com/kay404/Mux.git
cd Mux
npm install
npm run tauri build
```

The built app is at `src-tauri/target/release/bundle/macos/Mux.app`. Drag it to `/Applications`.

## How It Works

1. Finds running IDEs via `NSRunningApplication` (by bundle ID)
2. Reads window titles via macOS Accessibility API (`AXUIElement`)
3. Parses project names from titles (handles both default and custom title formats)
4. Resolves full paths by querying VSCode's `state.vscdb` (SQLite)
5. Focuses windows with two-step approach: `NSRunningApplication.activate` + `AXUIElementPerformAction(kAXRaiseAction)`

## Supported Apps

| App | Bundle ID |
|-----|-----------|
| Visual Studio Code | `com.microsoft.VSCode` |
| VSCode Insiders | `com.microsoft.VSCodeInsiders` |
| Cursor | `com.todesktop.230313mzl4w4u92` |

## Development

```bash
npm install
npm run tauri dev
```

### Tech Stack

- **Tauri v2** + Rust backend
- **Vanilla JS/HTML/CSS** frontend
- **macOS Accessibility API** via raw FFI + core-foundation
- **SQLite** (rusqlite) for reading VSCode workspace state

### Project Structure

```
├── src-tauri/src/
│   ├── lib.rs              # App setup, tray, popover, state management
│   ├── accessibility.rs    # macOS AX API: find apps, list windows, focus
│   ├── title_parser.rs     # Parse project names from window titles
│   ├── path_resolver.rs    # Resolve paths via state.vscdb
│   └── icon_cache.rs       # App icon extraction + disk/memory cache
├── src/
│   ├── index.html          # Popover HTML
│   ├── main.js             # Frontend logic
│   └── styles.css          # Dark vibrancy theme
└── docs/
    ├── PRD.md              # Product requirements
    ├── DESIGN.md           # Visual design spec
    └── DEVELOPMENT.md      # Development guide
```

## Limitations

- macOS only
- Cannot distribute via Mac App Store (Accessibility API requires non-sandboxed execution)
- Antigravity IDE support planned for a future release

## License

MIT
