#![allow(clippy::float_cmp)]

use pulse::audit::cycle_shapes::{classify, shape_label, shape_weight};
use pulse::audit::finding::CycleShape;
use pulse::audit::graph::NodeIndex;

#[test]
fn classify_empty_edges_hits_zero_denominator_ratio() {
    let members = [NodeIndex(0), NodeIndex(1), NodeIndex(2)];
    let edges: [(NodeIndex, NodeIndex); 0] = [];
    let shape = classify(&members, &edges);
    assert_eq!(shape, CycleShape::Circle);
}

#[test]
fn clique_descriptor_weight_and_label() {
    assert_eq!(shape_weight(CycleShape::Clique), 1.0);
    assert_eq!(shape_label(CycleShape::Clique), "clique");
}

#[test]
fn chain_descriptor_weight_and_label() {
    assert_eq!(shape_weight(CycleShape::Chain), 0.5);
    assert_eq!(shape_label(CycleShape::Chain), "chain");
}
