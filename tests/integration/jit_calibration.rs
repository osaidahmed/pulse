use pulse::history::jit_risk::{calib_path, hook_advisory, percentiles, write_calibration, JitCalibration, Quintiles};
use pulse::history::thresholds::HistoryThresholds;
use pulse::history::{calibrate, HistoryOpts};

use crate::history_common::{build_repo, CommitSpec};

#[test]
fn percentiles_use_nearest_rank() {
    let q = percentiles(vec![10.0, 20.0, 30.0, 40.0, 50.0]).expect("five values");
    assert!((q.p20 - 10.0).abs() < 1e-9);
    assert!((q.p50 - 30.0).abs() < 1e-9);
    assert!((q.p80 - 40.0).abs() < 1e-9);
}

#[test]
fn percentiles_none_for_empty_input() {
    assert!(percentiles(Vec::new()).is_none());
}

#[test]
fn calibration_yields_ordered_lt_and_age_quintiles_for_a_repo() {
    let repo = build_repo(&[CommitSpec {
        author: "a <a@x>",
        message: "init",
        writes: &[("a.py", "x = 1\n"), ("b.py", "x = 1\ny = 2\nz = 3\n"), ("c.py", "def f():\n    return 1\n")],
        deletes: &[],
    }]);
    let opts = HistoryOpts { root: repo.path().to_path_buf(), include_tests: true, since: None, max_commits: None };
    let now = 4_000_000_000;
    let calib = calibrate(&opts, &HistoryThresholds::default(), now).expect("calibration");

    let lt = calib.lt.expect("LT quintiles for three source files");
    assert!(lt.p20 <= lt.p50 && lt.p50 <= lt.p80, "LT quintiles must be ordered: {lt:?}");
    let age = calib.age_days.expect("AGE quintiles for committed files");
    assert!(age.p20 >= 0.0 && age.p20 <= age.p50 && age.p50 <= age.p80, "AGE quintiles must be ordered: {age:?}");
}

#[test]
fn calib_path_is_keyed_by_git_toplevel_not_the_passed_subdir() {
    let repo = build_repo(&[CommitSpec {
        author: "a <a@x>",
        message: "init",
        writes: &[("pkg/a.py", "x = 1\n")],
        deletes: &[],
    }]);
    assert_eq!(
        calib_path(repo.path()),
        calib_path(&repo.path().join("pkg")),
        "calibration must key by the git toplevel so the hook resolves the same path from any file in the repo"
    );
}

#[test]
fn hook_advisory_flags_smallest_and_largest_quintile_files() {
    let analytics = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_ANALYTICS_DIR", analytics.path());
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let calib = JitCalibration { lt: Some(Quintiles { p20: 10.0, p50: 50.0, p80: 100.0 }), age_days: None };
    write_calibration(repo.path(), &calib).expect("write calibration");
    let file = repo.path().join("a.py");

    let small = hook_advisory("x = 1\n", &file).expect("small file flagged");
    assert!(small.contains("below the repo's 20th percentile"), "got: {small}");
    let large = hook_advisory(&"y = 1\n".repeat(200), &file).expect("large file flagged");
    assert!(large.contains("above the repo's 80th percentile"), "got: {large}");
    assert!(hook_advisory(&"z = 1\n".repeat(50), &file).is_none(), "mid-range file must not flag");
}
