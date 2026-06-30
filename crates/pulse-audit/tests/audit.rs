#![allow(dead_code, unused_imports)]

#[path = "audit/common.rs"]
mod common;

#[path = "audit/audit_common.rs"]
mod audit_common;

#[path = "audit/binding_common.rs"]
mod binding_common;

#[path = "audit/sweep_harness.rs"]
mod sweep_harness;

#[path = "audit/audit_abstractness.rs"]
mod audit_abstractness;

#[path = "audit/audit_arch_smells.rs"]
mod audit_arch_smells;

#[path = "audit/audit_binding.rs"]
mod audit_binding;

#[path = "audit/audit_call_extraction_clean.rs"]
mod audit_call_extraction_clean;

#[path = "audit/audit_call_extraction_extended.rs"]
mod audit_call_extraction_extended;

#[path = "audit/audit_call_graph_extraction.rs"]
mod audit_call_graph_extraction;

#[path = "audit/audit_call_graph_skeleton.rs"]
mod audit_call_graph_skeleton;

#[path = "audit/audit_call_graph.rs"]
mod audit_call_graph;

#[path = "audit/audit_centrality.rs"]
mod audit_centrality;

#[path = "audit/audit_class_registry.rs"]
mod audit_class_registry;

#[path = "audit/audit_clones.rs"]
mod audit_clones;

#[path = "audit/audit_cohesion_corpus.rs"]
mod audit_cohesion_corpus;

#[path = "audit/audit_community.rs"]
mod audit_community;

#[path = "audit/audit_component_thresholds.rs"]
mod audit_component_thresholds;

#[path = "audit/audit_components.rs"]
mod audit_components;

#[path = "audit/audit_conceptual_cohesion_e2e.rs"]
mod audit_conceptual_cohesion_e2e;

#[path = "audit/audit_conceptual_cohesion.rs"]
mod audit_conceptual_cohesion;

#[path = "audit/audit_constraint_smells.rs"]
mod audit_constraint_smells;

#[path = "audit/audit_corpus_df.rs"]
mod audit_corpus_df;

#[path = "audit/audit_corpus_stats_unit.rs"]
mod audit_corpus_stats_unit;

#[path = "audit/audit_coverage_disclosure.rs"]
mod audit_coverage_disclosure;

#[path = "audit/audit_cross_language_separation.rs"]
mod audit_cross_language_separation;

#[path = "audit/audit_cycle_shapes.rs"]
mod audit_cycle_shapes;

#[path = "audit/audit_cycles.rs"]
mod audit_cycles;

#[path = "audit/audit_decoupling.rs"]
mod audit_decoupling;

#[path = "audit/audit_definitions.rs"]
mod audit_definitions;

#[path = "audit/audit_deps_reconcile.rs"]
mod audit_deps_reconcile;

#[path = "audit/audit_detector_correctness.rs"]
mod audit_detector_correctness;

#[path = "audit/audit_discovery_extended.rs"]
mod audit_discovery_extended;

#[path = "audit/audit_discovery.rs"]
mod audit_discovery;

#[path = "audit/audit_divergent_change_extended.rs"]
mod audit_divergent_change_extended;

#[path = "audit/audit_divergent_change.rs"]
mod audit_divergent_change;

#[path = "audit/audit_edge_cases.rs"]
mod audit_edge_cases;

#[path = "audit/audit_feature_envy_extended.rs"]
mod audit_feature_envy_extended;

#[path = "audit/audit_feature_envy.rs"]
mod audit_feature_envy;

#[path = "audit/audit_fixture_loader.rs"]
mod audit_fixture_loader;

#[path = "audit/audit_fragmentation.rs"]
mod audit_fragmentation;

#[path = "audit/audit_freshness.rs"]
mod audit_freshness;

#[path = "audit/audit_god_class_extended.rs"]
mod audit_god_class_extended;

#[path = "audit/audit_god_class.rs"]
mod audit_god_class;

