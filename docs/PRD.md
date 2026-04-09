# Mux — Product Requirements Document

## Overview

Mux is a macOS menu bar app that shows currently running developer tools and their open projects. Click a project to bring its window to the front. No configuration needed.

## Problem

Developers who work with multiple IDE windows (VSCode, Cursor, VSCode Insiders) have no dev-focused way to switch between projects. macOS Cmd+Tab and tools like AltTab show ALL open windows (browser, Slack, Finder, etc.). When you have 6+ VSCode windows open, finding the right project in a sea of 30+ windows is real friction.

## Solution

A menu bar utility that:
- Detects running developer tools automatically
- Lists their open projects with names and file paths
- Brings a specific project window to the front with one click
- Updates in real time as windows open and close

## Target Users

Developers on macOS who regularly work with multiple VSCode/Cursor windows.

## Supported Apps (V1)

| App | Bundle ID |
|-----|-----------|
| Visual Studio Code | `com.microsoft.VSCode` |
| VSCode Insiders | `com.microsoft.VSCodeInsiders` |
| Cursor | `com.todesktop.230313mzl4w4u92` |

## Core User Flow

1. Mux runs in the menu bar (tray icon: `{ }`)
2. User clicks the tray icon
3. A popover shows all open dev projects, grouped by app
4. User clicks a project name
5. That project's window comes to the front
6. Popover auto-hides

## First Launch Flow

1. App starts, tray icon appears
2. User clicks tray icon
3. If Accessibility permission not granted: show permission card with instructions
4. User grants permission in System Settings
5. App detects permission change, shows project list

## Non-Goals (V1)

- Antigravity IDE support (V1.1, title format unknown)
- Mac App Store distribution (permanently blocked by AX API + Sandbox)
- Keyboard shortcuts / global hotkeys
- Git branch info
- Search / filter
- Settings panel
- Auto-update

## Success Criteria

- [ ] Menu bar icon appears on launch
- [ ] Clicking icon shows popover with detected projects
- [ ] Each project shows name + path
- [ ] Each app section shows the real app icon
- [ ] Clicking a project brings that window to the front
- [ ] Empty state when no dev tools are running
- [ ] First-launch permission request works
- [ ] List updates in real time when windows open/close
- [ ] Binary under 10MB

## Distribution

Direct distribution only (.app binary, future: signed .dmg + Homebrew cask).
Mac App Store is permanently impossible due to Accessibility API requiring
non-sandboxed execution.

## Constraints

- macOS only
- Requires Accessibility permission (one-time grant)
- Cannot run in App Sandbox
