use std::collections::HashSet;

pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SwapConfig {
    pub samples: usize,
    pub step_multiplier: usize,
    pub seed: u64,
    pub max_cols: usize,
}

pub struct IncidenceMatrix {
    pub rows: usize,
    pub cols: usize,
    pub edges: Vec<(usize, usize)>,
    present: HashSet<(usize, usize)>,
}

impl IncidenceMatrix {
    pub fn new(rows: usize, cols: usize, edges: &[(usize, usize)]) -> Self {
        let present: HashSet<(usize, usize)> = edges.iter().copied().collect();
        let edges: Vec<(usize, usize)> = present.iter().copied().collect();
        Self { rows, cols, edges, present }
    }
}

pub fn p_values(matrix: &IncidenceMatrix, config: SwapConfig) -> Vec<f64> {
    if matrix.cols == 0 || matrix.cols > config.max_cols || matrix.edges.len() < 2 {
        return vec![0.5; matrix.cols];
    }
    let observed = cooccurrence_stats(&matrix.edges, matrix.cols);
    let mut below = vec![0u32; matrix.cols];
    let mut above = vec![0u32; matrix.cols];
    let mut rng = SplitMix64::new(config.seed);
    for _ in 0..config.samples {
        let randomized = swap_randomize(matrix, &mut rng, config.step_multiplier);
        let stats = cooccurrence_stats(&randomized, matrix.cols);
        accumulate(&observed, &stats, &mut below, &mut above);
    }
    empirical_pvalues(&below, &above, config.samples)
}

fn accumulate(observed: &[u32], stats: &[u32], below: &mut [u32], above: &mut [u32]) {
    for (c, &obs) in observed.iter().enumerate() {
        if stats[c] < obs {
            below[c] += 1;
        } else if stats[c] > obs {
            above[c] += 1;
        }
    }
}

fn empirical_pvalues(below: &[u32], above: &[u32], samples: usize) -> Vec<f64> {
    let k = samples as f64;
    below.iter().zip(above).map(|(&b, &a)| (f64::from(b.min(a)) + 1.0) / (k + 1.0)).collect()
}

fn swap_randomize(matrix: &IncidenceMatrix, rng: &mut SplitMix64, step_multiplier: usize) -> Vec<(usize, usize)> {
    let mut edges = matrix.edges.clone();
    let mut present = matrix.present.clone();
    let steps = step_multiplier.saturating_mul(edges.len());
    for _ in 0..steps {
        try_swap(&mut edges, &mut present, rng);
    }
    edges
}

fn try_swap(edges: &mut [(usize, usize)], present: &mut HashSet<(usize, usize)>, rng: &mut SplitMix64) {
    let first = rng.below(edges.len());
    let second = rng.below(edges.len());
    let (r1, c1) = edges[first];
    let (r2, c2) = edges[second];
    if r1 == r2 || c1 == c2 {
        return;
    }
    if present.contains(&(r1, c2)) || present.contains(&(r2, c1)) {
        return;
    }
    present.remove(&(r1, c1));
    present.remove(&(r2, c2));
    present.insert((r1, c2));
    present.insert((r2, c1));
    edges[first] = (r1, c2);
    edges[second] = (r2, c1);
}

fn cooccurrence_stats(edges: &[(usize, usize)], cols: usize) -> Vec<u32> {
    let col_rows = build_col_rows(edges, cols);
    let mut stats = vec![0u32; cols];
    for a in 0..cols {
        for b in (a + 1)..cols {
            if shared_count(&col_rows[a], &col_rows[b]) >= 2 {
                stats[a] += 1;
                stats[b] += 1;
            }
        }
    }
    stats
}

fn build_col_rows(edges: &[(usize, usize)], cols: usize) -> Vec<Vec<usize>> {
    let mut col_rows = vec![Vec::new(); cols];
    for &(row, col) in edges {
        col_rows[col].push(row);
    }
    for rows in &mut col_rows {
        rows.sort_unstable();
    }
    col_rows
}

fn shared_count(a: &[usize], b: &[usize]) -> usize {
    let mut count = 0;
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                count += 1;
                i += 1;
                j += 1;
            }
        }
    }
    count
}
