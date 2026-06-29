use super::finding::CycleShape;
use super::graph::NodeIndex;

const CLIQUE_DENSITY: f64 = 0.75;
const STAR_HUB_FRACTION: f64 = 0.4;

pub fn classify(members: &[NodeIndex], edges: &[(NodeIndex, NodeIndex)]) -> CycleShape {
    classify_counts(members.len(), edges.len(), max_degree(members, edges))
}

fn classify_counts(n: usize, edge_count: usize, max_deg: usize) -> CycleShape {
    if n <= 2 {
        return CycleShape::Tiny;
    }
    if ratio(edge_count, n * (n - 1)) >= CLIQUE_DENSITY {
        return CycleShape::Clique;
    }
    if ratio(max_deg, 2 * edge_count) >= STAR_HUB_FRACTION {
        return CycleShape::Star;
    }
    if edge_count <= n + n / 4 {
        return CycleShape::Circle;
    }
    CycleShape::Chain
}

fn max_degree(members: &[NodeIndex], edges: &[(NodeIndex, NodeIndex)]) -> usize {
    members.iter().map(|&m| degree(m, edges)).max().unwrap_or(0)
}

fn degree(node: NodeIndex, edges: &[(NodeIndex, NodeIndex)]) -> usize {
    edges.iter().filter(|(a, b)| *a == node || *b == node).count()
}

fn ratio(num: usize, den: usize) -> f64 {
    if den == 0 {
        0.0
    } else {
        num as f64 / den as f64
    }
}

fn descriptor(shape: CycleShape) -> (f64, &'static str) {
    match shape {
        CycleShape::Clique => (1.0, "clique"),
        CycleShape::Star => (0.7, "star"),
        CycleShape::Chain => (0.5, "chain"),
        CycleShape::Tiny => (0.4, "tiny"),
        CycleShape::Circle => (0.3, "circle"),
    }
}

pub fn shape_weight(shape: CycleShape) -> f64 {
    descriptor(shape).0
}

pub fn shape_label(shape: CycleShape) -> &'static str {
    descriptor(shape).1
}
