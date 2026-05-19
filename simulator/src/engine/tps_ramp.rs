use std::time::Duration;

use crate::config::ScenarioBlueprint;

/// Returns the target TPS at `elapsed_secs` into the simulation.
///
/// Phase schedule:
///   0% – 20% of duration  → linear ramp from base_tps to burst_tps
///   20% – 90% of duration → hold at burst_tps
///   90% – 100%            → linear decay back to base_tps
pub fn compute_target_tps(blueprint: &ScenarioBlueprint, elapsed_secs: f64) -> u64 {
    let total = blueprint.duration_seconds as f64;
    let base = blueprint.base_tps as f64;
    let burst = blueprint.burst_tps as f64;
    let progress = (elapsed_secs / total).clamp(0.0, 1.0);

    let tps = if progress < 0.20 {
        let t = progress / 0.20;
        base + t * (burst - base)
    } else if progress < 0.90 {
        burst
    } else {
        let t = (progress - 0.90) / 0.10;
        burst - t * (burst - base)
    };

    tps.round() as u64
}

/// Converts a per-worker TPS value into the sleep interval between log emissions.
///
/// Clamps to a minimum of 50µs to stay within tokio's timer resolution.
pub fn tps_to_interval(worker_tps: u64) -> Duration {
    if worker_tps == 0 {
        return Duration::from_millis(100);
    }
    let micros = 1_000_000u64 / worker_tps;
    Duration::from_micros(micros.max(50))
}

/// Returns how many worker goroutines to spawn for the given burst_tps.
/// Each worker targets ≤1000 TPS to keep sleep intervals ≥1ms.
pub fn worker_count(burst_tps: u64) -> usize {
    let n = (burst_tps + 999) / 1000;
    n.clamp(1, 64) as usize
}
