use crate::cpg::cfg::Cfg;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CpgMetrics {
    pub cfg: Cfg,
    pub simhash: u64,
    pub surprisal: f64,
}
