use pulse::config::{self, PulseConfig};
use pulse::parse::Language;
use pulse::smells::{self, Smell};
use pulse::thresholds::Thresholds;

use crate::common::t;

// ── TOML parsing ──

#[test]
fn empty_config_parses_to_defaults() {
    let cfg: PulseConfig = toml::from_str("").unwrap();
    let resolved = config::resolve_thresholds(Some(&cfg), Language::Python);
    assert_eq!(resolved, Thresholds::default());
}

#[test]
fn minimal_threshold_override() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds]
        arg_max = 10
        ",
    )
    .unwrap();
    let resolved = config::resolve_thresholds(Some(&cfg), Language::Python);
    assert_eq!(resolved.function.arg_max, 10);
    assert_eq!(resolved.function.cc_warning, t().function.cc_warning);
}

#[test]
fn package_metrics_thresholds_are_overridable() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds.package_metrics]
        unstable_dep_strength = 0.5
        god_component_loc_percentile = 0.95
        max_arch_findings_reported = 10
        ",
    )
    .unwrap();
    let resolved = config::resolve_base_thresholds(Some(&cfg));
    let pm = resolved.audit.package_metrics;
    assert!((pm.unstable_dep_strength - 0.5).abs() < 1e-9);
    assert!((pm.god_component_loc_percentile - 0.95).abs() < 1e-9);
    assert_eq!(pm.max_arch_findings_reported, 10);
    let default_pm = t().audit.package_metrics;
    assert!((pm.hublike_imbalance_ratio - default_pm.hublike_imbalance_ratio).abs() < 1e-9);
    assert_eq!(pm.pagerank, default_pm.pagerank, "algorithm internals keep defaults");
}

#[test]
fn unknown_package_metrics_key_is_rejected() {
    let result = toml::from_str::<PulseConfig>(
        r"
        [thresholds.package_metrics]
        bogus_knob = 5
        ",
    );
    assert!(result.is_err(), "unknown package-metrics keys must be rejected");
}

#[test]
fn audit_feature_thresholds_are_overridable() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds.taint]
        max_depth = 32
        [thresholds.clone_cluster]
        min_loc = 10
        [thresholds.naturalness]
        enabled = true
        zscore_cutoff = 4.0
        ",
    )
    .unwrap();
    let r = config::resolve_base_thresholds(Some(&cfg)).audit;
    assert_eq!(r.taint.max_depth, 32);
    assert_eq!(r.clone_cluster.min_loc, 10);
    assert!(r.naturalness.enabled);
    assert!((r.naturalness.zscore_cutoff - 4.0).abs() < 1e-9);
    let d = t().audit;
    assert_eq!(r.taint.visit_cap, d.taint.visit_cap, "untouched taint field keeps default");
    assert_eq!(
        r.clone_cluster.max_sim_threshold, d.clone_cluster.max_sim_threshold,
        "untouched clone field keeps default"
    );
}

#[test]
fn unknown_taint_key_is_rejected() {
    let result = toml::from_str::<PulseConfig>(
        r"
        [thresholds.taint]
        bogus = 1
        ",
    );
    assert!(result.is_err(), "unknown taint keys must be rejected");
}

#[test]
fn disable_section_parses() {
    let cfg: PulseConfig = toml::from_str(
        r#"
        [disable]
        smells = ["excess_arguments", "god_method"]
        "#,
    )
    .unwrap();
    let disabled = config::resolve_disabled(Some(&cfg));
    assert!(disabled.contains(&Smell::ExcessArguments));
    assert!(disabled.contains(&Smell::GodMethod));
    assert_eq!(disabled.len(), 2);
}

#[test]
fn language_override_parses() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds]
        arg_max = 8

        [languages.go]
        arg_max = 12
        ",
    )
    .unwrap();
    let go_thresholds = config::resolve_thresholds(Some(&cfg), Language::Go);
    let py_thresholds = config::resolve_thresholds(Some(&cfg), Language::Python);
    assert_eq!(go_thresholds.function.arg_max, 12);
    assert_eq!(py_thresholds.function.arg_max, 8);
}

