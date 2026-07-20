//! ARCHBP-011 verification-gate test suite (written FIRST — red phase).
//!
//! Binding gate (task row): "Red tests establish deterministic seeded runs,
//! leakage-free datasets, naive baselines, reservoir/readout separation,
//! ensemble outputs, calibrated uncertainty, and explicit non-authority
//! labeling for every timeline."
//!
//! Plus the ATAS output surface from the task goal: state diffs, resource
//! burn, complexity spikes, failure nodes — and a hard simulation budget
//! (blocked path: "uncontrolled simulation spend").
//!
//! IMPORTANT SCOPE RULE (blocked path: "forecast-as-proof"): no test in this
//! suite gates on forecast *skill*. Tests assert mechanics and verifiable
//! math only: determinism, leakage rejection, weight separation, exact
//! resource accounting, ridge-regression correctness, and the finite-sample
//! property of split-conformal calibration (which holds regardless of model
//! quality). Baseline/ablation NRMSE values are asserted *present and
//! computed*, never "good".

use atas_harness::atas::{run_studio, AtasError, SimulationBudget, StudioConfig};
use atas_harness::baseline::{mean_forecast, nrmse, persistence_forecast, targets};
use atas_harness::dataset::{ar1_noise, mackey_glass, DatasetContract, SeriesDataset};
use atas_harness::ensemble::{run_ensemble, EnsembleConfig};
use atas_harness::esn::{EsnConfig, EsnModel};
use atas_harness::linalg::ridge_solve;
use atas_harness::rng::SplitMix64;

fn small_cfg(seed: u64) -> EsnConfig {
    EsnConfig {
        reservoir_size: 80,
        spectral_radius: 0.9,
        leak_rate: 0.3,
        input_scale: 0.5,
        connectivity: 0.1,
        ridge_lambda: 1e-6,
        seed,
    }
}

fn studio_cfg(seed: u64, members: usize) -> StudioConfig {
    StudioConfig {
        ensemble: EnsembleConfig {
            base: small_cfg(seed),
            members,
        },
        budget: SimulationBudget::default(),
        nominal_coverage: 0.9,
        spike_window: 8,
        spike_ratio: 6.0,
        spike_floor: 1e-9,
        divergence_bound: 1e6,
    }
}

fn demo_contract(len: usize) -> DatasetContract {
    DatasetContract::chronological(len, 100, 0.6, 0.2).expect("demo contract must validate")
}

// ---------------------------------------------------------------------------
// Gate clause 1: deterministic seeded runs
// ---------------------------------------------------------------------------
#[test]
fn gate1_deterministic_seeded_runs() {
    // Dataset generation is seed-deterministic.
    let a = mackey_glass(400, 7);
    let b = mackey_glass(400, 7);
    assert_eq!(a.values, b.values, "same seed must give identical series");
    let c = mackey_glass(400, 8);
    assert_ne!(
        a.values, c.values,
        "different seed must give different series"
    );

    // Whole-studio runs are bitwise reproducible for a fixed seed.
    let ds = mackey_glass(1200, 42);
    let contract = demo_contract(ds.values.len());
    let cfg = studio_cfg(7, 4);
    let r1 = run_studio(&cfg, &ds, &contract).expect("studio run 1");
    let r2 = run_studio(&cfg, &ds, &contract).expect("studio run 2");
    assert_eq!(
        r1.forecast_fingerprint(),
        r2.forecast_fingerprint(),
        "fixed seed must reproduce the forecast fingerprint bit-for-bit"
    );
    assert_eq!(r1.timelines.len(), r2.timelines.len());
    for (t1, t2) in r1.timelines.iter().zip(r2.timelines.iter()) {
        assert_eq!(t1.steps.len(), t2.steps.len());
        for (s1, s2) in t1.steps.iter().zip(t2.steps.iter()) {
            assert_eq!(s1.forecast.to_bits(), s2.forecast.to_bits());
            assert_eq!(s1.state_diff.to_bits(), s2.state_diff.to_bits());
        }
    }

    // A different base seed must change the ensemble.
    let r3 = run_studio(&studio_cfg(8, 4), &ds, &contract).expect("studio run 3");
    assert_ne!(
        r1.forecast_fingerprint(),
        r3.forecast_fingerprint(),
        "different seed must change the forecast fingerprint"
    );
}

