use crate::audit_common::*;

const SCENARIOS: &[&str] = &[
    "shotgun_media_type",
    "shotgun_multi_class_init",
    "shotgun_cache_pattern",
    "clean_small",
    "clean_medium",
    "clean_disjoint",
    "idiom_heavy_listcomp",
    "idiom_heavy_dict_get",
];

fn expected_for(scenario: &str) -> Expected {
    let path = scenario_path(scenario, "python").join("expected.txt");
    load_expected(&path)
}

#[test]
fn audit_fixtures_root_exists_and_is_dir() {
    let root = audit_fixtures_root();
    assert!(root.exists(), "{} missing", root.display());
    assert!(root.is_dir());
}

#[test]
fn list_scenarios_returns_all_eight() {
    let scenarios = list_scenarios();
    for name in SCENARIOS {
        assert!(scenarios.contains(&(*name).to_string()), "missing scenario {name}, found: {scenarios:?}");
    }
}

#[test]
fn list_scenario_languages_for_each_includes_python() {
    for name in SCENARIOS {
        let langs = list_scenario_languages(name);
        assert!(langs.contains(&"python".to_string()), "{name} missing python subdir, got {langs:?}");
    }
}

#[test]
fn scenario_path_returns_absolute_path() {
    let p = scenario_path("clean_small", "python");
    assert!(p.is_absolute());
}

#[test]
fn each_scenario_has_python_subdir() {
    for name in SCENARIOS {
        let p = scenario_path(name, "python");
        assert!(p.exists() && p.is_dir(), "{} missing", p.display());
    }
}

#[test]
fn each_scenario_has_expected_txt() {
    for name in SCENARIOS {
        let p = scenario_path(name, "python").join("expected.txt");
        assert!(p.exists(), "{} missing", p.display());
    }
}

#[test]
fn each_scenario_has_at_least_one_python_file() {
    for name in SCENARIOS {
        let dir = scenario_path(name, "python");
        let py_count = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("py"))
            .count();
        assert!(py_count >= 1, "{} has no .py files", dir.display());
    }
}

#[test]
fn each_scenario_python_files_parse_with_tree_sitter() {
    use pulse::parse::{self, Language};
    for name in SCENARIOS {
        let dir = scenario_path(name, "python");
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) != Some("py") {
                continue;
            }
            let src = std::fs::read_to_string(&p).unwrap();
            let tree = parse::parse_only(&src, Language::Python);
            assert!(tree.is_some(), "{} did not parse", p.display());
        }
    }
}

#[test]
fn load_expected_parses_no_findings_marker() {
    let exp = expected_for("clean_small");
    assert!(exp.no_findings);
    assert_eq!(exp.scenario, "clean_small");
    assert_eq!(exp.language, "python");
    assert!(exp.findings.is_empty());
}

#[test]
fn load_expected_parses_findings_block() {
    let exp = expected_for("shotgun_media_type");
    assert!(!exp.no_findings);
    assert_eq!(exp.findings.len(), 1);
    let f = &exp.findings[0];
    assert_eq!(f.kind, "UncategorizedPattern");
    assert!(f.file_count_min >= 5);
    assert!(f.support_min >= 5);
}

#[test]
fn load_expected_populates_must_include_files() {
    let exp = expected_for("shotgun_media_type");
    let f = &exp.findings[0];
    assert!(f.must_include_files.contains(&"managers.py".to_string()));
    assert!(f.must_include_files.contains(&"services.py".to_string()));
}

#[test]
fn load_expected_populates_representative_substr() {
    let exp = expected_for("shotgun_media_type");
    let f = &exp.findings[0];
    assert_eq!(f.representative_substr.as_deref(), Some("media_type"));
}

#[test]
fn clean_scenarios_load_with_no_findings_true() {
    for name in ["clean_small", "clean_medium", "clean_disjoint"] {
        let exp = expected_for(name);
        assert!(exp.no_findings, "{name} should have no_findings=true");
        assert!(exp.findings.is_empty(), "{name} should have empty findings list");
    }
}

#[test]
fn idiom_heavy_scenarios_expect_no_findings_after_idf() {
    for name in ["idiom_heavy_listcomp", "idiom_heavy_dict_get"] {
        let exp = expected_for(name);
        assert!(exp.no_findings, "{name} idf should suppress all findings");
    }
}

#[test]
fn shotgun_scenarios_expect_at_least_one_finding() {
    for name in ["shotgun_media_type", "shotgun_multi_class_init", "shotgun_cache_pattern"] {
        let exp = expected_for(name);
        assert!(!exp.no_findings, "{name} should produce findings");
        assert!(!exp.findings.is_empty(), "{name} findings list should be non-empty");
    }
}

