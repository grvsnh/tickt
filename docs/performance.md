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

## v1.1 development benchmarks

The large-history fixture is reproducible without extra Python packages:

```sh
cargo build --release --locked --target x86_64-unknown-linux-musl
python3 scripts/benchmark-large-history.py --rows 100000 --repeats 10
python3 scripts/benchmark-large-history.py --rows 1000000 --repeats 10
```

It reports median and p95 timings for `stats` and bounded history, and prints the SQLite query
plan so an index regression is visible. It uses a temporary database and never touches user data.

SQLite write latency can be measured with:

```sh
cargo run --release --locked --example benchmark_storage
```

The benchmark measures complete file-backed operations, including transaction commit. A local
run on the development machine reported these p50 values:

| Operation | p50 |
| --- | ---: |
| `save_active` | 6.25 ms |
| `finish_session` | 6.81 ms |
| duplicate completion | 0.09 ms |
| `clear_active` | 6.32 ms |

These are a baseline, not a universal target. The benchmark uses temporary storage and SQLite's
default durability settings; changing those settings would trade crash durability for latency.

## What remains intentionally simple

- The event loop remains synchronous; notification delivery is best-effort and non-blocking.
- History display is bounded to recent rows.
- The release profile uses thin LTO to balance runtime performance with build time.
- The interactive statistics accumulator avoids re-reading historical rows after each successful
  completion, while startup still loads the source-of-truth history once.
- The TUI polls frequently enough for timer completion but skips draws when the visible state has
  not changed.
- A statistics cache or background worker is not justified until real users show larger databases
  or noticeable post-session latency.

If performance work continues, measure these next: key-to-render latency on slow filesystems,
startup with a million sessions, and redraw CPU usage during a long idle run.