#[test]
fn full_config_parses() {
    let cfg: PulseConfig = toml::from_str(
        r#"
        [thresholds]
        cc_warning = 12
        fn_loc_warning = 80
        arg_max = 8

        [disable]
        smells = ["primitive_obsession"]

        [languages.java]
        fn_loc_warning = 100
        "#,
    )
    .unwrap();
    let java = config::resolve_thresholds(Some(&cfg), Language::Java);
    assert_eq!(java.function.cc_warning, 12);
    assert_eq!(java.function.fn_loc_warning, 100);
    assert_eq!(java.function.arg_max, 8);
}

#[test]
fn unknown_top_level_key_errors() {
    let result = toml::from_str::<PulseConfig>(
        r"
        [bogus]
        foo = 1
        ",
    );
    assert!(result.is_err());
}

// ── Merge logic ──

#[test]
fn no_config_returns_defaults() {
    let resolved = config::resolve_thresholds(None, Language::Rust);
    assert_eq!(resolved, Thresholds::default());
}

#[test]
fn override_specific_fields_rest_default() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds]
        cc_warning = 15
        file_loc_warning = 600
        ",
    )
    .unwrap();
    let resolved = config::resolve_thresholds(Some(&cfg), Language::Rust);
    assert_eq!(resolved.function.cc_warning, 15);
    assert_eq!(resolved.module.file_loc_warning, 600);
    assert_eq!(resolved.function.cc_alert, t().function.cc_alert);
    assert_eq!(resolved.function.fn_loc_warning, t().function.fn_loc_warning);
    assert_eq!(resolved.function.arg_max, t().function.arg_max);
}

#[test]
fn language_override_further_overrides_base() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds]
        arg_max = 8
        cc_warning = 12

        [languages.go]
        arg_max = 12
        ",
    )
    .unwrap();
    let go = config::resolve_thresholds(Some(&cfg), Language::Go);
    assert_eq!(go.function.arg_max, 12);
    assert_eq!(go.function.cc_warning, 12);
}

#[test]
fn language_not_in_config_uses_base() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds]
        arg_max = 8

        [languages.java]
        arg_max = 10
        ",
    )
    .unwrap();
    let rust = config::resolve_thresholds(Some(&cfg), Language::Rust);
    assert_eq!(rust.function.arg_max, 8);
}

#[test]
fn multiple_language_sections_independent() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [languages.go]
        arg_max = 7

        [languages.java]
        arg_max = 10
        fn_loc_warning = 120
        ",
    )
    .unwrap();
    let go = config::resolve_thresholds(Some(&cfg), Language::Go);
    let java = config::resolve_thresholds(Some(&cfg), Language::Java);
    assert_eq!(go.function.arg_max, 7);
    assert_eq!(go.function.fn_loc_warning, t().function.fn_loc_warning);
    assert_eq!(java.function.arg_max, 10);
    assert_eq!(java.function.fn_loc_warning, 120);
}

#[test]
fn resolve_base_thresholds_ignores_language() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds]
        arg_max = 8

        [languages.go]
        arg_max = 20
        ",
    )
    .unwrap();
    let base = config::resolve_base_thresholds(Some(&cfg));
    assert_eq!(base.function.arg_max, 8);
}

// ── Disabled smells ──

#[test]
fn empty_disable_list_no_filtering() {
    let disabled = config::resolve_disabled(None);
    assert!(disabled.is_empty());
}

#[test]
fn invalid_smell_name_silently_ignored() {
    let cfg: PulseConfig = toml::from_str(
        r#"
        [disable]
        smells = ["nonexistent_smell", "excess_arguments"]
        "#,
    )
    .unwrap();
    let disabled = config::resolve_disabled(Some(&cfg));
    assert_eq!(disabled.len(), 1);
    assert!(disabled.contains(&Smell::ExcessArguments));
}

#[test]
fn filter_disabled_removes_matching() {
    let mut findings = vec![
        smells::Finding { smell: Smell::ExcessArguments, location: smells::Location::Module, detail: String::new() },
        smells::Finding { smell: Smell::GodMethod, location: smells::Location::Module, detail: String::new() },
    ];
    let mut disabled = std::collections::HashSet::new();
    disabled.insert(Smell::ExcessArguments);
    config::filter_disabled(&mut findings, &disabled);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell, Smell::GodMethod);
}

// ── smell_from_snake_case ──

