use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct WeightedHist {
    pub bins: BTreeMap<u32, Bin>,
    pub n: u64,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct Bin {
    pub count: u64,
    pub weight: f64,
}

impl WeightedHist {
    pub fn observe(&mut self, value: u32, weight: f64) {
        let bin = self.bins.entry(value).or_default();
        bin.count += 1;
        bin.weight += weight;
        self.n += 1;
        self.weight += weight;
    }

    pub fn merge(&mut self, other: &WeightedHist) {
        for (value, bin) in &other.bins {
            let target = self.bins.entry(*value).or_default();
            target.count += bin.count;
            target.weight += bin.weight;
        }
        self.n += other.n;
        self.weight += other.weight;
    }
}

pub fn weighted_quantile(hist: &WeightedHist, p: f64) -> f64 {
    quantile(hist, p, |bin| bin.weight, hist.weight)
}

pub fn count_quantile(hist: &WeightedHist, p: f64) -> f64 {
    quantile(hist, p, |bin| bin.count as f64, hist.n as f64)
}

fn quantile(hist: &WeightedHist, p: f64, mass: impl Fn(&Bin) -> f64, total: f64) -> f64 {
    if total <= 0.0 {
        return 0.0;
    }
    let target = p.clamp(0.0, 1.0) * total;
    let mut cumulative = 0.0;
    for (value, bin) in &hist.bins {
        cumulative += mass(bin);
        if cumulative >= target {
            return f64::from(*value);
        }
    }
    hist.bins.keys().last().map_or(0.0, |v| f64::from(*v))
}