#[test]
fn shotgun_media_type_thresholds_within_default_min_support() {
    let exp = expected_for("shotgun_media_type");
    let f = &exp.findings[0];
    let default_min_support = t().audit.pattern_mining.freqt_min_support as u32;
    assert!(
        f.support_min >= default_min_support,
        "support_min {} should clear default min_support {}",
        f.support_min,
        default_min_support
    );
}

#[test]
fn shotgun_multi_class_init_lists_all_six_files() {
    let exp = expected_for("shotgun_multi_class_init");
    let f = &exp.findings[0];
    assert!(f.must_include_files.len() >= 6);
}

#[test]
fn copy_scenario_to_tempdir_replicates_files() {
    let tmp = copy_scenario_to_tempdir("shotgun_media_type", "python");
    let original = scenario_path("shotgun_media_type", "python");
    let original_count = count_files(&original);
    let copy_count = count_files(tmp.path());
    assert_eq!(original_count, copy_count);
    assert!(tmp.path().join("managers.py").exists());
    assert!(tmp.path().join("services.py").exists());
}

fn count_files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir).unwrap().flatten().count()
}

#[test]
fn copy_scenario_to_tempdir_content_matches() {
    let tmp = copy_scenario_to_tempdir("shotgun_media_type", "python");
    let original_managers =
        std::fs::read_to_string(scenario_path("shotgun_media_type", "python").join("managers.py")).unwrap();
    let copied_managers = std::fs::read_to_string(tmp.path().join("managers.py")).unwrap();
    assert_eq!(original_managers, copied_managers);
}

#[test]
fn shotgun_cache_pattern_lists_five_provider_files() {
    let exp = expected_for("shotgun_cache_pattern");
    let f = &exp.findings[0];
    assert_eq!(f.must_include_files.len(), 5);
    for name in ["provider_a.py", "provider_b.py", "provider_c.py", "provider_d.py", "provider_e.py"] {
        assert!(f.must_include_files.contains(&name.to_string()), "missing {name} in cache pattern fixture");
    }
}

#[test]
fn idiom_heavy_listcomp_has_eight_files() {
    let dir = scenario_path("idiom_heavy_listcomp", "python");
    let py_count = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("py"))
        .count();
    assert_eq!(py_count, 8);
}

#[test]
fn idiom_heavy_dict_get_has_eight_files() {
    let dir = scenario_path("idiom_heavy_dict_get", "python");
    let py_count = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("py"))
        .count();
    assert_eq!(py_count, 8);
}

#[test]
fn clean_medium_has_at_least_eight_files() {
    let dir = scenario_path("clean_medium", "python");
    let py_count = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("py"))
        .count();
    assert!(py_count >= 8, "clean_medium should be larger; got {py_count}");
}

#[test]
fn audit_thresholds_default_freqt_min_support_is_five() {
    assert_eq!(t().audit.pattern_mining.freqt_min_support, 5);
}

#[test]
fn audit_thresholds_default_subtree_min_depth_is_three() {
    assert_eq!(t().audit.pattern_mining.subtree_min_depth, 3);
}

#[test]
fn audit_thresholds_default_subtree_min_nodes_is_five() {
    assert_eq!(t().audit.pattern_mining.subtree_min_nodes, 5);
}

#[test]
fn audit_thresholds_default_idiom_threshold_is_half() {
    assert!((t().audit.pattern_mining.idiom_suppression_threshold - 0.5).abs() < f64::EPSILON);
}

#[test]
fn audit_thresholds_default_max_findings_is_twenty_five() {
    assert_eq!(t().audit.pattern_mining.max_findings_reported, 25);
}

#[test]
fn audit_thresholds_default_max_locations_is_ten() {
    assert_eq!(t().audit.max_locations_per_finding, 10);
}

#[test]
fn fixture_managers_py_contains_media_type_comparison() {
    let src = std::fs::read_to_string(scenario_path("shotgun_media_type", "python").join("managers.py")).unwrap();
    assert!(src.contains("media_type =="), "fixture lost the headline marker");
}

#[test]
fn fixture_provider_a_contains_cache_get_and_set() {
    let src = std::fs::read_to_string(scenario_path("shotgun_cache_pattern", "python").join("provider_a.py")).unwrap();
    assert!(src.contains("cache.get"));
    assert!(src.contains("cache.set"));
}

#[test]
fn fixture_a_py_in_multi_class_init_contains_self_user_assignment() {
    let src = std::fs::read_to_string(scenario_path("shotgun_multi_class_init", "python").join("a.py")).unwrap();
    assert!(src.contains("self.user = user"));
}
