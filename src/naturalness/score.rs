use std::collections::BTreeMap;

use crate::thresholds::NaturalnessThresholds;

use super::model::{local_model, NgramModel};

const MIN_PROB: f64 = 1e-12;

struct Cache {
    counts: BTreeMap<u32, u32>,
    size: u32,
    cap: u32,
}

impl Cache {
    fn new(cap: u32) -> Self {
        Self { counts: BTreeMap::new(), size: 0, cap }
    }

    fn prob(&self, token: u32) -> f64 {
        if self.size == 0 {
            return 0.0;
        }
        f64::from(*self.counts.get(&token).unwrap_or(&0)) / f64::from(self.size)
    }

    fn entropy(&self) -> f64 {
        if self.size == 0 {
            return 0.0;
        }
        let total = f64::from(self.size);
        -self
            .counts
            .values()
            .map(|&c| {
                let p = f64::from(c) / total;
                p * p.log2()
            })
            .sum::<f64>()
    }

    fn observe(&mut self, token: u32) {
        if self.size >= self.cap {
            return;
        }
        *self.counts.entry(token).or_insert(0) += 1;
        self.size += 1;
    }
}

pub fn function_surprisal(tokens: &[u32], model: &NgramModel, thr: &NaturalnessThresholds) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }
    let local = local_model(tokens, model.order());
    let mut cache = Cache::new(thr.cache_k);
    let mut total = 0.0f64;
    for (i, &tok) in tokens.iter().enumerate() {
        let p_ngram = model.prob_loo(&tokens[..i], tok, &local);
        let lambda = thr.jm_gamma / (thr.jm_gamma + cache.entropy());
        let p = lambda * p_ngram + (1.0 - lambda) * cache.prob(tok);
        total += -p.max(MIN_PROB).log2();
        cache.observe(tok);
    }
    total / tokens.len() as f64
}
