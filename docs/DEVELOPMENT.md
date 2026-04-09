# Mux — Development Guide

## Tech Stack

- **Tauri v2** — app shell, tray icon, WebView popover
- **Rust** — backend (macOS API calls, window detection, path resolution)
- **Vanilla JS/HTML/CSS** — frontend (popover UI)
- **macOS Accessibility API** — window enumeration and focus
- **SQLite** — read VSCode's `state.vscdb` for project paths

## Prerequisites

- macOS (Sonoma or later recommended)
- Rust toolchain: `rustup`, `cargo` (1.70+)
- Node.js (18+) and npm
- Accessibility permission granted to your terminal app

## Quick Start

```bash
cd mux-app
npm install
npm run tauri dev
```

The app appears as a `{ }` icon in the menu bar. Click it to see open projects.

## Project Structure

```
mux-app/
├── src-tauri/
│   ├── Cargo.toml              # Rust dependencies
│   ├── tauri.conf.json         # Tauri config (no main window, tray-only)
│   ├── capabilities/
│   │   └── default.json        # Permission capabilities for popover
│   └── src/
│       ├── main.rs             # Entry point
│       ├── lib.rs              # App setup, tray, popover, state, refresh loop
│       ├── accessibility.rs    # macOS AX API: find apps, list windows, focus
│       ├── title_parser.rs     # Parse project names from window titles
│       ├── path_resolver.rs    # Resolve full paths via state.vscdb (SQLite)
│       └── icon_cache.rs       # Extract app icons, disk + memory cache
├── src/
│   ├── index.html              # Popover HTML shell
│   ├── main.js                 # Frontend: render, keyboard nav, Tauri invoke
│   └── styles.css              # Dark vibrancy theme
└── docs/
    ├── PRD.md                  # Product requirements
    ├── DESIGN.md               # Visual design specification
    └── DEVELOPMENT.md          # This file
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   macOS System                       │
│                                                      │
│  ┌──────────┐   AX Notifications   ┌────────────┐  │
│  │  VSCode   │ ─────────────────→  │ AXObserver │  │
│  │  Cursor   │                      │ (per app)  │  │
│  │  Insiders │                      └─────┬──────┘  │
│  └──────────┘                             │         │
│                                    callback│         │
│  ┌──────────┐   SQLite read       ┌───────▼──────┐  │
│  │state.vscdb│ ←─────────────────│  Rust Backend │  │
│  └──────────┘                    └───────┬──────┘  │
│                                           │         │
└───────────────────────────────────────────┼─────────┘
                                            │ Tauri invoke
┌───────────────────────────────────────────▼─────────┐
│  Tauri v2 Shell                                      │
│  ┌─────────────┐     ┌──────────────────────────┐   │
│  │ Tray Icon    │────→│ WebView Popover          │   │
│  │ { }          │     │ (project list UI)        │   │
│  └─────────────┘     └──────────────────────────┘   │
└──────────────────────────────────────────────────────┘
```

## Key Design Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| macOS API bindings | Raw FFI + core-foundation | Direct control, minimal dependencies |
| Window detection | 5s reconciliation poll | AXObserver planned for V2, polling is reliable |
| Permission model | Accessibility API only | Single permission, covers both listing and focus |
| Window focus | NSRunningApplication.activate + AXRaise | Both steps required (verified) |
| Path resolution | Title parsing + state.vscdb SQLite lookup | Folder name from title, full path from DB |
| Icon caching | Disk (~/.cache/mux/icons/) + memory HashMap | Fast cold start, extract once per app |
| Tray UI | Custom WebView popover | Native menus can't render icons or rich layout |
| App visibility | ActivationPolicy::Accessory | Hides from dock, menu bar only |

## How Window Detection Works

1. **Find running apps**: Call `NSRunningApplication.runningApplicationsWithBundleIdentifier:`
   for each supported bundle ID to get PIDs
2. **List windows**: Call `AXUIElementCreateApplication(pid)` then
   `AXUIElementCopyAttributeValue(kAXWindowsAttribute)` to get window list
3. **Parse titles**: Extract project name from window title. Handles both default
   (`"file — project — Visual Studio Code"`) and custom (`"file — project"`) formats
4. **Resolve paths**: Open `~/Library/Application Support/Code/User/globalStorage/state.vscdb`,
   query `history.recentlyOpenedPathsList`, match folder name to `folderUri`
5. **Focus**: Two-step: `NSRunningApplication.activate(options:)` then
   `AXUIElementPerformAction(kAXRaiseAction)`

## State.vscdb Paths

| App | Database Location |
|-----|------------------|
| VSCode | `~/Library/Application Support/Code/User/globalStorage/state.vscdb` |
| VSCode Insiders | `~/Library/Application Support/Code - Insiders/User/globalStorage/state.vscdb` |
| Cursor | `~/Library/Application Support/Cursor/User/globalStorage/state.vscdb` |

The database has a single `ItemTable` with `key TEXT, value BLOB`. Query:

```sql
SELECT value FROM ItemTable WHERE key = 'history.recentlyOpenedPathsList'
```

Returns JSON:

```json
{
  "entries": [
    { "folderUri": "file:///Users/dev/my-project" },
    { "folderUri": "file:///Users/dev/another" }
  ]
}
```

## Tauri v2 Known Issues

| Bug | Tauri Issue | Workaround |
|-----|-------------|------------|
| Focus stealing on popover click | #14102 | Delayed blur handler (100ms check) |
| Crash when no windows exist | #8812 | Guard event handlers |
| Double tray icon | #10912 | Single TrayIcon setup |

## Running Tests

```bash
# Rust unit tests (title parser, path resolver, URL decoder)
cargo test --manifest-path src-tauri/Cargo.toml

# 18 tests total
```

## Building for Release

```bash
npm run tauri build
```

Output: `src-tauri/target/release/bundle/macos/Mux.app`

## Debugging

Debug logs are printed to stderr with `[Mux]` prefix when running in dev mode.
They show:
- AX trusted status
- PIDs found per bundle ID
- Window titles per PID
- Parsed project names
- Resolved paths

## Adding a New Dev Tool

1. Add a new entry to `DEV_APPS` in `src-tauri/src/accessibility.rs`:
   ```rust
   DevApp {
       name: "Your IDE",
       bundle_id: "com.example.ide",
       storage_path: "YourIDE/User/globalStorage/state.vscdb",
       title_suffix: "Your IDE",
   },
   ```
2. Verify the window title format and adjust `title_parser.rs` if needed
3. Verify the workspace storage path and format
