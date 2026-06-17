use std::collections::{HashMap, HashSet};

pub struct VocabModel {
    idf: HashMap<String, f64>,
}

impl VocabModel {
    pub fn build(all_method_vocabs: &[Vec<String>]) -> Self {
        let n = all_method_vocabs.len() as f64;
        let mut df: HashMap<&str, u32> = HashMap::new();
        for vocab in all_method_vocabs {
            for term in vocab.iter().map(String::as_str).collect::<HashSet<&str>>() {
                *df.entry(term).or_insert(0) += 1;
            }
        }
        let idf = df.into_iter().map(|(t, d)| (t.to_string(), ((1.0 + n) / (1.0 + f64::from(d))).ln() + 1.0)).collect();
        Self { idf }
    }

    pub fn cohesion(&self, methods: &[&Vec<String>]) -> Option<f64> {
        let vectors: Vec<HashMap<String, f64>> =
            methods.iter().map(|m| self.tfidf(m)).filter(|v| !v.is_empty()).collect();
        let n = vectors.len();
        if n < 2 {
            return None;
        }
        let mut total = 0.0;
        let mut pairs: u32 = 0;
        for i in 0..n {
            for j in (i + 1)..n {
                total += cosine(&vectors[i], &vectors[j]);
                pairs += 1;
            }
        }
        (pairs > 0).then(|| total / f64::from(pairs))
    }

    fn tfidf(&self, vocab: &[String]) -> HashMap<String, f64> {
        let mut tf: HashMap<&str, u32> = HashMap::new();
        for term in vocab {
            *tf.entry(term.as_str()).or_insert(0) += 1;
        }
        tf.into_iter().filter_map(|(t, c)| self.idf.get(t).map(|idf| (t.to_string(), f64::from(c) * idf))).collect()
    }
}

fn cosine(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let dot: f64 = a.iter().filter_map(|(t, va)| b.get(t).map(|vb| va * vb)).sum();
    let denom = norm(a) * norm(b);
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

fn norm(v: &HashMap<String, f64>) -> f64 {
    v.values().map(|x| x * x).sum::<f64>().sqrt()
}
