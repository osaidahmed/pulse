#[path = "integration/common.rs"]
#[macro_use]
mod common;

#[path = "integration/audit_common.rs"]
mod audit_common;

#[path = "integration/boundaries.rs"]
mod boundaries;

#[path = "integration/buildmeta.rs"]
mod buildmeta;

#[path = "integration/calibrate_census.rs"]
mod calibrate_census;

#[path = "integration/calibrate_estimator.rs"]
mod calibrate_estimator;

#[path = "integration/calibrate_cdf.rs"]
mod calibrate_cdf;
#[path = "integration/calibrate_floor.rs"]
mod calibrate_floor;

#[path = "integration/session_clones.rs"]
mod session_clones;

#[path = "integration/dead_function.rs"]
mod dead_function;

#[path = "integration/dead_function_corpus.rs"]
mod dead_function_corpus;

#[path = "integration/cpg_corpus.rs"]
mod cpg_corpus;

#[path = "integration/framework_applicability.rs"]
mod framework_applicability;

#[path = "integration/calibrate_emit.rs"]
mod calibrate_emit;

#[path = "integration/calibrate_priors.rs"]
mod calibrate_priors;

#[path = "integration/priors_sweep.rs"]
mod priors_sweep;

#[path = "integration/walker_stack_guard.rs"]
mod walker_stack_guard;

#[path = "integration/baseline_ratchet.rs"]
mod baseline_ratchet;

#[path = "integration/cpg_mutation_hardening.rs"]
mod cpg_mutation_hardening;
#[path = "integration/mutation_hardening.rs"]
mod mutation_hardening;
#[path = "integration/negative_controls.rs"]
mod negative_controls;
#[path = "integration/parity.rs"]
mod parity;
#[path = "integration/turn_scan_backstop.rs"]
mod turn_scan_backstop;

#[path = "integration/history_common.rs"]
mod history_common;

