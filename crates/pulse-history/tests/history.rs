#![allow(dead_code, unused_imports)]

#[path = "history/common.rs"]
mod common;
#[path = "history/history_arch_trend.rs"]
mod history_arch_trend;
#[path = "history/history_build_co_change.rs"]
mod history_build_co_change;
#[path = "history/history_co_change.rs"]
mod history_co_change;
#[path = "history/history_common.rs"]
mod history_common;
#[path = "history/history_config_overrides.rs"]
mod history_config_overrides;
#[path = "history/history_contributors.rs"]
mod history_contributors;
#[path = "history/history_e2e_scenarios.rs"]
mod history_e2e_scenarios;
#[path = "history/history_edge_cases.rs"]
mod history_edge_cases;
#[path = "history/history_edges.rs"]
mod history_edges;
#[path = "history/history_finding.rs"]
mod history_finding;
#[path = "history/history_hist_smells.rs"]
mod history_hist_smells;
#[path = "history/history_hotspots.rs"]
mod history_hotspots;
#[path = "history/history_languages_static_link.rs"]
mod history_languages_static_link;
#[path = "history/history_negative_paths.rs"]
mod history_negative_paths;
#[path = "history/history_output.rs"]
mod history_output;
#[path = "history/history_szz.rs"]
mod history_szz;

#[path = "history/audit_common.rs"]
mod audit_common;

#[path = "history/history_git.rs"]
mod history_git;

#[path = "history/history_orchestrator.rs"]
mod history_orchestrator;

#[path = "history/cov_history_git_subprocess.rs"]
mod cov_history_git_subprocess;

#[path = "history/cov_history_output_blob_shotgun.rs"]
mod cov_history_output_blob_shotgun;

#[path = "history/jit_calibration.rs"]
mod jit_calibration;

#[path = "history/hist_crossval.rs"]
mod hist_crossval;
