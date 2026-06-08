#![allow(dead_code)]

pub mod cfg;
pub mod cpg_smells;
pub mod defuse;
pub mod reaching;
mod types;

pub use cfg::{build_cfg, CfgLang, C, CPP, CSHARP, GO, JAVA, JAVASCRIPT, PYTHON, RUST, TYPESCRIPT};
pub use cpg_smells::detect_all;
pub use types::CpgMetrics;
