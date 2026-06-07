use pulse::config::{
    combined_history_ignore_patterns, resolve_history_thresholds, HistoryCliOverrides, PulseConfig,
};
use pulse::history::thresholds::HistoryThresholds;

fn parse(s: &str) -> PulseConfig {
    toml::from_str(s).unwrap_or_else(|e| panic!("toml parse failed: {e}\n---\n{s}"))
}

fn parse_err(s: &str) -> bool {
    toml::from_str::<PulseConfig>(s).is_err()
}

fn defaults_co() -> u32 {
    HistoryThresholds::DEFAULTS.co_change.max_findings_reported
}
fn defaults_hot() -> u32 {
    HistoryThresholds::DEFAULTS.hotspot.max_findings_reported
}
fn defaults_ctr() -> u32 {
    HistoryThresholds::DEFAULTS.contributors.max_findings_reported
}

fn cli(co: Option<u32>, hot: Option<u32>, ctr: Option<u32>) -> HistoryCliOverrides {
    HistoryCliOverrides { co_change_top: co, hotspot_top: hot, contributors_top: ctr, hist: false, arch_trend: false }
}

#[test]
fn config_empty_string_parses_default_history() {
    let cfg = parse("");
    assert!(cfg.history.ignore_paths.is_empty());
    assert!(cfg.history.co_change.max_findings.is_none());
    assert!(cfg.history.hotspot.max_findings.is_none());
    assert!(cfg.history.contributors.max_findings.is_none());
}

#[test]
fn config_empty_history_table_parses() {
    let cfg = parse("[history]\n");
    assert!(cfg.history.ignore_paths.is_empty());
}

#[test]
fn config_ignore_paths_single_literal() {
    let cfg = parse("[history]\nignore_paths = [\"src/x.rs\"]\n");
    assert_eq!(cfg.history.ignore_paths, vec!["src/x.rs"]);
}

#[test]
fn config_ignore_paths_glob() {
    let cfg = parse("[history]\nignore_paths = [\"src/**/*.gen.rs\"]\n");
    assert_eq!(cfg.history.ignore_paths, vec!["src/**/*.gen.rs"]);
}

#[test]
fn config_ignore_paths_multiple() {
    let cfg = parse(
        "[history]\nignore_paths = [\"src/a/**\", \"src/b/**\", \"docs/**\"]\n",
    );
    assert_eq!(cfg.history.ignore_paths.len(), 3);
}

#[test]
fn config_ignore_paths_empty_array() {
    let cfg = parse("[history]\nignore_paths = []\n");
    assert!(cfg.history.ignore_paths.is_empty());
}

#[test]
fn config_co_change_max_findings_only() {
    let cfg = parse("[history.co_change]\nmax_findings = 50\n");
    assert_eq!(cfg.history.co_change.max_findings, Some(50));
    assert!(cfg.history.hotspot.max_findings.is_none());
    assert!(cfg.history.contributors.max_findings.is_none());
}

#[test]
fn config_hotspot_max_findings_only() {
    let cfg = parse("[history.hotspot]\nmax_findings = 25\n");
    assert_eq!(cfg.history.hotspot.max_findings, Some(25));
}

#[test]
fn config_contributors_max_findings_only() {
    let cfg = parse("[history.contributors]\nmax_findings = 99\n");
    assert_eq!(cfg.history.contributors.max_findings, Some(99));
}

#[test]
fn config_all_three_max_findings_combined() {
    let cfg = parse(
        "[history.co_change]\nmax_findings = 1\n[history.hotspot]\nmax_findings = 2\n[history.contributors]\nmax_findings = 3\n",
    );
    assert_eq!(cfg.history.co_change.max_findings, Some(1));
    assert_eq!(cfg.history.hotspot.max_findings, Some(2));
    assert_eq!(cfg.history.contributors.max_findings, Some(3));
}

#[test]
fn config_combined_ignore_and_caps() {
    let cfg = parse(
        "[history]\nignore_paths = [\"src/legacy/**\"]\n[history.hotspot]\nmax_findings = 7\n",
    );
    assert_eq!(cfg.history.ignore_paths, vec!["src/legacy/**"]);
    assert_eq!(cfg.history.hotspot.max_findings, Some(7));
}

#[test]
fn config_zero_caps_are_legal() {
    let cfg = parse(
        "[history.co_change]\nmax_findings = 0\n[history.hotspot]\nmax_findings = 0\n[history.contributors]\nmax_findings = 0\n",
    );
    assert_eq!(cfg.history.co_change.max_findings, Some(0));
    assert_eq!(cfg.history.hotspot.max_findings, Some(0));
    assert_eq!(cfg.history.contributors.max_findings, Some(0));
}

