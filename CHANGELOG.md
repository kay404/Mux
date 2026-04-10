# Changelog

All notable changes to Mux will be documented in this file.

## [0.1.0.1] - 2026-04-10

### Changed
- Background polling now only runs when the popover is visible, reducing idle CPU and memory usage
- Added visibility state tracking via AtomicBool to gate the 5-second reconciliation loop

### Fixed
- Popover data no longer goes stale if the window briefly loses and regains focus
- Unexpected window destruction no longer leaves the poll loop running indefinitely