#[test]
fn all_smells_roundtrip() {
    let cases = [
        ("god_method", Smell::GodMethod),
        ("complex_method", Smell::ComplexMethod),
        ("large_method", Smell::LargeMethod),
        ("excess_arguments", Smell::ExcessArguments),
        ("file_too_large", Smell::FileTooLarge),
        ("code_duplication", Smell::CodeDuplication),
        ("low_cohesion", Smell::LowCohesion),
        ("stringly_typed_switch", Smell::StringlyTypedSwitch),
    ];
    for (name, expected) in cases {
        assert_eq!(smells::smell_from_snake_case(name), Some(expected), "failed for {name}");
    }
}

#[test]
fn unknown_snake_case_returns_none() {
    assert_eq!(smells::smell_from_snake_case("not_a_smell"), None);
}

// ── File discovery ──

#[test]
fn find_config_in_same_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".pulse.toml"), "[thresholds]\narg_max = 10\n").unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(&file, "fn main() {}").unwrap();

    let found = config::find_config(&file);
    assert!(found.is_some());
    assert_eq!(found.unwrap(), dir.path().join(".pulse.toml"));
}

#[test]
fn find_config_in_parent_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".pulse.toml"), "[thresholds]\narg_max = 10\n").unwrap();
    let sub = dir.path().join("src");
    std::fs::create_dir(&sub).unwrap();
    let file = sub.join("test.rs");
    std::fs::write(&file, "fn main() {}").unwrap();

    let found = config::find_config(&file);
    assert!(found.is_some());
    assert_eq!(found.unwrap(), dir.path().join(".pulse.toml"));
}

#[test]
fn find_config_returns_none_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(&file, "fn main() {}").unwrap();

    let found = config::find_config(&file);
    assert!(found.is_none());
}

#[test]
fn closer_config_shadows_outer() {
    let outer = tempfile::tempdir().unwrap();
    std::fs::write(outer.path().join(".pulse.toml"), "[thresholds]\narg_max = 5\n").unwrap();
    let inner = outer.path().join("sub");
    std::fs::create_dir(&inner).unwrap();
    std::fs::write(inner.join(".pulse.toml"), "[thresholds]\narg_max = 20\n").unwrap();
    let file = inner.join("test.rs");
    std::fs::write(&file, "fn main() {}").unwrap();

    let cfg = config::load_config(&file).unwrap();
    let resolved = config::resolve_thresholds(Some(&cfg), Language::Rust);
    assert_eq!(resolved.function.arg_max, 20);
}

// ── Integration: config affects analysis ──

#[test]
fn config_raises_threshold_suppresses_finding() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".pulse.toml"), "[thresholds]\narg_max = 10\n").unwrap();

    let args: Vec<String> = (0..crate::common::args_above()).map(|i| format!("a{i}")).collect();
    let code = format!("def foo({}):\n    pass\n", args.join(", "));
    let file = dir.path().join("test.py");
    std::fs::write(&file, &code).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", file.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("Excess Arguments"), "should be suppressed by config: {stdout}");
}

#[test]
fn no_config_detects_finding() {
    let dir = tempfile::tempdir().unwrap();
    // No .pulse.toml

    let args: Vec<String> = (0..crate::common::args_above()).map(|i| format!("a{i}")).collect();
    let code = format!("def foo({}):\n    pass\n", args.join(", "));
    let file = dir.path().join("test.py");
    std::fs::write(&file, &code).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", file.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Excess Arguments"), "should detect without config: {stdout}");
}

#[test]
fn disable_config_suppresses_smell() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".pulse.toml"), "[disable]\nsmells = [\"excess_arguments\"]\n").unwrap();

    let args: Vec<String> = (0..crate::common::args_above()).map(|i| format!("a{i}")).collect();
    let code = format!("def foo({}):\n    pass\n", args.join(", "));
    let file = dir.path().join("test.py");
    std::fs::write(&file, &code).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", file.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("Excess Arguments"), "should be disabled: {stdout}");
}

// ── Language::to_config_key ──

#[test]
fn language_config_keys_correct() {
    assert_eq!(Language::Python.to_config_key(), "python");
    assert_eq!(Language::Go.to_config_key(), "go");
    assert_eq!(Language::Cpp.to_config_key(), "cpp");
    assert_eq!(Language::CSharp.to_config_key(), "csharp");
    assert_eq!(Language::ObjectiveC.to_config_key(), "objc");
}

#[test]
fn cpg_and_naturalness_default_off() {
    let resolved = config::resolve_thresholds(None, Language::Rust);
    assert!(!resolved.cpg.enabled);
    assert!(!resolved.audit.naturalness.enabled);
    assert_eq!(resolved.cpg, t().cpg);
    assert_eq!(resolved.audit.naturalness, t().audit.naturalness);
    assert_eq!(resolved.audit.clone_cluster, t().audit.clone_cluster);
}