// ---------------------------------------------------------------------------
// Gate clause 2: leakage-free datasets
// ---------------------------------------------------------------------------
#[test]
fn gate2_leakage_free_dataset_contract() {
    let c = DatasetContract::chronological(1000, 50, 0.6, 0.2).expect("valid contract");
    c.validate().expect("chronological contract must validate");
    assert!(c.washout <= c.train_range.0, "washout precedes training");
    assert!(
        c.train_range.1 <= c.calib_range.0,
        "train strictly before calibration"
    );
    assert!(
        c.calib_range.1 <= c.test_range.0,
        "calibration strictly before test"
    );
    assert!(
        c.test_range.1 <= c.series_len - 1,
        "last test input must still have a future target inside the series"
    );

    // Train/test overlap is leakage and must be rejected.
    let overlap = DatasetContract {
        series_len: 1000,
        washout: 50,
        train_range: (50, 700),
        calib_range: (600, 800),
        test_range: (800, 999),
    };
    assert!(
        overlap.validate().is_err(),
        "overlapping ranges must be rejected"
    );

    // A test range whose last input has no future target must be rejected.
    let past_end = DatasetContract {
        series_len: 1000,
        washout: 50,
        train_range: (50, 600),
        calib_range: (600, 800),
        test_range: (800, 1000),
    };
    assert!(
        past_end.validate().is_err(),
        "range past len-1 must be rejected"
    );

    // Training inside the washout region must be rejected.
    let in_washout = DatasetContract {
        series_len: 1000,
        washout: 100,
        train_range: (50, 600),
        calib_range: (600, 800),
        test_range: (800, 999),
    };
    assert!(
        in_washout.validate().is_err(),
        "train inside washout must be rejected"
    );

    // Targets are strictly future values: y(t) = u(t+1).
    let ds = mackey_glass(300, 1);
    let y = targets(&ds.values, (10, 20));
    assert_eq!(y.len(), 10);
    for (i, yi) in y.iter().enumerate() {
        assert_eq!(
            yi.to_bits(),
            ds.values[10 + i + 1].to_bits(),
            "target at input index t must be exactly u(t+1)"
        );
    }
}

// ---------------------------------------------------------------------------
// Gate clause 3: naive baselines
// ---------------------------------------------------------------------------
#[test]
fn gate3_naive_baselines_computed_and_reported() {
    let ds = mackey_glass(1200, 42);
    let contract = demo_contract(ds.values.len());
    let test_len = contract.test_range.1 - contract.test_range.0;

    let p = persistence_forecast(&ds.values, contract.test_range);
    assert_eq!(p.len(), test_len);
    assert_eq!(
        p[0].to_bits(),
        ds.values[contract.test_range.0].to_bits(),
        "persistence forecast at t is exactly u(t)"
    );

    let m = mean_forecast(&ds.values, contract.train_range, contract.test_range);
    assert_eq!(m.len(), test_len);
    let train_slice = &ds.values[contract.train_range.0..contract.train_range.1];
    let expected_mean: f64 = train_slice.iter().sum::<f64>() / train_slice.len() as f64;
    assert!(
        (m[0] - expected_mean).abs() < 1e-12,
        "mean baseline uses the TRAIN mean only"
    );
    assert!(
        m.iter().all(|v| v.to_bits() == m[0].to_bits()),
        "mean baseline is constant"
    );

    let y = targets(&ds.values, contract.test_range);
    assert!(nrmse(&p, &y).is_finite() && nrmse(&p, &y) > 0.0);
    assert!(nrmse(&m, &y).is_finite() && nrmse(&m, &y) > 0.0);

    // The studio report must carry both naive baselines.
    let report = run_studio(&studio_cfg(3, 4), &ds, &contract).expect("studio run");
    for name in ["persistence", "train_mean"] {
        let row = report
            .baselines
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("baseline '{name}' missing from report"));
        assert!(
            row.nrmse.is_finite(),
            "baseline '{name}' NRMSE must be computed"
        );
    }
}

