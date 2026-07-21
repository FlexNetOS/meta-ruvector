// ARCHBP-012 — reproducible attractor-study report generator.
// Run: cargo run --example attractor_report

use atas_harness::attractor_study::{run_attractor_study, StudyConfig};

fn main() {
    let report = run_attractor_study(&StudyConfig::preregistered()).expect("study runs");
    println!("permits_downstream = {}", report.permits_downstream);
    println!("attractor_median_gain = {:.4}", report.attractor_median_gain);
    println!("stochastic_median_gain = {:.4}", report.stochastic_median_gain);
    for c in &report.criteria {
        println!("[{}] passed={} :: {}", c.name, c.passed, c.detail);
    }
    println!("verdict: {}", report.verdict);
    println!("--- per-seed results ---");
    for r in &report.results {
        println!(
            "{:<12} seed={} esn={:.4} pers={:.4} mean={:.4} gain={:+.4} esn_beats_mean={}",
            r.label, r.seed, r.esn_nrmse, r.persistence_nrmse, r.mean_nrmse,
            r.relative_gain_over_persistence, r.esn_beats_mean
        );
    }
}