#[path = "integration/advisory_channel.rs"]
mod advisory_channel;
#[path = "integration/analytics_tests.rs"]
mod analytics_tests;
#[path = "integration/applicability_matrix.rs"]
mod applicability_matrix;
#[path = "integration/audit_abstractness.rs"]
mod audit_abstractness;
#[path = "integration/audit_arch_smells.rs"]
mod audit_arch_smells;
#[path = "integration/audit_call_extraction_clean.rs"]
mod audit_call_extraction_clean;
#[path = "integration/audit_call_extraction_extended.rs"]
mod audit_call_extraction_extended;
#[path = "integration/audit_call_graph.rs"]
mod audit_call_graph;
#[path = "integration/audit_call_graph_extraction.rs"]
mod audit_call_graph_extraction;
#[path = "integration/audit_call_graph_skeleton.rs"]
mod audit_call_graph_skeleton;
#[path = "integration/audit_centrality.rs"]
mod audit_centrality;
#[path = "integration/audit_class_registry.rs"]
mod audit_class_registry;
#[path = "integration/audit_clones.rs"]
mod audit_clones;
#[path = "integration/audit_community.rs"]
mod audit_community;
#[path = "integration/audit_component_thresholds.rs"]
mod audit_component_thresholds;
#[path = "integration/audit_components.rs"]
mod audit_components;
#[path = "integration/audit_constraint_smells.rs"]
mod audit_constraint_smells;
#[path = "integration/audit_corpus_stats_unit.rs"]
mod audit_corpus_stats_unit;
#[path = "integration/audit_coverage_disclosure.rs"]
mod audit_coverage_disclosure;
#[path = "integration/audit_cross_language_separation.rs"]
mod audit_cross_language_separation;
#[path = "integration/audit_cycle_shapes.rs"]
mod audit_cycle_shapes;
#[path = "integration/audit_cycles.rs"]
mod audit_cycles;
#[path = "integration/audit_decoupling.rs"]
mod audit_decoupling;
#[path = "integration/audit_definitions.rs"]
mod audit_definitions;
#[path = "integration/audit_deps_reconcile.rs"]
mod audit_deps_reconcile;
#[path = "integration/audit_detector_correctness.rs"]
mod audit_detector_correctness;
#[path = "integration/audit_discovery.rs"]
mod audit_discovery;
#[path = "integration/audit_discovery_extended.rs"]
mod audit_discovery_extended;
#[path = "integration/audit_divergent_change.rs"]
mod audit_divergent_change;
#[path = "integration/audit_divergent_change_extended.rs"]
mod audit_divergent_change_extended;
#[path = "integration/audit_e2e.rs"]
mod audit_e2e;
#[path = "integration/audit_edge_cases.rs"]
mod audit_edge_cases;
#[path = "integration/audit_feature_envy.rs"]
mod audit_feature_envy;
#[path = "integration/audit_feature_envy_extended.rs"]
mod audit_feature_envy_extended;
#[path = "integration/audit_fingerprint.rs"]
mod audit_fingerprint;
#[path = "integration/audit_fingerprint_extended.rs"]
mod audit_fingerprint_extended;
#[path = "integration/audit_fixture_loader.rs"]
mod audit_fixture_loader;
#[path = "integration/audit_god_class.rs"]
mod audit_god_class;
#[path = "integration/audit_god_class_extended.rs"]
mod audit_god_class_extended;
#[path = "integration/audit_graph.rs"]
mod audit_graph;
#[path = "integration/audit_ifdef_density.rs"]
mod audit_ifdef_density;
#[path = "integration/audit_ignore.rs"]
mod audit_ignore;
#[path = "integration/audit_import_call_form_extended.rs"]
mod audit_import_call_form_extended;
#[path = "integration/audit_import_command_form_extended.rs"]
mod audit_import_command_form_extended;
#[path = "integration/audit_import_jsts_extended.rs"]
mod audit_import_jsts_extended;
#[path = "integration/audit_import_php_extended.rs"]
mod audit_import_php_extended;
#[path = "integration/audit_import_preprocessor_extended.rs"]
mod audit_import_preprocessor_extended;
#[path = "integration/audit_imports.rs"]
mod audit_imports;
#[path = "integration/audit_imports_extended.rs"]
mod audit_imports_extended;
#[path = "integration/audit_imports_lossy.rs"]
mod audit_imports_lossy;
#[path = "integration/audit_imports_suffix_fallback.rs"]
mod audit_imports_suffix_fallback;
#[path = "integration/audit_inheritance_graph.rs"]
mod audit_inheritance_graph;
#[path = "integration/audit_lang_kinds_table.rs"]
mod audit_lang_kinds_table;
#[path = "integration/audit_languages.rs"]
mod audit_languages;
#[path = "integration/audit_martin.rs"]
mod audit_martin;
#[path = "integration/audit_multi_lang_dirs.rs"]
mod audit_multi_lang_dirs;
#[path = "integration/audit_named_smell_confidence_routing.rs"]
mod audit_named_smell_confidence_routing;
#[path = "integration/audit_named_smells_e2e.rs"]
mod audit_named_smells_e2e;
#[path = "integration/audit_named_smells_stress.rs"]
mod audit_named_smells_stress;
#[path = "integration/audit_named_smells_threshold.rs"]
mod audit_named_smells_threshold;
#[path = "integration/audit_naturalness.rs"]
mod audit_naturalness;
#[path = "integration/audit_negative_paths.rs"]
mod audit_negative_paths;
#[path = "integration/audit_output.rs"]
mod audit_output;
#[path = "integration/audit_output_advisory.rs"]
mod audit_output_advisory;
#[path = "integration/audit_output_cross_variant.rs"]
mod audit_output_cross_variant;
#[path = "integration/audit_output_divergent_change.rs"]
mod audit_output_divergent_change;
#[path = "integration/audit_output_extended.rs"]
mod audit_output_extended;
#[path = "integration/audit_output_feature_envy.rs"]
mod audit_output_feature_envy;
#[path = "integration/audit_output_god_class.rs"]
mod audit_output_god_class;
#[path = "integration/audit_output_martin.rs"]
mod audit_output_martin;
#[path = "integration/audit_output_named_smells.rs"]
mod audit_output_named_smells;
#[path = "integration/audit_output_parallel_inheritance.rs"]
mod audit_output_parallel_inheritance;
#[path = "integration/audit_output_refused_bequest.rs"]
mod audit_output_refused_bequest;
#[path = "integration/audit_output_remodularization.rs"]
mod audit_output_remodularization;
#[path = "integration/audit_package_metrics_skeleton.rs"]
mod audit_package_metrics_skeleton;
#[path = "integration/audit_parallel_inheritance.rs"]
mod audit_parallel_inheritance;
#[path = "integration/audit_parallel_inheritance_extended.rs"]
mod audit_parallel_inheritance_extended;
#[path = "integration/audit_per_language_deep.rs"]
mod audit_per_language_deep;
#[path = "integration/audit_pipeline_integration.rs"]
mod audit_pipeline_integration;
#[path = "integration/audit_reflexion.rs"]
mod audit_reflexion;
#[path = "integration/audit_refused_bequest.rs"]
mod audit_refused_bequest;
#[path = "integration/audit_refused_bequest_extended.rs"]
mod audit_refused_bequest_extended;
#[path = "integration/audit_remaining_coverage.rs"]
mod audit_remaining_coverage;
#[path = "integration/audit_remodularization.rs"]
mod audit_remodularization;
#[path = "integration/audit_scoring.rs"]
mod audit_scoring;
#[path = "integration/audit_scoring_extended.rs"]
mod audit_scoring_extended;
#[path = "integration/audit_seeded_fingerprint.rs"]
mod audit_seeded_fingerprint;
#[path = "integration/audit_shotgun_surgery.rs"]
mod audit_shotgun_surgery;
#[path = "integration/audit_stress.rs"]
mod audit_stress;
#[path = "integration/audit_suppression.rs"]
mod audit_suppression;
#[path = "integration/audit_taint.rs"]
mod audit_taint;
#[path = "integration/audit_test_exclusion.rs"]
mod audit_test_exclusion;
#[path = "integration/audit_test_roots.rs"]
mod audit_test_roots;
#[path = "integration/audit_threshold_martin.rs"]
mod audit_threshold_martin;
#[path = "integration/audit_vendor_filter_unit.rs"]
mod audit_vendor_filter_unit;
#[path = "integration/audit_vuln_clones.rs"]
mod audit_vuln_clones;
#[path = "integration/audit_walker.rs"]
mod audit_walker;
#[path = "integration/audit_walker_extended.rs"]
mod audit_walker_extended;
#[path = "integration/audit_ws1_stats.rs"]
mod audit_ws1_stats;
#[path = "integration/baseline_filtering.rs"]
mod baseline_filtering;
#[path = "integration/boundary_thresholds.rs"]
mod boundary_thresholds;
#[path = "integration/budget_command.rs"]
mod budget_command;
#[path = "integration/c_smells.rs"]
mod c_smells;
#[path = "integration/c_stress.rs"]
mod c_stress;
#[path = "integration/cfg_construction.rs"]
mod cfg_construction;
#[path = "integration/cli_clap_migration.rs"]
mod cli_clap_migration;
#[path = "integration/cli_commands.rs"]
mod cli_commands;
#[path = "integration/cobol_smells.rs"]
mod cobol_smells;
#[path = "integration/cobol_stress.rs"]
mod cobol_stress;
#[path = "integration/config_tests.rs"]
mod config_tests;
#[path = "integration/cov_audit_compound_severity.rs"]
mod cov_audit_compound_severity;
#[path = "integration/cov_audit_cycle_shapes.rs"]
mod cov_audit_cycle_shapes;
#[path = "integration/cov_audit_expression_filter.rs"]
mod cov_audit_expression_filter;
#[path = "integration/cov_audit_import_call_form.rs"]
mod cov_audit_import_call_form;
#[path = "integration/cov_audit_named_smells_confidence.rs"]
mod cov_audit_named_smells_confidence;
#[path = "integration/cov_baselines_internals.rs"]
mod cov_baselines_internals;
#[path = "integration/cov_config_empty_branches.rs"]
mod cov_config_empty_branches;
#[path = "integration/cov_detector_parallel_inheritance_workaround.rs"]
mod cov_detector_parallel_inheritance_workaround;
#[path = "integration/cov_history_cmd_calibrate.rs"]
mod cov_history_cmd_calibrate;
#[path = "integration/cov_history_git_subprocess.rs"]
mod cov_history_git_subprocess;
#[path = "integration/cov_history_output_blob_shotgun.rs"]
mod cov_history_output_blob_shotgun;
#[path = "integration/cov_import_php.rs"]
mod cov_import_php;
#[path = "integration/cov_import_preprocessor_branches.rs"]
mod cov_import_preprocessor_branches;
#[path = "integration/cov_output_arch.rs"]
mod cov_output_arch;
#[path = "integration/cov_r_walker_assignment_forms.rs"]
mod cov_r_walker_assignment_forms;
#[path = "integration/cov_setup_arch.rs"]
mod cov_setup_arch;
#[path = "integration/cov_taint_state_merge.rs"]
mod cov_taint_state_merge;
#[path = "integration/cov_typescript_walker.rs"]
mod cov_typescript_walker;
#[path = "integration/cov_walk_cpp.rs"]
mod cov_walk_cpp;
#[path = "integration/cov_walk_csharp.rs"]
mod cov_walk_csharp;
#[path = "integration/coverage_gaps.rs"]
mod coverage_gaps;
#[path = "integration/cpp_smells.rs"]
mod cpp_smells;
#[path = "integration/cpp_stress.rs"]
mod cpp_stress;
#[path = "integration/cross_lang_complexity.rs"]
mod cross_lang_complexity;
#[path = "integration/csharp_smells.rs"]
mod csharp_smells;
#[path = "integration/csharp_stress.rs"]
mod csharp_stress;
#[path = "integration/csharp_walker_extended.rs"]
mod csharp_walker_extended;
#[path = "integration/d_smells.rs"]
mod d_smells;
#[path = "integration/d_stress.rs"]
mod d_stress;
#[path = "integration/defensive_caps.rs"]
mod defensive_caps;
#[path = "integration/diff_filtering_stress.rs"]
mod diff_filtering_stress;
#[path = "integration/duplication_extended.rs"]
mod duplication_extended;
#[path = "integration/extract_suggester.rs"]
mod extract_suggester;
#[path = "integration/false_positive_reduction.rs"]
mod false_positive_reduction;
#[path = "integration/fused_equivalence.rs"]
mod fused_equivalence;
#[path = "integration/fuzz_corpus.rs"]
mod fuzz_corpus;
#[path = "integration/fuzzy_duplication.rs"]
mod fuzzy_duplication;
#[path = "integration/go_smells.rs"]
mod go_smells;
#[path = "integration/go_stress.rs"]
mod go_stress;
#[path = "integration/golden_findings.rs"]
mod golden_findings;
#[path = "integration/groovy_smells.rs"]
mod groovy_smells;
#[path = "integration/groovy_stress.rs"]
mod groovy_stress;
#[path = "integration/haskell_smells.rs"]
mod haskell_smells;
#[path = "integration/haskell_stress.rs"]
mod haskell_stress;
#[path = "integration/hist_crossval.rs"]
mod hist_crossval;
#[path = "integration/history_arch_trend.rs"]
mod history_arch_trend;
#[path = "integration/history_build_co_change.rs"]
mod history_build_co_change;
#[path = "integration/history_cli.rs"]
mod history_cli;
#[path = "integration/history_cli_overrides.rs"]
mod history_cli_overrides;
#[path = "integration/history_co_change.rs"]
mod history_co_change;
#[path = "integration/history_config_overrides.rs"]
mod history_config_overrides;
#[path = "integration/history_contributors.rs"]
mod history_contributors;
#[path = "integration/history_e2e_scenarios.rs"]
mod history_e2e_scenarios;
#[path = "integration/history_edge_cases.rs"]
mod history_edge_cases;
#[path = "integration/history_edges.rs"]
mod history_edges;
#[path = "integration/history_finding.rs"]
mod history_finding;
#[path = "integration/history_git.rs"]
mod history_git;
#[path = "integration/history_hist_smells.rs"]
mod history_hist_smells;
#[path = "integration/history_hotspots.rs"]
mod history_hotspots;
#[path = "integration/history_ignore_paths.rs"]
mod history_ignore_paths;
#[path = "integration/history_languages_static_link.rs"]
mod history_languages_static_link;
#[path = "integration/history_negative_paths.rs"]
mod history_negative_paths;
#[path = "integration/history_orchestrator.rs"]
mod history_orchestrator;
#[path = "integration/history_output.rs"]
mod history_output;
#[path = "integration/hook_diff_filtering.rs"]
mod hook_diff_filtering;
#[path = "integration/hook_filtering.rs"]
mod hook_filtering;
#[path = "integration/hook_protocol.rs"]
mod hook_protocol;
#[path = "integration/import_check.rs"]
mod import_check;
#[path = "integration/intensity_grading.rs"]
mod intensity_grading;
#[path = "integration/interaction_model.rs"]
mod interaction_model;
#[path = "integration/java_smells.rs"]
mod java_smells;
#[path = "integration/java_stress.rs"]
mod java_stress;
#[path = "integration/javascript_smells.rs"]
mod javascript_smells;
#[path = "integration/javascript_stress.rs"]
mod javascript_stress;
#[path = "integration/jit_calibration.rs"]
mod jit_calibration;
#[path = "integration/kotlin_smells.rs"]
mod kotlin_smells;
#[path = "integration/kotlin_stress.rs"]
mod kotlin_stress;
#[path = "integration/langkinds.rs"]
mod langkinds;
#[path = "integration/lua_smells.rs"]
mod lua_smells;
#[path = "integration/lua_stress.rs"]
mod lua_stress;
#[path = "integration/module_boundary_thresholds.rs"]
mod module_boundary_thresholds;
#[path = "integration/multi_lang_walker_extended.rs"]
mod multi_lang_walker_extended;
#[path = "integration/new_smells_stress.rs"]
mod new_smells_stress;
#[path = "integration/objc_smells.rs"]
mod objc_smells;
#[path = "integration/objc_stress.rs"]
mod objc_stress;
#[path = "integration/parse_refactor.rs"]
mod parse_refactor;
#[path = "integration/path_ignore.rs"]
mod path_ignore;
#[path = "integration/php_smells.rs"]
mod php_smells;
#[path = "integration/php_stress.rs"]
mod php_stress;
#[path = "integration/php_walker_extended.rs"]
mod php_walker_extended;
#[path = "integration/production_fixtures.rs"]
mod production_fixtures;
#[path = "integration/production_fixtures_new_smells.rs"]
mod production_fixtures_new_smells;
#[path = "integration/property_invariants.rs"]
mod property_invariants;
#[path = "integration/python_future_smells.rs"]
mod python_future_smells;
#[path = "integration/python_smells.rs"]
mod python_smells;
#[path = "integration/python_stress.rs"]
mod python_stress;
#[path = "integration/python_walker_extended.rs"]
mod python_walker_extended;
#[path = "integration/r_smells.rs"]
mod r_smells;
#[path = "integration/r_stress.rs"]
mod r_stress;
#[path = "integration/r_walker_extended.rs"]
mod r_walker_extended;
#[path = "integration/refmine.rs"]
mod refmine;
#[path = "integration/regression_detection.rs"]
mod regression_detection;
#[path = "integration/ruby_smells.rs"]
mod ruby_smells;
#[path = "integration/ruby_stress.rs"]
mod ruby_stress;
#[path = "integration/rust_smells.rs"]
mod rust_smells;
#[path = "integration/rust_stress.rs"]
mod rust_stress;
#[path = "integration/scoped_cache_isolation.rs"]
mod scoped_cache_isolation;
#[path = "integration/setup_idempotency.rs"]
mod setup_idempotency;
#[path = "integration/setup_tests.rs"]
mod setup_tests;
#[path = "integration/simhash_clones.rs"]
mod simhash_clones;
#[path = "integration/swift_smells.rs"]
mod swift_smells;
#[path = "integration/swift_stress.rs"]
mod swift_stress;
#[path = "integration/tcl_smells.rs"]
mod tcl_smells;
#[path = "integration/tcl_stress.rs"]
mod tcl_stress;
#[path = "integration/test_file_detection.rs"]
mod test_file_detection;
#[path = "integration/typescript_smells.rs"]
mod typescript_smells;
#[path = "integration/typescript_stress.rs"]
mod typescript_stress;
#[path = "integration/walk_field_accesses_foreign.rs"]
mod walk_field_accesses_foreign;
#[path = "integration/walk_field_accesses_foreign_extended.rs"]
mod walk_field_accesses_foreign_extended;
#[path = "integration/walk_parent_class.rs"]
mod walk_parent_class;
#[path = "integration/walker_structural_completeness.rs"]
mod walker_structural_completeness;
#[path = "integration/zig_smells.rs"]
mod zig_smells;
#[path = "integration/zig_stress.rs"]
mod zig_stress;