// ---------------------------------------------------------------------------
// Gate clause 4: reservoir/readout separation
// ---------------------------------------------------------------------------
#[test]
fn gate4_reservoir_readout_separation() {
    let ds = mackey_glass(1200, 42);
    let contract = demo_contract(ds.values.len());

    // Same seed -> identical fixed reservoir; different seed -> different.
    let m1 = EsnModel::new(small_cfg(3));
    let m2 = EsnModel::new(small_cfg(3));
    let m3 = EsnModel::new(small_cfg(4));
    assert_eq!(m1.reservoir.fingerprint(), m2.reservoir.fingerprint());
    assert_ne!(m1.reservoir.fingerprint(), m3.reservoir.fingerprint());

    // Training must touch ONLY the readout, never the reservoir.
    let mut model = EsnModel::new(small_cfg(3));
    let fp_before = model.reservoir.fingerprint();
    assert!(model.readout.is_none(), "untrained model has no readout");
    model.fit(&ds.values, &contract).expect("fit");
    assert_eq!(
        model.reservoir.fingerprint(),
        fp_before,
        "fit must not mutate reservoir weights (reservoir/readout separation)"
    );
    assert!(model.readout.is_some(), "fit must produce a readout");

    // Refitting on different data changes the readout but never the reservoir.
    let w1 = model.readout.as_ref().unwrap().weights.clone();
    let ds2 = mackey_glass(1200, 99);
    model.fit(&ds2.values, &contract).expect("refit");
    assert_eq!(model.reservoir.fingerprint(), fp_before);
    let w2 = &model.readout.as_ref().unwrap().weights;
    assert_ne!(&w1, w2, "different data must give a different readout");
}

// ---------------------------------------------------------------------------
// Gate clause 5: ensemble outputs (mean + variance reported)
// ---------------------------------------------------------------------------
#[test]
fn gate5_ensemble_outputs_mean_and_variance() {
    let ds = mackey_glass(1200, 42);
    let contract = demo_contract(ds.values.len());
    let test_len = contract.test_range.1 - contract.test_range.0;
    let cfg = EnsembleConfig {
        base: small_cfg(5),
        members: 6,
    };
    let ens = run_ensemble(&cfg, &ds.values, &contract, contract.test_range).expect("ensemble");
    assert_eq!(ens.member_preds.len(), 6, "one prediction track per member");
    for preds in &ens.member_preds {
        assert_eq!(preds.len(), test_len);
    }
    assert_eq!(ens.mean.len(), test_len);
    assert_eq!(ens.variance.len(), test_len);
    for (i, (&m, &v)) in ens.mean.iter().zip(ens.variance.iter()).enumerate() {
        assert!(v >= 0.0, "variance must be non-negative at step {i}");
        let recomputed: f64 =
            ens.member_preds.iter().map(|p| p[i]).sum::<f64>() / ens.member_preds.len() as f64;
        assert!(
            (m - recomputed).abs() <= 1e-12,
            "mean must be the member average at step {i}"
        );
    }
    assert!(
        ens.variance.iter().cloned().fold(0.0_f64, f64::max) > 0.0,
        "distinct seeds must produce ensemble spread (variance > 0 somewhere)"
    );
}