#[path = "audit/audit_graph.rs"]
mod audit_graph;

#[path = "audit/audit_ifdef_density.rs"]
mod audit_ifdef_density;

#[path = "audit/audit_ignore.rs"]
mod audit_ignore;

#[path = "audit/audit_import_call_form_extended.rs"]
mod audit_import_call_form_extended;

#[path = "audit/audit_import_command_form_extended.rs"]
mod audit_import_command_form_extended;

#[path = "audit/audit_import_jsts_extended.rs"]
mod audit_import_jsts_extended;

#[path = "audit/audit_import_php_extended.rs"]
mod audit_import_php_extended;

#[path = "audit/audit_import_preprocessor_extended.rs"]
mod audit_import_preprocessor_extended;

#[path = "audit/audit_imports_extended.rs"]
mod audit_imports_extended;

#[path = "audit/audit_imports_lossy.rs"]
mod audit_imports_lossy;

#[path = "audit/audit_imports_suffix_fallback.rs"]
mod audit_imports_suffix_fallback;

#[path = "audit/audit_imports.rs"]
mod audit_imports;

#[path = "audit/audit_inheritance_graph.rs"]
mod audit_inheritance_graph;

#[path = "audit/audit_lang_kinds_table.rs"]
mod audit_lang_kinds_table;

#[path = "audit/audit_languages.rs"]
mod audit_languages;

#[path = "audit/audit_martin.rs"]
mod audit_martin;

#[path = "audit/audit_mcd.rs"]
mod audit_mcd;

#[path = "audit/audit_method_vocab.rs"]
mod audit_method_vocab;

#[path = "audit/audit_named_smell_confidence_routing.rs"]
mod audit_named_smell_confidence_routing;

#[path = "audit/audit_named_smells_e2e.rs"]
mod audit_named_smells_e2e;

#[path = "audit/audit_named_smells_stress.rs"]
mod audit_named_smells_stress;

#[path = "audit/audit_named_smells_threshold.rs"]
mod audit_named_smells_threshold;

#[path = "audit/audit_naturalness.rs"]
mod audit_naturalness;

#[path = "audit/audit_output_advisory.rs"]
mod audit_output_advisory;

#[path = "audit/audit_output_cross_variant.rs"]
mod audit_output_cross_variant;

#[path = "audit/audit_output_divergent_change.rs"]
mod audit_output_divergent_change;

#[path = "audit/audit_output_extended.rs"]
mod audit_output_extended;

#[path = "audit/audit_output_feature_envy.rs"]
mod audit_output_feature_envy;

#[path = "audit/audit_output_god_class.rs"]
mod audit_output_god_class;

#[path = "audit/audit_output_martin.rs"]
mod audit_output_martin;

#[path = "audit/audit_output_named_smells.rs"]
mod audit_output_named_smells;

#[path = "audit/audit_output_parallel_inheritance.rs"]
mod audit_output_parallel_inheritance;

#[path = "audit/audit_output_refused_bequest.rs"]
mod audit_output_refused_bequest;

#[path = "audit/audit_output_remodularization.rs"]
mod audit_output_remodularization;

#[path = "audit/audit_output.rs"]
mod audit_output;

#[path = "audit/audit_package_metrics_skeleton.rs"]
mod audit_package_metrics_skeleton;

#[path = "audit/audit_parallel_inheritance_extended.rs"]
mod audit_parallel_inheritance_extended;

#[path = "audit/audit_parallel_inheritance.rs"]
mod audit_parallel_inheritance;

#[path = "audit/audit_per_language_deep.rs"]
mod audit_per_language_deep;

#[path = "audit/audit_pipeline_integration.rs"]
mod audit_pipeline_integration;

#[path = "audit/audit_reflexion.rs"]
mod audit_reflexion;

#[path = "audit/audit_refused_bequest_extended.rs"]
mod audit_refused_bequest_extended;

#[path = "audit/audit_refused_bequest.rs"]
mod audit_refused_bequest;

