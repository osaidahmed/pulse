#![allow(dead_code)]

pub mod cfg;
mod cfg_langs;
mod cfg_nodes;
pub mod cpg_smells;
pub mod defuse;
pub mod reaching;
mod types;

pub use cfg::{build_cfg, CfgLang};
pub use cfg_langs::{C, CPP, CSHARP, GO, JAVA, JAVASCRIPT, KOTLIN, PHP, PYTHON, RUBY, RUST, SWIFT, TYPESCRIPT};
pub use cpg_smells::detect_all;
pub use types::CpgMetrics;
