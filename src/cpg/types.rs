use crate::cpg::cfg::Cfg;
use crate::cpg::defuse::DefUseRecord;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CpgMetrics {
    pub cfg: Cfg,
    pub def_use: Vec<DefUseRecord>,
    pub simhash: u64,
    pub surprisal: f64,
}
