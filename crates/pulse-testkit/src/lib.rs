//! Shared, binary-free test helpers for the pulse workspace crates.
//! Binary-driven helpers (run_check, lang_helpers!) stay in the umbrella test binary.
#![allow(dead_code)]

use std::path::PathBuf;

pub use pulse_thresholds::Thresholds;

pub fn t() -> Thresholds {
    Thresholds::default()
}

/// Walks up from this crate's manifest dir to the workspace root (the Cargo.toml
/// declaring `[workspace]`), so per-crate tests can reach the shared fixtures.
pub fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let cargo = dir.join("Cargo.toml");
        if cargo.exists() && std::fs::read_to_string(&cargo).is_ok_and(|s| s.contains("[workspace]")) {
            return dir;
        }
        if !dir.pop() {
            return PathBuf::from(".");
        }
    }
}

pub fn fixtures_dir(lang: &str) -> PathBuf {
    workspace_root().join("tests").join("fixtures").join(lang)
}

pub fn fn_padding() -> usize {
    t().function.fn_loc_warning as usize + 20
}

pub fn file_padding() -> usize {
    t().module.file_loc_warning as usize + 100
}

pub fn cc_branches() -> usize {
    t().function.cc_warning as usize + 1
}

pub fn declarations_above() -> usize {
    t().module.max_declarations as usize + 5
}

pub fn functions_above() -> usize {
    t().module.file_function_count as usize + 1
}

pub fn struct_fields_at() -> usize {
    t().module.max_struct_fields as usize
}

pub fn struct_fields_above() -> usize {
    t().module.max_struct_fields as usize + 3
}

pub fn asserts_at() -> usize {
    t().analysis.consecutive_asserts_max as usize
}

pub fn asserts_above() -> usize {
    t().analysis.consecutive_asserts_max as usize + 5
}

pub fn large_fn_lines() -> usize {
    t().module.large_fn_loc as usize + 15
}

pub fn embedded_lines_above() -> usize {
    t().function.embedded_block_loc as usize + 5
}

pub fn args_above() -> usize {
    t().function.arg_max as usize + 1
}
