#![allow(dead_code, unused_imports)]

#[path = "audit/common.rs"]
mod common;

#[path = "audit/audit_common.rs"]
mod audit_common;

#[path = "audit/binding_common.rs"]
mod binding_common;

#[path = "audit/sweep_harness.rs"]
mod sweep_harness;

#[path = "audit/abstractness.rs"]
mod abstractness;

#[path = "audit/arch_smells.rs"]
mod arch_smells;

#[path = "audit/binding.rs"]
mod binding;

#[path = "audit/call_extraction_clean.rs"]
mod call_extraction_clean;

#[path = "audit/call_extraction_extended.rs"]
mod call_extraction_extended;

#[path = "audit/call_graph_extraction.rs"]
mod call_graph_extraction;

#[path = "audit/call_graph_skeleton.rs"]
mod call_graph_skeleton;

#[path = "audit/call_graph.rs"]
mod call_graph;

#[path = "audit/centrality.rs"]
mod centrality;

#[path = "audit/class_registry.rs"]
mod class_registry;

#[path = "audit/clones.rs"]
mod clones;

#[path = "audit/cohesion_corpus.rs"]
mod cohesion_corpus;

#[path = "audit/community.rs"]
mod community;

#[path = "audit/component_thresholds.rs"]
mod component_thresholds;

#[path = "audit/components.rs"]
mod components;

#[path = "audit/conceptual_cohesion_e2e.rs"]
mod conceptual_cohesion_e2e;

#[path = "audit/conceptual_cohesion.rs"]
mod conceptual_cohesion;

#[path = "audit/constraint_smells.rs"]
mod constraint_smells;

#[path = "audit/corpus_df.rs"]
mod corpus_df;

#[path = "audit/corpus_stats_unit.rs"]
mod corpus_stats_unit;

#[path = "audit/coverage_disclosure.rs"]
mod coverage_disclosure;

#[path = "audit/cross_language_separation.rs"]
mod cross_language_separation;

#[path = "audit/cycle_shapes.rs"]
mod cycle_shapes;

#[path = "audit/cycles.rs"]
mod cycles;

#[path = "audit/decoupling.rs"]
mod decoupling;

#[path = "audit/definitions.rs"]
mod definitions;

#[path = "audit/deps_reconcile.rs"]
mod deps_reconcile;

#[path = "audit/detector_correctness.rs"]
mod detector_correctness;

#[path = "audit/discovery_extended.rs"]
mod discovery_extended;

#[path = "audit/discovery.rs"]
mod discovery;

#[path = "audit/divergent_change_extended.rs"]
mod divergent_change_extended;

#[path = "audit/divergent_change.rs"]
mod divergent_change;

#[path = "audit/edge_cases.rs"]
mod edge_cases;

#[path = "audit/feature_envy_extended.rs"]
mod feature_envy_extended;

#[path = "audit/feature_envy.rs"]
mod feature_envy;

#[path = "audit/fixture_loader.rs"]
mod fixture_loader;

#[path = "audit/fragmentation.rs"]
mod fragmentation;

#[path = "audit/freshness.rs"]
mod freshness;

#[path = "audit/god_class_extended.rs"]
mod god_class_extended;

#[path = "audit/god_class.rs"]
mod god_class;

#[path = "audit/graph.rs"]
mod graph;

#[path = "audit/ifdef_density.rs"]
mod ifdef_density;

#[path = "audit/ignore.rs"]
mod ignore;

#[path = "audit/import_call_form_extended.rs"]
mod import_call_form_extended;

#[path = "audit/import_command_form_extended.rs"]
mod import_command_form_extended;

#[path = "audit/import_jsts_extended.rs"]
mod import_jsts_extended;

#[path = "audit/import_php_extended.rs"]
mod import_php_extended;

#[path = "audit/import_preprocessor_extended.rs"]
mod import_preprocessor_extended;

#[path = "audit/imports_extended.rs"]
mod imports_extended;

#[path = "audit/imports_lossy.rs"]
mod imports_lossy;

#[path = "audit/imports_suffix_fallback.rs"]
mod imports_suffix_fallback;

#[path = "audit/imports.rs"]
mod imports;

#[path = "audit/inheritance_graph.rs"]
mod inheritance_graph;

#[path = "audit/lang_kinds_table.rs"]
mod lang_kinds_table;

#[path = "audit/languages.rs"]
mod languages;

#[path = "audit/martin.rs"]
mod martin;

#[path = "audit/mcd.rs"]
mod mcd;

#[path = "audit/method_vocab.rs"]
mod method_vocab;

#[path = "audit/named_smell_confidence_routing.rs"]
mod named_smell_confidence_routing;

#[path = "audit/named_smells_e2e.rs"]
mod named_smells_e2e;

#[path = "audit/named_smells_stress.rs"]
mod named_smells_stress;

#[path = "audit/named_smells_threshold.rs"]
mod named_smells_threshold;

#[path = "audit/naturalness.rs"]
mod naturalness;

#[path = "audit/output_advisory.rs"]
mod output_advisory;

#[path = "audit/output_cross_variant.rs"]
mod output_cross_variant;

#[path = "audit/output_divergent_change.rs"]
mod output_divergent_change;

#[path = "audit/output_extended.rs"]
mod output_extended;

#[path = "audit/output_feature_envy.rs"]
mod output_feature_envy;

#[path = "audit/output_god_class.rs"]
mod output_god_class;

#[path = "audit/output_martin.rs"]
mod output_martin;

#[path = "audit/output_named_smells.rs"]
mod output_named_smells;

#[path = "audit/output_parallel_inheritance.rs"]
mod output_parallel_inheritance;

#[path = "audit/output_refused_bequest.rs"]
mod output_refused_bequest;

#[path = "audit/output_remodularization.rs"]
mod output_remodularization;

#[path = "audit/output.rs"]
mod output;

#[path = "audit/package_metrics_skeleton.rs"]
mod package_metrics_skeleton;

#[path = "audit/parallel_inheritance_extended.rs"]
mod parallel_inheritance_extended;

#[path = "audit/parallel_inheritance.rs"]
mod parallel_inheritance;

#[path = "audit/per_language_deep.rs"]
mod per_language_deep;

#[path = "audit/pipeline_integration.rs"]
mod pipeline_integration;

#[path = "audit/reflexion.rs"]
mod reflexion;

#[path = "audit/refused_bequest_extended.rs"]
mod refused_bequest_extended;

#[path = "audit/refused_bequest.rs"]
mod refused_bequest;

#[path = "audit/remaining_coverage.rs"]
mod remaining_coverage;

#[path = "audit/remodularization.rs"]
mod remodularization;

#[path = "audit/scoring_extended.rs"]
mod scoring_extended;

#[path = "audit/scoring.rs"]
mod scoring;

#[path = "audit/shotgun_surgery.rs"]
mod shotgun_surgery;

#[path = "audit/suppression.rs"]
mod suppression;

#[path = "audit/taint.rs"]
mod taint;

#[path = "audit/test_roots.rs"]
mod test_roots;

#[path = "audit/vendor_filter_unit.rs"]
mod vendor_filter_unit;

#[path = "audit/vuln_clones.rs"]
mod vuln_clones;

#[path = "audit/walker_extended.rs"]
mod walker_extended;

#[path = "audit/walker.rs"]
mod walker;

#[path = "audit/ws1_stats.rs"]
mod ws1_stats;

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

#[path = "audit/serde_roundtrip.rs"]
mod serde_roundtrip;
#[path = "audit/walker_stack_guard.rs"]
mod walker_stack_guard;