#[test]
fn config_large_cap_is_legal() {
    let cfg = parse("[history.hotspot]\nmax_findings = 4294967295\n");
    assert_eq!(cfg.history.hotspot.max_findings, Some(u32::MAX));
}

#[test]
fn config_negative_cap_is_rejected() {
    assert!(parse_err("[history.hotspot]\nmax_findings = -1\n"));
}

#[test]
fn config_string_cap_is_rejected() {
    assert!(parse_err("[history.hotspot]\nmax_findings = \"50\"\n"));
}

#[test]
fn config_float_cap_is_rejected() {
    assert!(parse_err("[history.hotspot]\nmax_findings = 1.5\n"));
}

#[test]
fn config_unknown_top_level_field_rejected() {
    assert!(parse_err("[history]\nbogus = 1\n"));
}

#[test]
fn config_unknown_sub_field_rejected() {
    assert!(parse_err("[history.hotspot]\nbogus = 1\n"));
}

#[test]
fn config_unknown_co_change_field_rejected() {
    assert!(parse_err("[history.co_change]\nbogus = 1\n"));
}

#[test]
fn config_unknown_contributors_field_rejected() {
    assert!(parse_err("[history.contributors]\nbogus = 1\n"));
}

#[test]
fn config_ignore_paths_typo_field_rejected() {
    assert!(parse_err("[history]\nignore_path = [\"x\"]\n"));
}

#[test]
fn config_history_alongside_ignore_section() {
    let cfg = parse(
        "[ignore]\npaths = [\"vendor/**\"]\n[history]\nignore_paths = [\"docs/**\"]\n",
    );
    assert_eq!(cfg.ignore.paths, vec!["vendor/**"]);
    assert_eq!(cfg.history.ignore_paths, vec!["docs/**"]);
}

#[test]
fn config_history_alongside_audit_section() {
    let cfg = parse(
        "[audit]\nhide_categories = [\"x\"]\n[history]\nignore_paths = [\"y\"]\n",
    );
    assert_eq!(cfg.audit.hide_categories, vec!["x"]);
    assert_eq!(cfg.history.ignore_paths, vec!["y"]);
}

#[test]
fn config_history_alongside_thresholds() {
    let cfg = parse(
        "[thresholds]\ncc_warning = 5\n[history.hotspot]\nmax_findings = 3\n",
    );
    assert_eq!(cfg.thresholds.function.cc_warning, Some(5));
    assert_eq!(cfg.history.hotspot.max_findings, Some(3));
}

#[test]
fn resolve_history_defaults_with_no_config_no_cli() {
    let t = resolve_history_thresholds(None, cli(None, None, None));
    assert_eq!(t.co_change.max_findings_reported, defaults_co());
    assert_eq!(t.hotspot.max_findings_reported, defaults_hot());
    assert_eq!(t.contributors.max_findings_reported, defaults_ctr());
}

