# Configuration reference

Tickt reads one Linux TOML file:

```text
${XDG_CONFIG_HOME:-$HOME/.config}/tickt/config.toml
```

Create it without overwriting an existing file:

```sh
tickt config init
```

Unknown fields and invalid values are rejected during startup. A relative XDG variable is ignored
and the Linux default is used; XDG overrides must be absolute paths.

## Timer

```toml
[timer]
focus_minutes = 25          # 1..=240
short_break_minutes = 5      # 1..=120
long_break_minutes = 15      # 1..=240
long_break_interval = 4      # 1..=12 completed focus sessions
```

## Productivity

```toml
[productivity]
daily_goal_minutes = 100    # 1..=1440
```

The goal is used for daily progress and streaks. Only fully completed focus sessions count.

## User interface

```toml
[ui]
theme = "dark"               # dark, light, or default
unicode = true                # false uses ASCII progress bars
```

## Optional alerts

```toml
[notifications]
enabled = true                # best-effort notify-send integration
sound = false                 # terminal bell
```

Missing `notify-send` or desktop notification services never stop the timer.

## Recovery

```toml
[recovery]
on_startup = "ask"            # ask, resume, or discard
```

`resume` only resumes an active timer that has not expired. An expired timer is never counted as
completed work. A paused timer remains paused across restarts.

The complete valid example is `examples/config.toml`.
