use super::graph::{ImportGraph, NodeIndex};

#[derive(Debug, Clone)]
pub struct Scc {
    pub members: Vec<NodeIndex>,
    pub edges: Vec<(NodeIndex, NodeIndex)>,
}

#[doc(hidden)]
#[derive(Debug, Clone, Default)]
pub struct TarjanDiagnostics {
    pub iteration_cap_hit: bool,
    pub max_iters: usize,
}

pub fn find_cycles(graph: &ImportGraph, min_size: u32) -> Vec<Scc> {
    find_cycles_with_max_iters(graph, min_size, default_max_iters(graph)).0
}

#[doc(hidden)]
pub fn find_cycles_with_max_iters(
    graph: &ImportGraph,
    min_size: u32,
    max_iters: usize,
) -> (Vec<Scc>, TarjanDiagnostics) {
    let mut state = TarjanState::new(graph, max_iters);
    state.run();
    let diag = TarjanDiagnostics { iteration_cap_hit: state.iteration_cap_hit, max_iters: state.max_iters };
    (state.into_components(graph, min_size), diag)
}

#[doc(hidden)]
pub fn default_max_iters(graph: &ImportGraph) -> usize {
    let n = graph.registry.count();
    let total_edges: usize = (0..n).map(|i| graph.adjacency.outgoing(NodeIndex(i as u32)).len()).sum();
    n.saturating_mul(2).saturating_add(total_edges).saturating_add(8)
}

const NIL: u32 = u32::MAX;

struct TarjanState<'g> {
    graph: &'g ImportGraph,
    index_counter: u32,
    indices: Vec<u32>,
    lowlinks: Vec<u32>,
    on_stack: Vec<bool>,
    stack: Vec<NodeIndex>,
    components: Vec<Vec<NodeIndex>>,
    max_iters: usize,
    iteration_cap_hit: bool,
}

impl<'g> TarjanState<'g> {
    fn new(graph: &'g ImportGraph, max_iters: usize) -> Self {
        let n = graph.registry.count();
        Self {
            graph,
            index_counter: 0,
            indices: vec![NIL; n],
            lowlinks: vec![NIL; n],
            on_stack: vec![false; n],
            stack: Vec::new(),
            components: Vec::new(),
            max_iters,
            iteration_cap_hit: false,
        }
    }

    fn run(&mut self) {
        let n = self.graph.registry.count() as u32;
        for i in 0..n {
            if self.indices[i as usize] == NIL {
                self.visit_iterative(NodeIndex(i));
            }
        }
    }

    fn visit_iterative(&mut self, root: NodeIndex) {
        let mut frame_stack: Vec<Frame> = vec![self.start_frame(root)];
        let mut steps: usize = 0;
        while !frame_stack.is_empty() && steps < self.max_iters {
            self.advance_top(&mut frame_stack);
            steps += 1;
        }
        if !frame_stack.is_empty() {
            self.iteration_cap_hit = true;
        }
    }

    fn advance_top(&mut self, frame_stack: &mut Vec<Frame>) {
        let top = frame_stack.last_mut().expect("non-empty");
        let succs = self.graph.adjacency.outgoing(top.node);
        if top.next_child >= succs.len() {
            self.unwind_top(frame_stack);
            return;
        }
        let child = succs[top.next_child];
        top.next_child += 1;
        let parent_node = top.node;
        self.descend_or_update(child, parent_node, frame_stack);
    }

    fn unwind_top(&mut self, frame_stack: &mut Vec<Frame>) {
        let popped = frame_stack.pop().expect("frame");
        self.maybe_pop_component(popped.node);
        let Some(parent) = frame_stack.last_mut() else {
            return;
        };
        let popped_low = self.lowlinks[popped.node.0 as usize];
        let parent_low = &mut self.lowlinks[parent.node.0 as usize];
        if popped_low < *parent_low {
            *parent_low = popped_low;
        }
    }

    fn start_frame(&mut self, node: NodeIndex) -> Frame {
        let i = node.0 as usize;
        self.indices[i] = self.index_counter;
        self.lowlinks[i] = self.index_counter;
        self.index_counter += 1;
        self.stack.push(node);
        self.on_stack[i] = true;
        Frame { node, next_child: 0 }
    }

    fn descend_or_update(&mut self, child: NodeIndex, parent_node: NodeIndex, frame_stack: &mut Vec<Frame>) {
        let ci = child.0 as usize;
        if self.indices[ci] == NIL {
            let frame = self.start_frame(child);
            frame_stack.push(frame);
            return;
        }
        if !self.on_stack[ci] {
            return;
        }
        let parent_low = &mut self.lowlinks[parent_node.0 as usize];
        let child_index = self.indices[ci];
        if child_index < *parent_low {
            *parent_low = child_index;
        }
    }

    fn maybe_pop_component(&mut self, node: NodeIndex) {
        let i = node.0 as usize;
        if self.lowlinks[i] != self.indices[i] {
            return;
        }
        let mut component = Vec::new();
        loop {
            let popped = self.stack.pop().expect("stack non-empty inside scc");
            self.on_stack[popped.0 as usize] = false;
            component.push(popped);
            if popped == node {
                break;
            }
        }
        self.components.push(component);
    }

    fn into_components(self, graph: &ImportGraph, min_size: u32) -> Vec<Scc> {
        self.components.into_iter().filter_map(|members| build_scc(graph, members, min_size)).collect()
    }
}

struct Frame {
    node: NodeIndex,
    next_child: usize,
}

fn build_scc(graph: &ImportGraph, members: Vec<NodeIndex>, min_size: u32) -> Option<Scc> {
    let n = members.len();
    if n == 1 && !has_self_loop(graph, members[0]) {
        return None;
    }
    if n >= 2 && (n as u32) < min_size {
        return None;
    }
    let mut sorted_members = members.clone();
    sorted_members.sort_by(|a, b| graph.registry.path_of(*a).cmp(graph.registry.path_of(*b)));
    let edges = collect_intra_edges(graph, &members);
    Some(Scc { members: sorted_members, edges })
}

fn has_self_loop(graph: &ImportGraph, node: NodeIndex) -> bool {
    graph.adjacency.outgoing(node).contains(&node)
}

fn collect_intra_edges(graph: &ImportGraph, members: &[NodeIndex]) -> Vec<(NodeIndex, NodeIndex)> {
    let member_set: std::collections::BTreeSet<NodeIndex> = members.iter().copied().collect();
    let mut edges: Vec<(NodeIndex, NodeIndex)> = Vec::new();
    for &m in members {
        for &t in graph.adjacency.outgoing(m) {
            if member_set.contains(&t) {
                edges.push((m, t));
            }
        }
    }
    edges.sort_by(|a, b| {
        graph
            .registry
            .path_of(a.0)
            .cmp(graph.registry.path_of(b.0))
            .then_with(|| graph.registry.path_of(a.1).cmp(graph.registry.path_of(b.1)))
    });
    edges
}