// ---------------------------------------------------------------------------
// Gate clause 6: calibrated uncertainty
// ---------------------------------------------------------------------------
#[test]
fn gate6_calibrated_uncertainty_split_conformal() {
    // Split-conformal calibration has a *finite-sample coverage property* on
    // near-exchangeable residuals that holds regardless of forecast skill —
    // so asserting coverage here verifies the calibration MATH, and does not
    // promote forecast quality to a gate.
    let ds = ar1_noise(3000, 0.8, 0.1, 9);
    let contract = DatasetContract::chronological(3000, 100, 0.5, 0.25).expect("contract");
    let report = run_studio(&studio_cfg(11, 6), &ds, &contract).expect("studio");
    let cal = &report.calibration;
    assert!((cal.nominal_coverage - 0.9).abs() < 1e-12);
    assert!(cal.conformal_width.is_finite() && cal.conformal_width > 0.0);
    assert!(cal.calib_points > 0 && cal.test_points > 0);
    assert!((0.0..=1.0).contains(&cal.empirical_coverage));
    assert!(
        cal.empirical_coverage >= 0.85,
        "split-conformal width from the calibration split must transfer to \
         near-nominal empirical coverage on stationary AR(1) data; got {}",
        cal.empirical_coverage
    );
}

// ---------------------------------------------------------------------------
// Gate clause 7: explicit non-authority labeling for every timeline
// ---------------------------------------------------------------------------
#[test]
fn gate7_non_authority_labeling_on_every_timeline() {
    let ds = mackey_glass(1200, 42);
    let contract = demo_contract(ds.values.len());
    let report = run_studio(&studio_cfg(13, 3), &ds, &contract).expect("studio");

    assert_eq!(report.schema, "atas.future-timeline.v0");
    assert_eq!(report.label.authority, "research-only");
    assert!(report.label.non_authoritative);
    assert!(
        report.label.statement.to_lowercase().contains("never"),
        "label statement must forbid authoritative use"
    );
    for tl in &report.timelines {
        assert_eq!(
            tl.label.authority, "research-only",
            "member {} unlabeled",
            tl.member
        );
        assert!(
            tl.label.non_authoritative,
            "member {} not marked non-authoritative",
            tl.member
        );
    }

    // The serialized schema must carry the label on the report AND on every
    // single timeline — downstream consumers can never see an unlabeled one.
    let json = report.to_json();
    let needle = "\"authority\":\"research-only\"";
    let count = json.matches(needle).count();
    assert!(
        count >= report.timelines.len() + 1,
        "expected >= {} research-only labels in JSON, found {count}",
        report.timelines.len() + 1
    );
    assert!(json.contains("\"non_authoritative\":true"));
    assert!(json.contains("\"schema\":\"atas.future-timeline.v0\""));
}

// ---------------------------------------------------------------------------
// ATAS output: per-step state diffs
// ---------------------------------------------------------------------------
#[test]
fn gate8_state_diffs_emitted_per_timeline_step() {
    let ds = mackey_glass(1200, 42);
    let contract = demo_contract(ds.values.len());
    let test_len = contract.test_range.1 - contract.test_range.0;
    let report = run_studio(&studio_cfg(17, 3), &ds, &contract).expect("studio");
    for tl in &report.timelines {
        assert_eq!(tl.steps.len(), test_len, "one step record per test point");
        let mut total = 0.0;
        for (i, s) in tl.steps.iter().enumerate() {
            assert_eq!(
                s.step,
                contract.test_range.0 + i,
                "steps carry absolute series index"
            );
            assert!(s.state_diff.is_finite() && s.state_diff >= 0.0);
            total += s.state_diff;
        }
        assert!(total > 0.0, "a driven reservoir must actually move");
    }
}

