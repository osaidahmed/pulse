use tree_sitter::Node;

use super::{find_child_by_kind, node_text, track_global_nesting};

pub fn count_boolean_ops(node: Node, cc: &mut u32, op_kinds: &[&str], stop_kinds: &[&str]) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| {
        let kind = child.kind();
        if op_kinds.contains(&kind) {
            *cc += 1;
        } else if !stop_kinds.contains(&kind) {
            count_boolean_ops(child, cc, op_kinds, stop_kinds);
        }
    });
}

pub fn count_cogc_sequences(
    node: Node,
    cogc: &mut u32,
    op_kinds: &[&str],
    stop_kinds: &[&str],
) {
    let mut last_op: Option<&str> = None;
    cogc_walk(node, cogc, &mut last_op, op_kinds, stop_kinds);
}

fn cogc_walk(
    node: Node,
    cogc: &mut u32,
    last_op: &mut Option<&str>,
    op_kinds: &[&str],
    stop_kinds: &[&str],
) {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        let kind = child.kind();
        if op_kinds.contains(&kind) {
            if *last_op != Some(kind) {
                *cogc += 1;
                *last_op = Some(kind);
            }
        } else if !stop_kinds.contains(&kind) {
            cogc_walk(child, cogc, last_op, op_kinds, stop_kinds);
        }
    }
}

pub fn check_condition_complexity_text(
    node: Node,
    source: &str,
    count: &mut u32,
    cond_kinds: &[&str],
) {
    let cond = cond_kinds
        .iter()
        .find_map(|kind| find_child_by_kind(node, kind));
    let Some(cond) = cond else { return };
    let text = node_text(cond, source);
    let ops = text.matches("&&").count() + text.matches("||").count();
    if ops >= 2 {
        *count += 1;
    }
}

pub struct GlobalMetricsConfig<'a> {
    pub cond: &'a [&'a str],
    pub loops: &'a [&'a str],
    pub branches: &'a [&'a str],
    pub recurse: &'a [&'a str],
}

pub fn collect_global_metrics(
    root: Node,
    cond_count: &mut u32,
    max_nesting: &mut u32,
    cfg: &GlobalMetricsConfig,
) {
    let cursor = root.walk();
    let mut child_opt = cursor.node().child(0);
    while let Some(child) = child_opt {
        dispatch_global_child(child, cond_count, max_nesting, cfg);
        child_opt = child.next_sibling();
    }
}

fn dispatch_global_child(
    child: Node,
    cond_count: &mut u32,
    max_nesting: &mut u32,
    cfg: &GlobalMetricsConfig,
) {
    let kind = child.kind();
    if cfg.cond.contains(&kind) {
        *cond_count += 1;
        track_global_nesting(child, max_nesting, cfg.branches);
    } else if cfg.loops.contains(&kind) {
        track_global_nesting(child, max_nesting, cfg.branches);
    } else if cfg.recurse.contains(&kind) {
        collect_global_metrics(child, cond_count, max_nesting, cfg);
    }
}

pub struct BlockWalkCtx<'a> {
    pub source: &'a str,
    pub depth: u32,
    pub state: &'a mut super::WalkState,
}

pub fn walk_block_children(
    node: Node,
    ctx: &mut BlockWalkCtx,
    block_kind: &str,
    walk_fn: fn(Node, &str, u32, &mut super::WalkState),
) {
    let mut child_opt = node.child(0);
    while let Some(child) = child_opt {
        if child.kind() == block_kind {
            walk_fn(child, ctx.source, ctx.depth, ctx.state);
        }
        child_opt = child.next_sibling();
    }
}

pub fn is_catch_body_empty(catch_node: Node, body_kind: &str, pass_kind: Option<&str>) -> bool {
    let Some(body) = find_child_by_kind(catch_node, body_kind) else {
        return true;
    };
    let mut cursor = body.walk();
    let meaningful = body
        .children(&mut cursor)
        .filter(|c| {
            let kind = c.kind();
            kind != body_kind
                && kind != "comment"
                && kind != "line_comment"
                && kind != "block_comment"
                && kind != "{"
                && kind != "}"
                && kind != ":"
                && (pass_kind != Some(kind))
        })
        .count();
    meaningful == 0
}
