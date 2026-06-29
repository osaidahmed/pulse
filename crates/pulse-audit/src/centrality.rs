#[derive(Debug, Clone, Copy)]
pub struct PageRankParams {
    pub damping: f64,
    pub max_iters: u32,
    pub epsilon: f64,
}

pub fn pagerank(adjacency: &[Vec<usize>], params: PageRankParams) -> (Vec<f64>, bool) {
    let n = adjacency.len();
    if n == 0 {
        return (Vec::new(), true);
    }
    let base = (1.0 - params.damping) / n as f64;
    let mut pr = vec![1.0 / n as f64; n];
    for _ in 0..params.max_iters {
        let next = pagerank_step(adjacency, &pr, params.damping, base);
        let delta: f64 = pr.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();
        pr = next;
        if delta < params.epsilon {
            return (pr, true);
        }
    }
    (pr, false)
}

fn pagerank_step(adjacency: &[Vec<usize>], pr: &[f64], damping: f64, base: f64) -> Vec<f64> {
    let n = pr.len();
    let dangling: f64 = (0..n).filter(|&u| adjacency[u].is_empty()).map(|u| pr[u]).sum();
    let mut next = vec![base + damping * dangling / n as f64; n];
    for (u, outs) in adjacency.iter().enumerate() {
        if outs.is_empty() {
            continue;
        }
        let share = damping * pr[u] / outs.len() as f64;
        for &v in outs {
            next[v] += share;
        }
    }
    next
}