// ---------------------------------------------------------------------------
// ATAS output: resource burn (exact accounting, not vibes)
// ---------------------------------------------------------------------------
#[test]
fn gate9_resource_burn_exact_accounting() {
    let ds = mackey_glass(1200, 42);
    let contract = demo_contract(ds.values.len());
    let members = 4usize;
    let report = run_studio(&studio_cfg(19, members), &ds, &contract).expect("studio");
    let burn = &report.resource_burn;

    // Per member: fit sweeps t in [0, train_end) and the single prediction
    // pass sweeps t in [0, test_end). This formula is the spec.
    let expected = members as u64 * (contract.train_range.1 + contract.test_range.1) as u64;
    assert_eq!(
        burn.state_updates, expected,
        "state updates must be exactly accounted"
    );
    assert_eq!(burn.members_run, members);
    assert!(burn.flops_per_update > 0);
    assert_eq!(
        burn.flop_estimate,
        burn.state_updates * burn.flops_per_update
    );
    assert!(burn.wall_time_ms >= 0.0);
}

// ---------------------------------------------------------------------------
// ATAS output: complexity spikes
// ---------------------------------------------------------------------------
#[test]
fn gate10_complexity_spikes_detected() {
    let n = 800;
    let contract = DatasetContract::chronological(n, 50, 0.6, 0.2).expect("contract");

    // A constant input settles the reservoir: no complexity spikes.
    let flat = SeriesDataset::from_values("flat", 0, vec![0.5; n]);
    let r_flat = run_studio(&studio_cfg(23, 3), &flat, &contract).expect("flat studio");
    for tl in &r_flat.timelines {
        assert!(
            tl.complexity_spikes.is_empty(),
            "constant input must not produce complexity spikes; member {} got {:?}",
            tl.member,
            tl.complexity_spikes
        );
    }

    // A step change inside the test range must be flagged near the jump.
    let jump = contract.test_range.0 + (contract.test_range.1 - contract.test_range.0) / 2;
    let mut vals = vec![0.5; n];
    for v in vals.iter_mut().skip(jump) {
        *v = 5.0;
    }
    let stepped = SeriesDataset::from_values("stepped", 0, vals);
    let r_step = run_studio(&studio_cfg(23, 3), &stepped, &contract).expect("step studio");
    for tl in &r_step.timelines {
        assert!(
            !tl.complexity_spikes.is_empty(),
            "member {} missed the step change",
            tl.member
        );
        assert!(
            tl.complexity_spikes
                .iter()
                .any(|&s| s >= jump.saturating_sub(2) && s <= jump + 8),
            "member {} spikes {:?} not near jump index {jump}",
            tl.member,
            tl.complexity_spikes
        );
    }
}

// ---------------------------------------------------------------------------
// ATAS output: failure nodes
// ---------------------------------------------------------------------------
#[test]
fn gate11_failure_nodes_flagged() {
    let n = 800;
    let contract = DatasetContract::chronological(n, 50, 0.6, 0.2).expect("contract");
    let clean = mackey_glass(n, 21);

    // Clean run with sane bounds: no failure nodes.
    let r_clean = run_studio(&studio_cfg(29, 3), &clean, &contract).expect("clean studio");
    for tl in &r_clean.timelines {
        assert!(
            tl.failure_nodes.is_empty(),
            "clean run must have no failure nodes"
        );
    }

    // Non-finite input inside the test range must be flagged where it lands.
    let poison = contract.test_range.0 + 10;
    let mut vals = clean.values.clone();
    vals[poison] = f64::NAN;
    let poisoned = SeriesDataset::from_values("poisoned", 21, vals);
    let r_bad = run_studio(&studio_cfg(29, 3), &poisoned, &contract).expect("poisoned studio");
    for tl in &r_bad.timelines {
        assert!(
            !tl.failure_nodes.is_empty(),
            "member {} missed the poison",
            tl.member
        );
        let first = &tl.failure_nodes[0];
        assert_eq!(
            first.step, poison,
            "failure must be flagged at the poisoned step"
        );
        assert!(
            first.reason.contains("non-finite"),
            "reason should name the non-finite state; got '{}'",
            first.reason
        );
    }

    // A divergence bound must convert large state motion into failure nodes.
    let mut tight = studio_cfg(29, 3);
    tight.divergence_bound = 1e-12;
    let r_div = run_studio(&tight, &clean, &contract).expect("divergence studio");
    for tl in &r_div.timelines {
        assert!(
            !tl.failure_nodes.is_empty(),
            "tight divergence bound must flag nodes"
        );
        assert!(
            tl.failure_nodes
                .iter()
                .any(|f| f.reason.contains("divergence")),
            "at least one node must cite divergence"
        );
    }
}

