use std::collections::HashSet;

use crate::cpg::cfg::Cfg;
use crate::cpg::defuse::DefUseRecord;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CpgMetrics {
    pub cfg: Cfg,
    pub def_use: Vec<DefUseRecord>,
    pub captured: HashSet<String>,
    pub simhash: u64,
    pub surprisal: f64,
    pub suppress_dead_store: bool,
    pub flag_all_dead_stores: bool,
}
