pub mod cfg;
mod cfg_langs;
mod cfg_nodes;
pub mod cpg_smells;
pub mod defuse;
pub mod implicit_return;
pub(crate) mod nested;
pub mod reaching;
mod types;

pub use cfg::{build_cfg, CfgLang};
pub use cfg_langs::{covers, C, CPP, CSHARP, GO, JAVA, JAVASCRIPT, KOTLIN, PHP, PYTHON, RUBY, RUST, SWIFT, TYPESCRIPT};
pub use cpg_smells::detect_all;
pub use types::CpgMetrics;