// ---------------------------------------------------------------------------
// Bounded studio: simulation budget is enforced BEFORE spend
// ---------------------------------------------------------------------------
#[test]
fn gate12_simulation_budget_enforced() {
    let ds = mackey_glass(1200, 42);
    let contract = demo_contract(ds.values.len());

    let mut over_members = studio_cfg(31, 8);
    over_members.budget = SimulationBudget {
        max_members: 4,
        max_steps: 100_000,
        max_state_updates: u64::MAX,
    };
    match run_studio(&over_members, &ds, &contract) {
        Err(AtasError::BudgetExceeded {
            what,
            limit,
            requested,
        }) => {
            assert!(what.contains("members"), "wrong budget clause: {what}");
            assert_eq!(limit, 4);
            assert_eq!(requested, 8);
        }
        other => panic!("expected BudgetExceeded for members, got {other:?}"),
    }

    let mut over_steps = studio_cfg(31, 2);
    over_steps.budget = SimulationBudget {
        max_members: 32,
        max_steps: 10,
        max_state_updates: u64::MAX,
    };
    assert!(
        matches!(
            run_studio(&over_steps, &ds, &contract),
            Err(AtasError::BudgetExceeded { ref what, .. }) if what.contains("steps")
        ),
        "timeline longer than max_steps must be refused"
    );

    let mut over_updates = studio_cfg(31, 2);
    over_updates.budget = SimulationBudget {
        max_members: 32,
        max_steps: 100_000,
        max_state_updates: 10,
    };
    assert!(
        matches!(
            run_studio(&over_updates, &ds, &contract),
            Err(AtasError::BudgetExceeded { ref what, .. }) if what.contains("state_updates")
        ),
        "runs costing more than max_state_updates must be refused pre-flight"
    );
}

// ---------------------------------------------------------------------------
// Verifiable math: the ridge readout solver recovers a known linear system
// ---------------------------------------------------------------------------
#[test]
fn gate13_ridge_readout_solves_known_linear_system() {
    let mut rng = SplitMix64::new(1);
    let w_true = [0.5, -1.25, 2.0, 0.0, 3.5, -0.75, 1.0, -2.0];
    let mut rows = Vec::with_capacity(200);
    let mut y = Vec::with_capacity(200);
    for _ in 0..200 {
        let row: Vec<f64> = (0..w_true.len())
            .map(|_| rng.next_uniform(-1.0, 1.0))
            .collect();
        y.push(
            row.iter()
                .zip(w_true.iter())
                .map(|(a, b)| a * b)
                .sum::<f64>(),
        );
        rows.push(row);
    }
    let w = ridge_solve(&rows, &y, 1e-9).expect("ridge solve");
    assert_eq!(w.len(), w_true.len());
    for (i, (got, want)) in w.iter().zip(w_true.iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-5,
            "ridge weight {i}: got {got}, want {want}"
        );
    }
}

// ---------------------------------------------------------------------------
// Ablation table: reported comparisons, never gates
// ---------------------------------------------------------------------------
#[test]
fn gate14_ablation_table_reported() {
    let ds = mackey_glass(1200, 42);
    let contract = demo_contract(ds.values.len());
    let report = run_studio(&studio_cfg(37, 4), &ds, &contract).expect("studio");
    for name in ["esn_ensemble", "readout_only", "persistence", "train_mean"] {
        let row = report
            .ablations
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("ablation '{name}' missing from report"));
        assert!(
            row.nrmse.is_finite(),
            "ablation '{name}' NRMSE must be computed"
        );
    }
    assert!(report.to_json().contains("\"ablations\""));
}
