use crate::checks::{
    alert_latency::AlertLatencyResult,
    compression_check::CompressionResult,
    log_completeness::CompletenessResult,
    reorg_handling::ReorgResult,
};

pub struct AssertionReport {
    pub scenario_name: String,
    pub latency: AlertLatencyResult,
    pub completeness: CompletenessResult,
    pub reorg: ReorgResult,
    pub compression: CompressionResult,
}

/// Prints a structured pass/fail table. Returns true if all assertions passed.
pub fn print_report(report: &AssertionReport) -> bool {
    let width = 60;
    let sep = "═".repeat(width);

    println!();
    println!("╔{}╗", sep);
    println!(
        "║  ASSERTION RESULTS — {:<38}║",
        truncate(&report.scenario_name, 38)
    );
    println!("╠{}╣", sep);

    // Log completeness
    let c = &report.completeness;
    let c_label = if c.passed { "[PASS]" } else { "[FAIL]" };
    println!(
        "║  {}  Log Completeness    {} expected / {} actual ({:.4}% loss){:<3}║",
        c_label,
        c.expected,
        c.actual,
        c.loss_pct,
        ""
    );

    // Alert latency
    let l = &report.latency;
    let l_label = if l.passed { "[PASS]" } else { "[FAIL]" };
    if l.alerts_total == 0 {
        println!("║  {}  Alert Latency       No alerts table found — skipped{:<8}║", l_label, "");
    } else {
        println!(
            "║  {}  Alert Latency       {:.1}% sub-second (threshold: {:.0}%){:<6}║",
            l_label, l.sub_second_pct, l.threshold_pct, ""
        );
        println!(
            "║           max={:.0}ms  p99={:.0}ms  total_alerts={}{:<16}║",
            l.max_latency_ms, l.p99_latency_ms, l.alerts_total, ""
        );
    }

    // Reorg
    let r = &report.reorg;
    let r_label = if r.passed { "[PASS]" } else { "[FAIL]" };
    if r.skipped {
        println!("║  {}  Reorg Handling      No reorg configured — skipped{:<9}║", r_label, "");
    } else {
        println!(
            "║  {}  Reorg Handling      {} reorg logs / {} duplicates{:<11}║",
            r_label, r.actual_reorg_logs, r.duplicate_block_hashes, ""
        );
    }

    // Compression
    let k = &report.compression;
    let k_label = if k.passed { "[PASS]" } else { "[FAIL]" };
    if k.skipped {
        println!("║  {}  Compression Ratio   system.parts unavailable — skipped{:<4}║", k_label, "");
    } else {
        println!(
            "║  {}  Compression Ratio   {:.2}:1  (cost reduction: {:.0}%){:<11}║",
            k_label, k.ratio, k.cost_reduction_pct, ""
        );
    }

    println!("╠{}╣", sep);

    let all_passed = c.passed && l.passed && r.passed && k.passed;
    if all_passed {
        println!("║  ✓  ALL ASSERTIONS PASSED — exit code 0{:<21}║", "");
    } else {
        println!("║  ✗  ONE OR MORE ASSERTIONS FAILED — exit code 1{:<13}║", "");
    }
    println!("╚{}╝", sep);
    println!();

    all_passed
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