#[path = "audit/audit_remaining_coverage.rs"]
mod audit_remaining_coverage;

#[path = "audit/audit_remodularization.rs"]
mod audit_remodularization;

#[path = "audit/audit_scoring_extended.rs"]
mod audit_scoring_extended;

#[path = "audit/audit_scoring.rs"]
mod audit_scoring;

#[path = "audit/audit_shotgun_surgery.rs"]
mod audit_shotgun_surgery;

#[path = "audit/audit_suppression.rs"]
mod audit_suppression;

#[path = "audit/audit_taint.rs"]
mod audit_taint;

#[path = "audit/audit_test_roots.rs"]
mod audit_test_roots;

#[path = "audit/audit_vendor_filter_unit.rs"]
mod audit_vendor_filter_unit;

#[path = "audit/audit_vuln_clones.rs"]
mod audit_vuln_clones;

#[path = "audit/audit_walker_extended.rs"]
mod audit_walker_extended;

#[path = "audit/audit_walker.rs"]
mod audit_walker;

#[path = "audit/audit_ws1_stats.rs"]
mod audit_ws1_stats;

#[path = "audit/binding_languages.rs"]
mod binding_languages;

#[path = "audit/buildmeta_corpus.rs"]
mod buildmeta_corpus;

#[path = "audit/corpus_df_sweep.rs"]
mod corpus_df_sweep;

#[path = "audit/cov_audit_compound_severity.rs"]
mod cov_audit_compound_severity;

#[path = "audit/cov_audit_cycle_shapes.rs"]
mod cov_audit_cycle_shapes;

#[path = "audit/cov_audit_expression_filter.rs"]
mod cov_audit_expression_filter;

#[path = "audit/cov_audit_import_call_form.rs"]
mod cov_audit_import_call_form;

#[path = "audit/cov_audit_misc.rs"]
mod cov_audit_misc;

#[path = "audit/cov_audit_named_smells_confidence.rs"]
mod cov_audit_named_smells_confidence;

#[path = "audit/cov_binding_cfamily.rs"]
mod cov_binding_cfamily;

#[path = "audit/cov_binding_jvm.rs"]
mod cov_binding_jvm;

#[path = "audit/cov_binding_misc.rs"]
mod cov_binding_misc;

#[path = "audit/cov_config_empty_branches.rs"]
mod cov_config_empty_branches;

#[path = "audit/cov_detector_parallel_inheritance_workaround.rs"]
mod cov_detector_parallel_inheritance_workaround;

#[path = "audit/cov_import_php.rs"]
mod cov_import_php;

#[path = "audit/cov_import_preprocessor_branches.rs"]
mod cov_import_preprocessor_branches;

#[path = "audit/cov_network.rs"]
mod cov_network;

#[path = "audit/cov_output_arch.rs"]
mod cov_output_arch;

#[path = "audit/cov_output_deps_arch.rs"]
mod cov_output_deps_arch;

#[path = "audit/cov_taint_state_merge.rs"]
mod cov_taint_state_merge;

#[path = "audit/cov2_audit_small.rs"]
mod cov2_audit_small;

#[path = "audit/cov2_binding_go.rs"]
mod cov2_binding_go;

#[path = "audit/cov2_binding_jvm_swift_cs.rs"]
mod cov2_binding_jvm_swift_cs;

#[path = "audit/cov2_binding_objc_d.rs"]
mod cov2_binding_objc_d;

#[path = "audit/cov2_freshness_registry.rs"]
mod cov2_freshness_registry;

#[path = "audit/defensive_caps.rs"]
mod defensive_caps;

#[path = "audit/walk_field_accesses_foreign_extended.rs"]
mod walk_field_accesses_foreign_extended;

#[path = "audit/walk_field_accesses_foreign.rs"]
mod walk_field_accesses_foreign;

#[path = "audit/walk_parent_class.rs"]
mod walk_parent_class;

#[path = "audit/walker_stack_guard.rs"]
mod walker_stack_guard;
