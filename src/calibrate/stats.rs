use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct WeightedHist {
    pub bins: BTreeMap<u32, Bin>,
    pub n: u64,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, Default)]
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
}

pub fn weighted_quantile(hist: &WeightedHist, p: f64) -> f64 {
    if hist.weight <= 0.0 {
        return 0.0;
    }
    let target = p.clamp(0.0, 1.0) * hist.weight;
    let mut cumulative = 0.0;
    for (value, bin) in &hist.bins {
        cumulative += bin.weight;
        if cumulative >= target {
            return f64::from(*value);
        }
    }
    hist.bins.keys().last().map_or(0.0, |v| f64::from(*v))
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GpdFit {
    pub threshold: f64,
    pub xi: f64,
    pub sigma: f64,
    pub tail_n: u64,
}

pub const GPD_MIN_TAIL_N: u64 = 30;
pub const GPD_MIN_DISTINCT: usize = 10;
const GPD_XI_RANGE: (f64, f64) = (-1.0, 2.0);

pub fn gpd_fit(hist: &WeightedHist, threshold: f64) -> Option<GpdFit> {
    let tail: Vec<(f64, f64)> = hist
        .bins
        .iter()
        .filter(|(value, _)| f64::from(**value) > threshold)
        .map(|(value, bin)| (f64::from(*value) - threshold, bin.weight))
        .collect();
    let tail_n: u64 = hist.bins.iter().filter(|(v, _)| f64::from(**v) > threshold).map(|(_, b)| b.count).sum();
    if tail_n < GPD_MIN_TAIL_N || tail.len() < GPD_MIN_DISTINCT {
        return None;
    }
    let (a0, a1) = probability_weighted_moments(&tail)?;
    let denom = a0 - 2.0 * a1;
    if denom.abs() < 1e-9 {
        return None;
    }
    let xi = 2.0 - a0 / denom;
    let sigma = 2.0 * a0 * a1 / denom;
    if !(GPD_XI_RANGE.0..=GPD_XI_RANGE.1).contains(&xi) || sigma <= 0.0 {
        return None;
    }
    Some(GpdFit { threshold, xi, sigma, tail_n })
}

fn probability_weighted_moments(tail: &[(f64, f64)]) -> Option<(f64, f64)> {
    let total: f64 = tail.iter().map(|(_, w)| w).sum();
    if total <= 0.0 {
        return None;
    }
    let mut cumulative = 0.0;
    let mut a0 = 0.0;
    let mut a1 = 0.0;
    for (excess, weight) in tail {
        let mass = weight / total;
        let mid_cdf = (cumulative + 0.5 * weight) / total;
        a0 += mass * excess;
        a1 += mass * excess * (1.0 - mid_cdf);
        cumulative += weight;
    }
    Some((a0, a1))
}
