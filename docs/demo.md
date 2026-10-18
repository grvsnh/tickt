# Demo checklist

Use a temporary config for a fast demonstration; do not put demo data in a user's normal
database.

```sh
mkdir -p /tmp/tickt-demo/config/tickt /tmp/tickt-demo/data/tickt
cat >/tmp/tickt-demo/config/tickt/config.toml <<'EOF'
[timer]
focus_minutes = 1
short_break_minutes = 1
long_break_minutes = 2
long_break_interval = 2
[productivity]
daily_goal_minutes = 1
EOF
XDG_CONFIG_HOME=/tmp/tickt-demo/config \
XDG_DATA_HOME=/tmp/tickt-demo/data \
cargo run --locked
```

Then:

1. Press Space to start the one-minute focus session.
2. Press Space to pause and again to resume.
3. Let it complete, then press `n` for the suggested break.
4. Open `h` for persisted activity and `t` for the daily summary.
5. Quit with `q`, relaunch, and confirm that the database-backed history remains.

The production default remains 25/5/15 minutes. Tickt does not insert fake completions or
sample statistics.
