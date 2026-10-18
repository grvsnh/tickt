# Tickt architecture

Tickt is a synchronous, local-first Rust application. The event loop owns the application
state and SQLite connection; there is no daemon, network service, or background timer thread.

```text
                    ┌───────────────┐
terminal events ───▶│ app event loop │───▶ Ratatui renderer
                    └──────┬────────┘
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
        TimerEngine    Statistics     Storage
        (Instant)      (pure)         (SQLite)
             │                           │
             └────────── config ─────────┘
                      (TOML/XDG)
```

## Timer correctness

While the process is running, `TimerEngine` derives elapsed time from `Instant`, not from the
number of redraws or one-second sleeps. Rendering can be delayed without making the countdown
drift. A running active timer is also saved with a UTC start timestamp. On restart, an
unexpired timer resumes from the saved wall-clock elapsed time; a timer that expired while
Tickt was closed is recorded as interrupted after the user acknowledges recovery. This is
deliberately conservative: Tickt does not fabricate observed work during downtime.

## Persistence

`Storage` creates and migrates a SQLite database at the XDG data path. Session completion and
active-state deletion use one transaction, and session IDs are unique so repeated completion
events cannot create duplicate history. Configuration remains a human-editable TOML file and
mutable timer state never goes into that file.

## Statistics

Only completed focus sessions that reached their planned duration count toward focus totals.
Breaks, skips, resets, interruptions, and partial sessions are excluded. Dates are grouped by
the local calendar day at presentation time. A streak is a run of days reaching the configured
daily goal; before today's goal is reached, the current streak is measured backward from
yesterday.

## Terminal lifecycle

`TerminalGuard` enters raw mode and the alternate screen, then restores raw mode, the screen,
and cursor in `Drop`. The application keeps rendering separate from input and persistence so
the TUI can be tested and evolved without putting SQL in widget code.
