use std::fs;
use std::path::Path;

use pulse::audit::walk_typed_source_files;
use pulse::buildmeta::{declared_test_roots, is_under_test_root};

fn mkfile(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

#[test]
fn pyproject_testpaths_become_declared_roots() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("pyproject.toml"), "[tool.pytest.ini_options]\ntestpaths = [\"suite\", \"e2e\"]\n")
        .unwrap();
    fs::create_dir_all(dir.path().join("suite")).unwrap();
    fs::create_dir_all(dir.path().join("e2e")).unwrap();
    let roots = declared_test_roots(dir.path());
    assert!(roots.contains(&dir.path().join("suite")), "{roots:?}");
    assert!(roots.contains(&dir.path().join("e2e")), "{roots:?}");
}

#[test]
fn ini_multiline_testpaths_become_roots() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("setup.cfg"), "[tool:pytest]\ntestpaths =\n    suite\n    integ\naddopts = -q\n")
        .unwrap();
    fs::create_dir_all(dir.path().join("suite")).unwrap();
    fs::create_dir_all(dir.path().join("integ")).unwrap();
    let roots = declared_test_roots(dir.path());
    assert!(roots.contains(&dir.path().join("suite")), "{roots:?}");
    assert!(roots.contains(&dir.path().join("integ")), "{roots:?}");
    assert!(!is_under_test_root(&dir.path().join("src/main.py"), &roots), "addopts is not a test root");
}

#[test]
fn nonexistent_testpath_is_not_a_root() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("pytest.ini"), "[pytest]\ntestpaths = ghost\n").unwrap();
    assert!(declared_test_roots(dir.path()).is_empty(), "a configured path that is not a directory is ignored");
}

#[test]
fn audit_walk_excludes_non_conventional_declared_test_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("pyproject.toml"), "[tool.pytest.ini_options]\ntestpaths = [\"suite\"]\n").unwrap();
    mkfile(dir.path(), "suite/check_thing.py", "x = 1\n");
    mkfile(dir.path(), "src/app.py", "y = 2\n");

    let without_tests = walk_typed_source_files(dir.path(), false);
    assert!(without_tests.iter().any(|(p, _)| p.ends_with("src/app.py")));
    assert!(
        !without_tests.iter().any(|(p, _)| p.ends_with("suite/check_thing.py")),
        "a file under a declared pytest testpath is treated as test code: {without_tests:?}"
    );

    let with_tests = walk_typed_source_files(dir.path(), true);
    assert!(
        with_tests.iter().any(|(p, _)| p.ends_with("suite/check_thing.py")),
        "include_tests keeps the declared test dir"
    );
}
