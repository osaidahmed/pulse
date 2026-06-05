use std::collections::HashMap;

const NGRAM_LAMBDA: f64 = 0.5;
const MAX_ORDER: usize = 8;

#[derive(Default)]
pub struct Interner {
    ids: HashMap<String, u32>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, token: &str) -> u32 {
        if let Some(&id) = self.ids.get(token) {
            return id;
        }
        let id = self.ids.len() as u32;
        self.ids.insert(token.to_string(), id);
        id
    }

    pub fn vocab(&self) -> usize {
        self.ids.len()
    }
}

pub struct NgramModel {
    order: usize,
    total: u64,
    counts: Vec<HashMap<u64, u32>>,
    vocab: usize,
}

pub fn train(docs: &[Vec<u32>], order: usize, vocab: usize) -> NgramModel {
    let order = order.clamp(1, MAX_ORDER);
    let mut counts = vec![HashMap::new(); order + 1];
    let mut total = 0u64;
    for doc in docs {
        total += doc.len() as u64;
        for (k, table) in counts.iter_mut().enumerate().skip(1) {
            for gram in doc.windows(k) {
                *table.entry(gram_hash(gram)).or_insert(0) += 1;
            }
        }
    }
    NgramModel { order, total, counts, vocab: vocab.max(1) }
}

pub fn local_model(tokens: &[u32], order: usize) -> NgramModel {
    let docs = [tokens.to_vec()];
    train(&docs, order, 1)
}

impl NgramModel {
    pub fn order(&self) -> usize {
        self.order
    }

    pub fn prob_loo(&self, history: &[u32], token: u32, local: &NgramModel) -> f64 {
        let mut p = 1.0 / self.vocab as f64;
        for k in 1..=self.order {
            let ctx_len = k - 1;
            if history.len() < ctx_len {
                break;
            }
            let ctx = &history[history.len() - ctx_len..];
            let ctx_count = self.ctx_count_loo(ctx, local);
            if ctx_count == 0 {
                continue;
            }
            p = NGRAM_LAMBDA * self.mle_loo(ctx, token, local, ctx_count) + (1.0 - NGRAM_LAMBDA) * p;
        }
        p
    }

    fn ctx_count_loo(&self, ctx: &[u32], local: &NgramModel) -> u64 {
        if ctx.is_empty() {
            return self.total.saturating_sub(local.total);
        }
        let key = gram_hash(ctx);
        let global = u64::from(*self.counts[ctx.len()].get(&key).unwrap_or(&0));
        let own = u64::from(*local.counts[ctx.len()].get(&key).unwrap_or(&0));
        global.saturating_sub(own)
    }

    fn mle_loo(&self, ctx: &[u32], token: u32, local: &NgramModel, ctx_count: u64) -> f64 {
        let mut gram = ctx.to_vec();
        gram.push(token);
        let key = gram_hash(&gram);
        let global = u64::from(*self.counts[gram.len()].get(&key).unwrap_or(&0));
        let own = u64::from(*local.counts[gram.len()].get(&key).unwrap_or(&0));
        global.saturating_sub(own) as f64 / ctx_count as f64
    }
}

fn gram_hash(ids: &[u32]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &id in ids {
        h ^= u64::from(id);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
