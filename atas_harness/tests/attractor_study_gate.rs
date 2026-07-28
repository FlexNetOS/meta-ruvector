// ARCHBP-012 — empirical attractor-hypothesis gate.
//
// Pre-registered: does attractor-like structure in the declared state series
// yield reproducible out-of-sample forecast gains over simple baselines beyond
// what linear/stochastic structure explains? The study compares an ESN against
// persistence and mean baselines on an attractor-bearing series (mackey-glass)
// and a stochastic control (ar1), across seeds, with a shuffle ablation. These
// tests assert the study machinery is deterministic, leakage-free, and reports
// every seed — and they assert the HONEST verdict this harness actually yields.

use atas_harness::attractor_study::{run_attractor_study, StudyConfig};
use atas_harness::dataset::DatasetContract;

const ATTRACTOR: &str = "mackey-glass";
const STOCHASTIC: &str = "ar1";

fn report() -> atas_harness::attractor_study::StudyReport {
    run_attractor_study(&StudyConfig::preregistered()).expect("study runs")
}

#[test]
fn study_is_reproducible_bit_for_bit() {
    let a = report();
    let b = report();
    assert_eq!(a.results.len(), b.results.len());
    for (x, y) in a.results.iter().zip(b.results.iter()) {
        assert_eq!(x.label, y.label);
        assert_eq!(x.seed, y.seed);
        assert_eq!(x.esn_nrmse.to_bits(), y.esn_nrmse.to_bits(), "esn nrmse must be deterministic");
        assert_eq!(x.persistence_nrmse.to_bits(), y.persistence_nrmse.to_bits());
    }
    assert_eq!(a.permits_downstream, b.permits_downstream);
}

#[test]
fn every_series_and_seed_is_reported_no_cherry_picking() {
    let cfg = StudyConfig::preregistered();
    let r = report();
    assert_eq!(r.results.len(), cfg.seeds.len() * 2, "both series x all seeds must appear");
    assert!(r.results.iter().any(|s| s.label == ATTRACTOR));
    assert!(r.results.iter().any(|s| s.label == STOCHASTIC));
    for seed in &cfg.seeds {
        assert!(r.results.iter().any(|s| s.label == ATTRACTOR && s.seed == *seed));
        assert!(r.results.iter().any(|s| s.label == STOCHASTIC && s.seed == *seed));
    }
}

#[test]
fn dataset_contract_is_leakage_free_and_chronological() {
    let cfg = StudyConfig::preregistered();
    let contract = DatasetContract::chronological(
        cfg.series_len,
        cfg.washout,
        cfg.train_fraction,
        cfg.calibration_fraction,
    )
    .expect("valid contract");
    contract.validate().expect("contract validates");
    assert!(contract.train_range.1 <= contract.calib_range.0);
    assert!(contract.calib_range.1 <= contract.test_range.0, "test must follow train — no leakage");
}

#[test]
fn esn_beats_the_mean_baseline_everywhere() {
    // Sanity: the reservoir learns real structure — it must beat a constant-mean
    // forecast on every series and seed, or the negative result below would be
    // vacuous (a model that learns nothing).
    let r = report();
    for s in &r.results {
        assert!(s.esn_beats_mean, "{} seed {} ESN must beat mean", s.label, s.seed);
        assert!(s.esn_nrmse < s.mean_nrmse);
    }
}

#[test]
fn preregistered_criteria_are_all_evaluated() {
    let r = report();
    assert_eq!(r.criteria.len(), 3, "A1 skill-over-naive, A2 attractor-specific, A3 temporal-dependent");
    assert!(r.criteria.iter().any(|c| c.name.contains("skill-over-naive")));
    assert!(r.criteria.iter().any(|c| c.name.contains("attractor-specific")));
    assert!(r.criteria.iter().any(|c| c.name.contains("temporal")));
}

#[test]
fn attractor_hypothesis_fails_out_of_sample_skill_over_naive() {
    // Honest negative: on the attractor-bearing series the ESN does NOT beat the
    // naive persistence baseline out-of-sample by the pre-registered margin.
    let r = report();
    let a1 = r.criteria.iter().find(|c| c.name.contains("skill-over-naive")).unwrap();
    assert!(!a1.passed, "attractor series shows no reproducible skill over persistence");
}

#[test]
fn measured_gain_is_not_attractor_specific() {
    // The ESN's out-of-sample advantage over persistence is not larger on the
    // attractor series than on the stochastic control — the advantage is linear/
    // stochastic structure, not attractor exploitation.
    let r = report();
    assert!(
        r.attractor_median_gain <= r.stochastic_median_gain,
        "attractor gain {:.4} must not exceed stochastic gain {:.4}",
        r.attractor_median_gain,
        r.stochastic_median_gain
    );
}

#[test]
fn downstream_use_is_not_permitted() {
    // Only reproducible out-of-sample gains permit downstream use; there are none.
    let r = report();
    assert!(!r.permits_downstream, "verdict: {}", r.verdict);
    assert!(!r.verdict.is_empty());
}
