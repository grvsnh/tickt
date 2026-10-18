# Performance notes

Tickt is designed to start quickly and remain quiet while idle. Performance work should be
measured against a real workload rather than judged from build time alone.

## Current optimization

Statistics used to load and decode every session row before filtering it in Rust. The storage
layer now filters completed focus sessions in SQLite and reads only the three fields statistics
needs. A partial index supports that query:

```sql
session_type = 'focus'
status = 'completed'
actual_seconds >= planned_seconds
```

This keeps breaks, skips, resets, interruptions, and partial sessions out of the statistics
read path. The application also seeds the timer from the statistics result instead of running a
second full completed-focus count query during startup.

## Measured local result

Workload: 100,000 synthetic SQLite session rows, warm local filesystem cache, optimized static
Linux binary, ten process launches per command on the development machine.

| Command | Before | After |
| --- | ---: | ---: |
| `tickt stats` | 104.3 ms median | 44.8 ms median |
| `tickt history --limit 50` | 2.2 ms median | 2.1 ms median |

The history result is within normal process and filesystem variation; it was not the target of
the optimization. The statistics path improved by about 57% for this large-history workload.
An earlier post-change run measured 37.1 ms; the final repeated run measured 44.8 ms. The table
reports the final median of ten runs rather than the best launch.

Reproduce a simple timing loop with Python's standard library:

```sh
python3 - <<'PY'
import os, statistics, subprocess, time

binary = "target/x86_64-unknown-linux-musl/release/tickt"
env = os.environ.copy()
samples = []
for _ in range(10):
    started = time.perf_counter()
    subprocess.run([binary, "stats"], env=env, check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    samples.append((time.perf_counter() - started) * 1000)
print(f"median_ms={statistics.median(samples):.3f}")
PY
```

## What remains intentionally simple

- The event loop remains synchronous; notification delivery is best-effort and non-blocking.
- History display is bounded to recent rows.
- The release profile uses thin LTO to balance runtime performance with build time.
- A statistics cache or background worker is not justified until real users show larger databases
  or noticeable post-session latency.

If performance work continues, measure these next: key-to-render latency on slow filesystems,
startup with a million sessions, and redraw CPU usage during a long idle run.
