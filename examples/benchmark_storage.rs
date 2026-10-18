use std::time::Instant;

use chrono::Utc;
use tempfile::tempdir;
use tickt::model::{
    ActiveSessionSnapshot, ActiveStatus, SessionRecord, SessionStatus, SessionType,
};
use tickt::storage::Storage;

fn percentile(samples: &mut [f64], percentile: f64) -> f64 {
    samples.sort_by(f64::total_cmp);
    let index = ((samples.len() - 1) as f64 * percentile).round() as usize;
    samples[index]
}

fn summarize(name: &str, mut samples: Vec<f64>) {
    let p50 = percentile(&mut samples, 0.50);
    let p95 = percentile(&mut samples, 0.95);
    let p99 = percentile(&mut samples, 0.99);
    println!("operation={name} p50_us={p50:.1} p95_us={p95:.1} p99_us={p99:.1}");
}

fn active_snapshot(id: &str) -> ActiveSessionSnapshot {
    let now = Utc::now();
    ActiveSessionSnapshot {
        id: id.to_owned(),
        session_type: SessionType::Focus,
        status: ActiveStatus::Running,
        started_at: now,
        running_since: Some(now),
        planned_seconds: 1_500,
        elapsed_seconds: 10,
    }
}

fn completed_record(id: &str) -> SessionRecord {
    let started_at = Utc::now();
    SessionRecord {
        id: id.to_owned(),
        session_type: SessionType::Focus,
        started_at,
        ended_at: Some(started_at + chrono::Duration::minutes(25)),
        planned_seconds: 1_500,
        actual_seconds: 1_500,
        status: SessionStatus::Completed,
    }
}

fn main() -> anyhow::Result<()> {
    let directory = tempdir()?;
    let storage = Storage::open(&directory.path().join("writes.db"))?;
    let samples = 100_usize;

    for _ in 0..10 {
        storage.save_active(&active_snapshot("warmup"))?;
        storage.clear_active()?;
    }

    let mut save_active_samples = Vec::with_capacity(samples);
    for index in 0..samples {
        let started = Instant::now();
        storage.save_active(&active_snapshot(&format!("active-{index}")))?;
        save_active_samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    summarize("save_active", save_active_samples);

    let mut finish_samples = Vec::with_capacity(samples);
    for index in 0..samples {
        let started = Instant::now();
        storage.finish_session(&completed_record(&format!("finished-{index}")))?;
        finish_samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    summarize("finish_session", finish_samples);

    let duplicate = completed_record("duplicate");
    storage.finish_session(&duplicate)?;
    let mut duplicate_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        storage.finish_session(&duplicate)?;
        duplicate_samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    summarize("duplicate_finish", duplicate_samples);

    let mut clear_samples = Vec::with_capacity(samples);
    for index in 0..samples {
        storage.save_active(&active_snapshot(&format!("clear-{index}")))?;
        let started = Instant::now();
        storage.clear_active()?;
        clear_samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    summarize("clear_active", clear_samples);

    println!("samples={samples} filesystem=temporary-file-backed sqlite=default-pragmas");
    Ok(())
}
