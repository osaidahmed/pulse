#![allow(dead_code, unused_imports)]

#[path = "syntax/common.rs"]
mod common;

#[path = "syntax/audit_fingerprint_extended.rs"]
mod audit_fingerprint_extended;

#[path = "syntax/audit_fingerprint.rs"]
mod audit_fingerprint;

#[path = "syntax/audit_seeded_fingerprint.rs"]
mod audit_seeded_fingerprint;

#[path = "syntax/cov_r_walker_assignment_forms.rs"]
mod cov_r_walker_assignment_forms;

#[path = "syntax/cov_typescript_walker.rs"]
mod cov_typescript_walker;

#[path = "syntax/cov_walk_cpp.rs"]
mod cov_walk_cpp;

#[path = "syntax/cov_walk_csharp.rs"]
mod cov_walk_csharp;

#[path = "syntax/csharp_walker_extended.rs"]
mod csharp_walker_extended;

#[path = "syntax/fuzz_corpus.rs"]
mod fuzz_corpus;

#[path = "syntax/langkinds.rs"]
mod langkinds;

#[path = "syntax/multi_lang_walker_extended.rs"]
mod multi_lang_walker_extended;

#[path = "syntax/parity.rs"]
mod parity;

#[path = "syntax/parse_refactor.rs"]
mod parse_refactor;

#[path = "syntax/php_walker_extended.rs"]
mod php_walker_extended;

#[path = "syntax/property_invariants.rs"]
mod property_invariants;

#[path = "syntax/python_walker_extended.rs"]
mod python_walker_extended;

#[path = "syntax/r_walker_extended.rs"]
mod r_walker_extended;

#[path = "syntax/simhash_clones.rs"]
mod simhash_clones;

#[path = "syntax/walker_structural_completeness.rs"]
mod walker_structural_completeness;

#[path = "syntax/test_file_detection.rs"]
mod test_file_detection;