#[test]
fn cpg_section_overrides_flags() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds.cpg]
        enabled = true
        unused_result = true
        ",
    )
    .unwrap();
    let resolved = config::resolve_thresholds(Some(&cfg), Language::Python);
    assert!(resolved.cpg.enabled);
    assert!(resolved.cpg.unused_result);
    assert_eq!(resolved.cpg.dead_store, t().cpg.dead_store);
    assert_eq!(resolved.cpg.use_before_def, t().cpg.use_before_def);
}

#[test]
fn global_scope_thresholds_are_overridable() {
    let cfg: PulseConfig = toml::from_str(
        r"
        [thresholds]
        global_conditionals_max = 5
        global_nesting_depth = 7
        dup_assert_min = 12
        ",
    )
    .unwrap();
    let resolved = config::resolve_thresholds(Some(&cfg), Language::Python);
    assert_eq!(resolved.module.global_conditionals_max, 5);
    assert_eq!(resolved.module.global_nesting_depth, 7);
    assert_eq!(resolved.analysis.dup_assert_min, 12);
    assert_eq!(resolved.module.file_loc_warning, t().module.file_loc_warning);
    assert_eq!(resolved.analysis.consecutive_asserts_max, t().analysis.consecutive_asserts_max);
}

fn run_check_in(dir: &std::path::Path, name: &str, code: &str) -> String {
    let file = dir.join(name);
    std::fs::write(&file, code).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", file.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn config_raises_global_conditionals_threshold_suppresses_finding() {
    let code = "import os\nif os.name == 'nt':\n    sep = '\\\\'\n";

    let plain = tempfile::tempdir().unwrap();
    let default_out = run_check_in(plain.path(), "platform.py", code);
    assert!(default_out.contains("Global Conditionals"), "fixture must fire at defaults: {default_out}");

    let configured = tempfile::tempdir().unwrap();
    std::fs::write(configured.path().join(".pulse.toml"), "[thresholds]\nglobal_conditionals_max = 50\n").unwrap();
    let suppressed_out = run_check_in(configured.path(), "platform.py", code);
    assert!(!suppressed_out.contains("Global Conditionals"), "should be suppressed by config: {suppressed_out}");
}

#[test]
fn config_raises_global_nesting_threshold_suppresses_finding() {
    let code = "import os\nif os.name == 'nt':\n    if os.sep:\n        if os.curdir:\n            x = 1\n";

    let plain = tempfile::tempdir().unwrap();
    let default_out = run_check_in(plain.path(), "nested.py", code);
    assert!(default_out.contains("Deep Global Nesting"), "fixture must fire at defaults: {default_out}");

    let configured = tempfile::tempdir().unwrap();
    std::fs::write(configured.path().join(".pulse.toml"), "[thresholds]\nglobal_nesting_depth = 50\n").unwrap();
    let suppressed_out = run_check_in(configured.path(), "nested.py", code);
    assert!(!suppressed_out.contains("Deep Global Nesting"), "should be suppressed by config: {suppressed_out}");
}

#[test]
fn config_raises_dup_assert_threshold_suppresses_finding() {
    let assert_count = t().analysis.dup_assert_min * 2;
    let asserts: String = (0..assert_count).fold(String::new(), |mut s, i| {
        use std::fmt::Write;
        let _ = writeln!(s, "    assert x == {i}");
        s
    });
    let code = format!("def test_alpha():\n    x = 1\n{asserts}\n\ndef test_beta():\n    x = 1\n{asserts}");

    let plain = tempfile::tempdir().unwrap();
    let default_out = run_check_in(plain.path(), "test_dups.py", &code);
    assert!(default_out.contains("Duplicated Assertion Blocks"), "fixture must fire at defaults: {default_out}");

    let configured = tempfile::tempdir().unwrap();
    let raised = assert_count * 2;
    std::fs::write(configured.path().join(".pulse.toml"), format!("[thresholds]\ndup_assert_min = {raised}\n"))
        .unwrap();
    let suppressed_out = run_check_in(configured.path(), "test_dups.py", &code);
    assert!(
        !suppressed_out.contains("Duplicated Assertion Blocks"),
        "should be suppressed by config: {suppressed_out}"
    );
}
