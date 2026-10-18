#!/usr/bin/env python3
"""Measure statistics and bounded history against deterministic SQLite fixtures."""

from __future__ import annotations

import argparse
import os
import statistics
import subprocess
import tempfile
import time
from pathlib import Path
import sqlite3


def create_database(path: Path, rows: int) -> None:
    connection = sqlite3.connect(path)
    connection.executescript(
        """
        CREATE TABLE sessions (
            id TEXT PRIMARY KEY NOT NULL,
            session_type TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            ended_at INTEGER,
            planned_seconds INTEGER NOT NULL,
            actual_seconds INTEGER NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch())
        );
        CREATE INDEX sessions_started_at_idx ON sessions (started_at DESC);
        CREATE INDEX sessions_completed_focus_idx ON sessions (started_at)
            WHERE session_type = 'focus'
              AND status = 'completed'
              AND actual_seconds >= planned_seconds;
        CREATE TABLE active_timer (
            slot INTEGER PRIMARY KEY,
            session_id TEXT NOT NULL UNIQUE,
            session_type TEXT NOT NULL,
            state TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            planned_seconds INTEGER NOT NULL,
            elapsed_seconds INTEGER NOT NULL,
            running_since INTEGER
        );
        PRAGMA user_version = 3;
        """
    )
    batch = []
    for index in range(rows):
        session_type = "focus" if index % 5 else "short_break"
        status = "completed" if index % 7 else "skipped"
        planned = 1_500 if session_type == "focus" else 300
        actual = planned if status == "completed" else planned // 2
        started = 1_700_000_000 + index
        batch.append(
            (f"session-{index}", session_type, started, started + actual, planned, actual, status)
        )
        if len(batch) == 1_000:
            connection.executemany(
                "INSERT INTO sessions VALUES (?, ?, ?, ?, ?, ?, ?, unixepoch())", batch
            )
            batch.clear()
    if batch:
        connection.executemany("INSERT INTO sessions VALUES (?, ?, ?, ?, ?, ?, ?, unixepoch())", batch)
    connection.commit()
    connection.close()


def measure(binary: str, env: dict[str, str], command: list[str], repeats: int) -> tuple[float, float]:
    samples = []
    for _ in range(repeats):
        started = time.perf_counter()
        subprocess.run([binary, *command], env=env, check=True, stdout=subprocess.DEVNULL)
        samples.append((time.perf_counter() - started) * 1000)
    percentile_95 = statistics.quantiles(samples, n=20, method="inclusive")[18]
    return statistics.median(samples), percentile_95


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/x86_64-unknown-linux-musl/release/tickt")
    parser.add_argument("--rows", type=int, default=100_000)
    parser.add_argument("--repeats", type=int, default=10)
    args = parser.parse_args()
    if args.rows < 1 or args.repeats < 3:
        parser.error("--rows must be positive and --repeats must be at least 3")

    with tempfile.TemporaryDirectory(prefix="tickt-benchmark-") as directory:
        root = Path(directory)
        data = root / "data" / "tickt"
        data.mkdir(parents=True)
        create_database(data / "tickt.db", args.rows)
        environment = os.environ.copy()
        environment.update(
            {
                "HOME": str(root / "home"),
                "XDG_CONFIG_HOME": str(root / "config"),
                "XDG_DATA_HOME": str(root / "data"),
                "XDG_STATE_HOME": str(root / "state"),
            }
        )
        binary = str(Path(args.binary).resolve())
        for command in (["stats"], ["history", "--limit", "50"]):
            subprocess.run([binary, *command], env=environment, check=True, stdout=subprocess.DEVNULL)
            median, p95 = measure(binary, environment, command, args.repeats)
            print(f"rows={args.rows} command={' '.join(command)} median_ms={median:.3f} p95_ms={p95:.3f}")

        connection = sqlite3.connect(data / "tickt.db")
        plan = connection.execute(
            "EXPLAIN QUERY PLAN SELECT started_at, planned_seconds, actual_seconds FROM sessions "
            "WHERE started_at >= 0 AND session_type = 'focus' AND status = 'completed' "
            "AND actual_seconds >= planned_seconds ORDER BY started_at ASC"
        ).fetchall()
        connection.close()
        print(f"statistics_query_plan={plan}")


if __name__ == "__main__":
    main()
