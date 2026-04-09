# Mux — Design Specification

## Design Philosophy

Utility-first. Mux is a tool, not a product showcase. Every pixel earns its
place by reducing the time between "I need that window" and having it in front of you.
The UI should feel like a native macOS popover, not an Electron app.

## Tray Icon

- PNG template image: two overlapping windows with a switch arrow
- 22×22 (@1x) and 44×44 (@2x Retina) in `src-tauri/icons/barIcon*.png`
- Rendered as a macOS template image (`icon_as_template(true)`) — auto adapts to light/dark menu bar

## Popover Layout

```
┌─────────────────────────────────┐
│ ▲ (notch toward tray icon)      │
├─────────────────────────────────┤
│                                 │
│  [icon] Visual Studio Code      │  App header
│  ─────────────────────────────  │
│  ● my-project    ~/work/my-p..  │  Active project (blue dot)
│    api-server    ~/work/api-..  │  Inactive project
│    frontend      ~/dev/front..  │
│                                 │
│  [icon] Cursor                  │  Second app group
│  ─────────────────────────────  │
│    dashboard     ~/work/dash..  │
│    docs          ~/repos/doc..  │
│                                 │
├─────────────────────────────────┤
│  Mux v0.1                 │  Footer
└─────────────────────────────────┘
```

### Dimensions

| Property | Value |
|----------|-------|
| Width | 320px |
| Max height | 400px (scrollable) |
| Background | macOS vibrancy blur (dark) |
| Rounded corners | 10px |
| Open animation | Slide down, 150ms ease-out |
| Dismiss | Click outside or Escape |

## Visual Tokens

| Token | Value |
|-------|-------|
| Font family | SF Pro (system) |
| Body text | 13px |
| Secondary text | 11px |
| Footer text | 10px |
| Text color | #ffffff |
| Secondary color | #666666 |
| Accent (active dot) | #007AFF (system blue) |
| Hover highlight | rgba(255, 255, 255, 0.1) |
| Spacing | 8px vertical between rows |
| Horizontal padding | 12px |
| App icon size | 16x16 in headers |
| Row height | 32px |
| Min contrast | 4.5:1 on vibrancy background |

## Interaction States

| State | What the user sees |
|-------|--------------------|
| Normal | App groups with project rows |
| Empty | Muted text: "No developer tools running" + "Open VSCode or Cursor to get started" |
| Permission needed | Lock icon + "Mux needs Accessibility permission" + [Open Settings] button |
| Stale window click | No-op, row removed on next refresh |
| App quit | Section removed on next refresh |
| Scrolling | Scroll within 400px max height |

## Keyboard Navigation

| Key | Action |
|-----|--------|
| Arrow Up/Down | Move between project rows |
| Enter | Focus selected project (same as click) |
| Escape | Close popover |
| Tab | Move between app sections |

## Accessibility

- VoiceOver: announce "app name, project name, path" for each row
- All interactive elements have ARIA roles and labels
- Minimum 4.5:1 contrast ratio
- Touch target minimum 32px height
