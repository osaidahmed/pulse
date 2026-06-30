use pulse_config::{resolve_history_thresholds, HistoryCliOverrides, PulseConfig};
use pulse_history::jit_risk::{
    calib_path, hook_advisory, percentiles, read_calibration, write_calibration, JitCalibration, Quintiles,
};
use pulse_history::thresholds::HistoryThresholds;
use pulse_history::{calibrate, HistoryOpts};

use crate::history_common::{build_repo, commit_file_dated, CommitSpec};

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
    let calib =
        JitCalibration { lt: Some(Quintiles { p20: 10.0, p50: 50.0, p80: 100.0 }), age_days: None, entropy_bits: None };
    write_calibration(repo.path(), &calib).expect("write calibration");
    let file = repo.path().join("a.py");

    let small = hook_advisory("x = 1\n", &file).expect("small file flagged");
    assert!(small.contains("below the repo's 20th percentile"), "got: {small}");
    let large = hook_advisory(&"y = 1\n".repeat(200), &file).expect("large file flagged");
    assert!(large.contains("above the repo's 80th percentile"), "got: {large}");
    assert!(hook_advisory(&"z = 1\n".repeat(50), &file).is_none(), "mid-range file must not flag");
}

#[test]
fn hook_advisory_flags_a_recently_committed_file_against_the_age_band() {
    let analytics = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_ANALYTICS_DIR", analytics.path());
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let calib = JitCalibration {
        lt: None,
        age_days: Some(Quintiles { p20: 3650.0, p50: 7300.0, p80: 10950.0 }),
        entropy_bits: None,
    };
    write_calibration(repo.path(), &calib).expect("write calibration");
    let file = repo.path().join("a.py");

    let msg = hook_advisory("x = 1\n", &file).expect("just-committed file flagged by the age band");
    assert!(msg.contains("file age is below the repo's 20th percentile"), "got: {msg}");
}

#[test]
fn hook_advisory_skips_the_age_band_for_an_untracked_file() {
    let analytics = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_ANALYTICS_DIR", analytics.path());
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let calib = JitCalibration {
        lt: None,
        age_days: Some(Quintiles { p20: 3650.0, p50: 7300.0, p80: 10950.0 }),
        entropy_bits: None,
    };
    write_calibration(repo.path(), &calib).expect("write calibration");
    let untracked = repo.path().join("untracked.py");
    std::fs::write(&untracked, "x = 1\n").unwrap();

    assert!(hook_advisory("x = 1\n", &untracked).is_none(), "an untracked file has no commit age to band");
}

#[test]
fn hook_advisory_fails_open_when_the_last_commit_is_future_dated() {
    let analytics = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_ANALYTICS_DIR", analytics.path());
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    commit_file_dated(repo.path(), "a.py", "x = 2\n", 4_000_000_000);
    let calib = JitCalibration {
        lt: None,
        age_days: Some(Quintiles { p20: 3650.0, p50: 7300.0, p80: 10950.0 }),
        entropy_bits: None,
    };
    write_calibration(repo.path(), &calib).expect("write calibration");
    let file = repo.path().join("a.py");

    assert!(
        hook_advisory("x = 2\n", &file).is_none(),
        "a future-dated last commit (now < commit ts) must fail open, not clamp to age 0 and flag"
    );
}

#[test]
fn hook_advisory_bands_age_for_a_file_in_a_subdirectory() {
    let analytics = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_ANALYTICS_DIR", analytics.path());
    let repo = build_repo(&[CommitSpec {
        author: "a <a@x>",
        message: "init",
        writes: &[("pkg/a.py", "x = 1\n")],
        deletes: &[],
    }]);
    let calib = JitCalibration {
        lt: None,
        age_days: Some(Quintiles { p20: 3650.0, p50: 7300.0, p80: 10950.0 }),
        entropy_bits: None,
    };
    write_calibration(repo.path(), &calib).expect("write calibration");
    let file = repo.path().join("pkg").join("a.py");

    let msg =
        hook_advisory("x = 1\n", &file).expect("a subdir file resolves its commit age and the toplevel-keyed calib");
    assert!(msg.contains("file age is below the repo's 20th percentile"), "got: {msg}");
}

#[test]
fn hook_advisory_flags_a_diffuse_working_tree_change() {
    let analytics = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_ANALYTICS_DIR", analytics.path());
    let repo = build_repo(&[CommitSpec {
        author: "a <a@x>",
        message: "init",
        writes: &[("a.py", "x = 1\n"), ("b.py", "x = 1\n"), ("c.py", "x = 1\n"), ("d.py", "x = 1\n")],
        deletes: &[],
    }]);
    for name in ["a.py", "b.py", "c.py", "d.py"] {
        std::fs::write(repo.path().join(name), "x = 1\ny = 2\n").unwrap();
    }
    let calib = JitCalibration { lt: None, age_days: None, entropy_bits: Some(1.5) };
    write_calibration(repo.path(), &calib).expect("write calibration");
    let file = repo.path().join("a.py");

    let msg = hook_advisory("x = 1\ny = 2\n", &file).expect("a change spread evenly across four files is diffuse");
    assert!(msg.contains("change entropy"), "got: {msg}");
}

#[test]
fn hook_advisory_skips_entropy_for_a_single_file_change() {
    let analytics = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_ANALYTICS_DIR", analytics.path());
    let repo = build_repo(&[CommitSpec {
        author: "a <a@x>",
        message: "init",
        writes: &[("a.py", "x = 1\n"), ("b.py", "x = 1\n")],
        deletes: &[],
    }]);
    std::fs::write(repo.path().join("a.py"), "x = 1\ny = 2\n").unwrap();
    let calib = JitCalibration { lt: None, age_days: None, entropy_bits: Some(0.5) };
    write_calibration(repo.path(), &calib).expect("write calibration");
    let file = repo.path().join("a.py");

    assert!(
        hook_advisory("x = 1\ny = 2\n", &file).is_none(),
        "a single-file working-tree change has no diffusion entropy, even below a tiny threshold"
    );
}

#[test]
fn read_calibration_treats_a_pre_entropy_file_as_entropy_off() {
    let analytics = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_ANALYTICS_DIR", analytics.path());
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let path = calib_path(repo.path());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, r#"{"lt":{"p20":1.0,"p50":2.0,"p80":3.0},"age_days":null}"#).unwrap();

    let calib = read_calibration(repo.path()).expect("a pre-2c calibration file still deserializes");
    assert!(calib.entropy_bits.is_none(), "a missing entropy_bits key must read back as None (entropy off)");
    assert!(calib.lt.is_some(), "the existing lt quintiles must survive the upgrade");
}

#[test]
fn config_use_entropy_false_omits_entropy_from_calibration() {
    let cfg: PulseConfig = toml::from_str("[history.jit]\nuse_entropy = false\n").expect("parse config");
    let t = resolve_history_thresholds(Some(&cfg), HistoryCliOverrides::default());
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let opts = HistoryOpts { root: repo.path().to_path_buf(), include_tests: true, since: None, max_commits: None };

    let calib = calibrate(&opts, &t, 4_000_000_000).expect("calibration");
    assert!(calib.entropy_bits.is_none(), "use_entropy=false must omit the entropy threshold from the calibration");
    assert!(calib.lt.is_some(), "lt stays calibrated when only entropy is disabled");
}