#[test]
fn resolve_history_config_co_change_applied() {
    let cfg = parse("[history.co_change]\nmax_findings = 50\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    assert_eq!(t.co_change.max_findings_reported, 50);
    assert_eq!(t.hotspot.max_findings_reported, defaults_hot());
    assert_eq!(t.contributors.max_findings_reported, defaults_ctr());
}

#[test]
fn resolve_history_config_hotspot_applied() {
    let cfg = parse("[history.hotspot]\nmax_findings = 100\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    assert_eq!(t.hotspot.max_findings_reported, 100);
    assert_eq!(t.co_change.max_findings_reported, defaults_co());
}

#[test]
fn resolve_history_config_contributors_applied() {
    let cfg = parse("[history.contributors]\nmax_findings = 7\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    assert_eq!(t.contributors.max_findings_reported, 7);
    assert_eq!(t.co_change.max_findings_reported, defaults_co());
}

#[test]
fn resolve_history_cli_co_change_overrides_default() {
    let t = resolve_history_thresholds(None, cli(Some(2), None, None));
    assert_eq!(t.co_change.max_findings_reported, 2);
}

#[test]
fn resolve_history_cli_hotspot_overrides_default() {
    let t = resolve_history_thresholds(None, cli(None, Some(3), None));
    assert_eq!(t.hotspot.max_findings_reported, 3);
}

#[test]
fn resolve_history_cli_contributors_overrides_default() {
    let t = resolve_history_thresholds(None, cli(None, None, Some(4)));
    assert_eq!(t.contributors.max_findings_reported, 4);
}

#[test]
fn resolve_history_cli_overrides_config() {
    let cfg = parse("[history.co_change]\nmax_findings = 50\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(Some(2), None, None));
    assert_eq!(t.co_change.max_findings_reported, 2);
}

#[test]
fn resolve_history_cli_overrides_config_for_all_three() {
    let cfg = parse(
        "[history.co_change]\nmax_findings = 10\n[history.hotspot]\nmax_findings = 20\n[history.contributors]\nmax_findings = 30\n",
    );
    let t = resolve_history_thresholds(Some(&cfg), cli(Some(1), Some(2), Some(3)));
    assert_eq!(t.co_change.max_findings_reported, 1);
    assert_eq!(t.hotspot.max_findings_reported, 2);
    assert_eq!(t.contributors.max_findings_reported, 3);
}

#[test]
fn resolve_history_cli_partial_overrides_keep_config() {
    let cfg = parse(
        "[history.co_change]\nmax_findings = 10\n[history.hotspot]\nmax_findings = 20\n[history.contributors]\nmax_findings = 30\n",
    );
    let t = resolve_history_thresholds(Some(&cfg), cli(None, Some(99), None));
    assert_eq!(t.co_change.max_findings_reported, 10);
    assert_eq!(t.hotspot.max_findings_reported, 99);
    assert_eq!(t.contributors.max_findings_reported, 30);
}

#[test]
fn resolve_history_cli_zero_is_respected() {
    let t = resolve_history_thresholds(None, cli(Some(0), Some(0), Some(0)));
    assert_eq!(t.co_change.max_findings_reported, 0);
    assert_eq!(t.hotspot.max_findings_reported, 0);
    assert_eq!(t.contributors.max_findings_reported, 0);
}

#[test]
fn resolve_history_config_zero_is_respected() {
    let cfg = parse(
        "[history.co_change]\nmax_findings = 0\n[history.hotspot]\nmax_findings = 0\n[history.contributors]\nmax_findings = 0\n",
    );
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    assert_eq!(t.co_change.max_findings_reported, 0);
    assert_eq!(t.hotspot.max_findings_reported, 0);
    assert_eq!(t.contributors.max_findings_reported, 0);
}

#[test]
fn resolve_history_max_value_is_respected() {
    let t = resolve_history_thresholds(None, cli(Some(u32::MAX), None, None));
    assert_eq!(t.co_change.max_findings_reported, u32::MAX);
}

#[test]
fn resolve_history_does_not_touch_min_support() {
    let cfg = parse("[history.co_change]\nmax_findings = 999\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    assert_eq!(t.co_change.min_support, HistoryThresholds::DEFAULTS.co_change.min_support);
}

#[test]
fn resolve_history_does_not_touch_min_revisions() {
    let cfg = parse("[history.hotspot]\nmax_findings = 999\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    assert_eq!(t.hotspot.min_revisions, HistoryThresholds::DEFAULTS.hotspot.min_revisions);
}

#[test]
fn resolve_history_does_not_touch_min_score() {
    let cfg = parse("[history.hotspot]\nmax_findings = 999\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    assert_eq!(t.hotspot.min_score, HistoryThresholds::DEFAULTS.hotspot.min_score);
}

#[test]
fn resolve_history_does_not_touch_minor_contributor_pct() {
    let cfg = parse("[history.contributors]\nmax_findings = 999\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    let expected = HistoryThresholds::DEFAULTS.contributors.minor_contributor_pct;
    assert!((t.contributors.minor_contributor_pct - expected).abs() < f64::EPSILON);
}

#[test]
fn resolve_history_does_not_touch_max_commit_files() {
    let cfg = parse("[history.hotspot]\nmax_findings = 9\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    assert_eq!(t.max_commit_files, HistoryThresholds::DEFAULTS.max_commit_files);
}

#[test]
fn combined_ignore_no_config_is_empty() {
    let patterns = combined_history_ignore_patterns(None);
    assert!(patterns.is_empty());
}

#[test]
fn combined_ignore_empty_config_is_empty() {
    let cfg = parse("");
    let patterns = combined_history_ignore_patterns(Some(&cfg));
    assert!(patterns.is_empty());
}

#[test]
fn combined_ignore_global_only() {
    let cfg = parse("[ignore]\npaths = [\"vendor/**\"]\n");
    let patterns = combined_history_ignore_patterns(Some(&cfg));
    assert_eq!(patterns, vec!["vendor/**"]);
}

#[test]
fn combined_ignore_history_only() {
    let cfg = parse("[history]\nignore_paths = [\"docs/**\"]\n");
    let patterns = combined_history_ignore_patterns(Some(&cfg));
    assert_eq!(patterns, vec!["docs/**"]);
}

#[test]
fn combined_ignore_merges_global_and_history() {
    let cfg = parse(
        "[ignore]\npaths = [\"vendor/**\"]\n[history]\nignore_paths = [\"docs/**\", \"build/**\"]\n",
    );
    let patterns = combined_history_ignore_patterns(Some(&cfg));
    assert_eq!(patterns.len(), 3);
    assert!(patterns.contains(&"vendor/**".to_string()));
    assert!(patterns.contains(&"docs/**".to_string()));
    assert!(patterns.contains(&"build/**".to_string()));
}

#[test]
fn combined_ignore_preserves_global_first_then_history() {
    let cfg = parse(
        "[ignore]\npaths = [\"global/a\", \"global/b\"]\n[history]\nignore_paths = [\"hist/a\", \"hist/b\"]\n",
    );
    let patterns = combined_history_ignore_patterns(Some(&cfg));
    assert_eq!(patterns, vec!["global/a", "global/b", "hist/a", "hist/b"]);
}

#[test]
fn combined_ignore_duplicates_are_kept() {
    let cfg = parse(
        "[ignore]\npaths = [\"vendor/**\"]\n[history]\nignore_paths = [\"vendor/**\"]\n",
    );
    let patterns = combined_history_ignore_patterns(Some(&cfg));
    assert_eq!(patterns.len(), 2);
}

#[test]
fn combined_ignore_handles_empty_strings_in_input() {
    let cfg = parse("[history]\nignore_paths = [\"\", \"keep/**\"]\n");
    let patterns = combined_history_ignore_patterns(Some(&cfg));
    assert_eq!(patterns.len(), 2);
}

#[test]
fn combined_ignore_global_empty_history_nonempty() {
    let cfg = parse("[ignore]\npaths = []\n[history]\nignore_paths = [\"x/**\"]\n");
    let patterns = combined_history_ignore_patterns(Some(&cfg));
    assert_eq!(patterns, vec!["x/**"]);
}

#[test]
fn combined_ignore_returns_owned_strings_no_aliasing() {
    let cfg = parse("[history]\nignore_paths = [\"a/**\"]\n");
    let patterns = combined_history_ignore_patterns(Some(&cfg));
    assert_eq!(patterns[0], "a/**");
    drop(cfg);
    assert_eq!(patterns[0], "a/**");
}

#[test]
fn cli_overrides_default_is_all_none() {
    let o = HistoryCliOverrides::default();
    assert!(o.co_change_top.is_none());
    assert!(o.hotspot_top.is_none());
    assert!(o.contributors_top.is_none());
}

#[test]
fn cli_overrides_is_copy_and_clone() {
    let o = cli(Some(1), Some(2), Some(3));
    let o2 = o;
    let o3 = o2;
    assert_eq!(o3.co_change_top, Some(1));
    assert_eq!(o3.hotspot_top, Some(2));
    assert_eq!(o3.contributors_top, Some(3));
}

#[test]
fn resolve_history_with_only_two_config_caps_set() {
    let cfg = parse(
        "[history.co_change]\nmax_findings = 5\n[history.contributors]\nmax_findings = 6\n",
    );
    let t = resolve_history_thresholds(Some(&cfg), cli(None, None, None));
    assert_eq!(t.co_change.max_findings_reported, 5);
    assert_eq!(t.hotspot.max_findings_reported, defaults_hot());
    assert_eq!(t.contributors.max_findings_reported, 6);
}

#[test]
fn resolve_history_cli_only_one_with_config_present_but_unrelated() {
    let cfg = parse("[history]\nignore_paths = [\"x/**\"]\n");
    let t = resolve_history_thresholds(Some(&cfg), cli(Some(9), None, None));
    assert_eq!(t.co_change.max_findings_reported, 9);
    assert_eq!(t.hotspot.max_findings_reported, defaults_hot());
    assert_eq!(t.contributors.max_findings_reported, defaults_ctr());
}

#[test]
fn resolve_history_defaults_are_the_documented_values() {
    let t = resolve_history_thresholds(None, cli(None, None, None));
    assert_eq!(t.co_change.max_findings_reported, 20);
    assert_eq!(t.hotspot.max_findings_reported, 10);
    assert_eq!(t.contributors.max_findings_reported, 20);
}

#[test]
fn config_history_with_other_sections_no_cross_contamination() {
    let cfg = parse(
        "[ignore]\npaths = [\"g/**\"]\n[audit]\nhide_categories = [\"x\"]\n[history]\nignore_paths = [\"h/**\"]\n[history.hotspot]\nmax_findings = 7\n",
    );
    assert_eq!(cfg.ignore.paths, vec!["g/**"]);
    assert_eq!(cfg.audit.hide_categories, vec!["x"]);
    assert_eq!(cfg.history.ignore_paths, vec!["h/**"]);
    assert_eq!(cfg.history.hotspot.max_findings, Some(7));
}

#[test]
fn config_history_pass_max_findings_is_optional() {
    let cfg = parse("[history.co_change]\n");
    assert!(cfg.history.co_change.max_findings.is_none());
}
