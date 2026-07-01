#[path = "integration/common.rs"]
#[macro_use]
mod common;

#[path = "integration/audit_common.rs"]
mod audit_common;

#[path = "integration/binding_common.rs"]
mod binding_common;

#[path = "integration/cov3_stragglers.rs"]
mod cov3_stragglers;

#[path = "integration/session_clones.rs"]
mod session_clones;

#[path = "integration/dead_function.rs"]
mod dead_function;

#[path = "integration/dead_function_corpus.rs"]
mod dead_function_corpus;

#[path = "integration/framework_applicability.rs"]
mod framework_applicability;

#[path = "integration/calibrate_emit.rs"]
mod calibrate_emit;

#[path = "integration/baseline_ratchet.rs"]
mod baseline_ratchet;

#[path = "integration/cpg_mutation_hardening.rs"]
mod cpg_mutation_hardening;
#[path = "integration/mutation_hardening.rs"]
mod mutation_hardening;
#[path = "integration/turn_scan_backstop.rs"]
mod turn_scan_backstop;

#[path = "integration/history_common.rs"]
#[allow(dead_code)]
mod history_common;

#[path = "integration/advisory_channel.rs"]
mod advisory_channel;
#[path = "integration/analytics_serde.rs"]
mod analytics_serde;
#[path = "integration/analytics_tests.rs"]
mod analytics_tests;
#[path = "integration/applicability_matrix.rs"]
mod applicability_matrix;
#[path = "integration/audit_e2e.rs"]
mod audit_e2e;
#[path = "integration/audit_multi_lang_dirs.rs"]
mod audit_multi_lang_dirs;
#[path = "integration/audit_negative_paths.rs"]
mod audit_negative_paths;
#[path = "integration/audit_stress.rs"]
mod audit_stress;
#[path = "integration/audit_test_exclusion.rs"]
mod audit_test_exclusion;
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
#[path = "integration/cov2_history_cmd.rs"]
mod cov2_history_cmd;
#[path = "integration/cov_baselines_internals.rs"]
mod cov_baselines_internals;
#[path = "integration/cov_history.rs"]
mod cov_history;
#[path = "integration/cov_history_cmd_calibrate.rs"]
mod cov_history_cmd_calibrate;
#[path = "integration/cov_runtime_misc.rs"]
mod cov_runtime_misc;
#[path = "integration/cov_setup_arch.rs"]
mod cov_setup_arch;
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
#[path = "integration/d_smells.rs"]
mod d_smells;
#[path = "integration/d_stress.rs"]
mod d_stress;
#[path = "integration/diff_filtering_stress.rs"]
mod diff_filtering_stress;
#[path = "integration/extract_suggester.rs"]
mod extract_suggester;
#[path = "integration/false_positive_reduction.rs"]
mod false_positive_reduction;
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
#[path = "integration/history_cli.rs"]
mod history_cli;
#[path = "integration/history_cli_overrides.rs"]
mod history_cli_overrides;
#[path = "integration/history_ignore_paths.rs"]
mod history_ignore_paths;
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
#[path = "integration/kotlin_smells.rs"]
mod kotlin_smells;
#[path = "integration/kotlin_stress.rs"]
mod kotlin_stress;
#[path = "integration/lua_smells.rs"]
mod lua_smells;
#[path = "integration/lua_stress.rs"]
mod lua_stress;
#[path = "integration/module_boundary_thresholds.rs"]
mod module_boundary_thresholds;
#[path = "integration/new_smells_stress.rs"]
mod new_smells_stress;
#[path = "integration/objc_smells.rs"]
mod objc_smells;
#[path = "integration/objc_stress.rs"]
mod objc_stress;
#[path = "integration/path_ignore.rs"]
mod path_ignore;
#[path = "integration/php_smells.rs"]
mod php_smells;
#[path = "integration/php_stress.rs"]
mod php_stress;
#[path = "integration/production_fixtures.rs"]
mod production_fixtures;
#[path = "integration/production_fixtures_new_smells.rs"]
mod production_fixtures_new_smells;
#[path = "integration/python_future_smells.rs"]
mod python_future_smells;
#[path = "integration/python_smells.rs"]
mod python_smells;
#[path = "integration/python_stress.rs"]
mod python_stress;
#[path = "integration/r_smells.rs"]
mod r_smells;
#[path = "integration/r_stress.rs"]
mod r_stress;
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
#[path = "integration/swift_smells.rs"]
mod swift_smells;
#[path = "integration/swift_stress.rs"]
mod swift_stress;
#[path = "integration/tcl_smells.rs"]
mod tcl_smells;
#[path = "integration/tcl_stress.rs"]
mod tcl_stress;
#[path = "integration/typescript_smells.rs"]
mod typescript_smells;
#[path = "integration/typescript_stress.rs"]
mod typescript_stress;
#[path = "integration/zig_smells.rs"]
mod zig_smells;
#[path = "integration/zig_stress.rs"]
mod zig_stress;
