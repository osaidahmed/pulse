use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct CommunityParams {
    pub resolution: f64,
    pub max_passes: u32,
}

pub struct CommunityResult {
    pub assignment: Vec<usize>,
    pub modularity: f64,
    pub community_count: usize,
}

struct Weighted<'a> {
    adjacency: &'a [Vec<(usize, f64)>],
    degree: Vec<f64>,
    m: f64,
    resolution: f64,
}

struct Partition {
    assignment: Vec<usize>,
    tot: Vec<f64>,
}

pub fn louvain(adjacency: &[Vec<(usize, f64)>], params: CommunityParams) -> CommunityResult {
    let n = adjacency.len();
    let graph = Weighted::new(adjacency, params.resolution);
    if n == 0 || graph.m == 0.0 {
        return CommunityResult { assignment: (0..n).collect(), modularity: 0.0, community_count: n };
    }
    let mut part = Partition::singletons(&graph.degree);
    for _ in 0..params.max_passes {
        if !local_move_pass(&graph, &mut part) {
            break;
        }
    }
    let modularity = graph.modularity(&part);
    let (assignment, community_count) = renumber(&part.assignment);
    CommunityResult { assignment, modularity, community_count }
}

fn local_move_pass(graph: &Weighted, part: &mut Partition) -> bool {
    let mut moved = false;
    for node in 0..part.assignment.len() {
        let from = part.assignment[node];
        part.tot[from] -= graph.degree[node];
        let to = graph.best_community(node, part);
        part.tot[to] += graph.degree[node];
        part.assignment[node] = to;
        if to != from {
            moved = true;
        }
    }
    moved
}

fn renumber(assignment: &[usize]) -> (Vec<usize>, usize) {
    let mut map: BTreeMap<usize, usize> = BTreeMap::new();
    let result: Vec<usize> = assignment
        .iter()
        .map(|&c| {
            let next = map.len();
            *map.entry(c).or_insert(next)
        })
        .collect();
    (result, map.len())
}

impl<'a> Weighted<'a> {
    fn new(adjacency: &'a [Vec<(usize, f64)>], resolution: f64) -> Self {
        let degree: Vec<f64> =
            adjacency.iter().map(|nbrs| nbrs.iter().map(|&(_, w)| w).sum()).collect();
        let m = degree.iter().sum::<f64>() / 2.0;
        Weighted { adjacency, degree, m, resolution }
    }

    fn best_community(&self, node: usize, part: &Partition) -> usize {
        let mut k_in: BTreeMap<usize, f64> = BTreeMap::new();
        for &(nbr, w) in &self.adjacency[node] {
            if nbr != node {
                *k_in.entry(part.assignment[nbr]).or_insert(0.0) += w;
            }
        }
        k_in.entry(part.assignment[node]).or_insert(0.0);
        let factor = self.resolution * self.degree[node] / (2.0 * self.m);
        let mut best = part.assignment[node];
        let mut best_gain = f64::NEG_INFINITY;
        for (&community, &inside) in &k_in {
            let gain = inside - factor * part.tot[community];
            if gain > best_gain {
                best_gain = gain;
                best = community;
            }
        }
        best
    }

    fn modularity(&self, part: &Partition) -> f64 {
        let two_m = 2.0 * self.m;
        let mut internal: BTreeMap<usize, f64> = BTreeMap::new();
        let mut total: BTreeMap<usize, f64> = BTreeMap::new();
        for node in 0..self.adjacency.len() {
            let c = part.assignment[node];
            *total.entry(c).or_insert(0.0) += self.degree[node];
            for &(nbr, w) in &self.adjacency[node] {
                if nbr != node && part.assignment[nbr] == c {
                    *internal.entry(c).or_insert(0.0) += w;
                }
            }
        }
        total
            .iter()
            .map(|(c, &tot)| {
                let inside = internal.get(c).copied().unwrap_or(0.0);
                inside / two_m - self.resolution * (tot / two_m).powi(2)
            })
            .sum()
    }
}

impl Partition {
    fn singletons(degree: &[f64]) -> Self {
        Partition { assignment: (0..degree.len()).collect(), tot: degree.to_vec() }
    }
}
