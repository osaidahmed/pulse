use pulse::history::jit_risk::{calib_path, percentiles};
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
