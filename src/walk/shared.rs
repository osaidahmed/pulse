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

pub type WalkFn = fn(Node, &str, u32, &mut super::WalkState);

pub struct BranchKinds {
    pub blocks: &'static [&'static str],
    pub else_clause: &'static str,
    pub catch_clause: Option<&'static str>,
    pub finally_clause: Option<&'static str>,
    pub catch_body_kind: &'static str,
}

pub struct BranchHandlers {
    pub kinds: &'static BranchKinds,
    pub walk_body: WalkFn,
    pub walk_else: WalkFn,
}

pub struct ElseBranchCfg {
    pub block_kind: &'static str,
    pub if_kind: &'static str,
    pub cond_kinds: &'static [&'static str],
    pub bool_ops: &'static [&'static str],
    pub bool_stops: &'static [&'static str],
}

pub struct ElseHandlers {
    pub cfg: &'static ElseBranchCfg,
    pub walk_body: WalkFn,
    pub walk_children: WalkFn,
}

pub fn walk_branches(node: Node, ctx: &mut BlockWalkCtx, h: &BranchHandlers) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if h.kinds.blocks.contains(&kind) {
            push_nested(child, ctx, h.walk_body);
        } else if kind == h.kinds.else_clause {
            (h.walk_else)(child, ctx.source, ctx.depth, ctx.state);
        } else if h.kinds.catch_clause == Some(kind) {
            handle_catch_block(child, ctx, h.kinds.catch_body_kind, h.walk_body);
        } else if h.kinds.finally_clause == Some(kind) {
            walk_block_children(child, ctx, h.kinds.catch_body_kind, h.walk_body);
        }
    }
}

pub fn walk_else_branch(node: Node, ctx: &mut BlockWalkCtx, h: &ElseHandlers) {
    let block_kind = h.cfg.block_kind;
    let if_kind = h.cfg.if_kind;
    let mut cursor = node.walk();
    let kids: Vec<Node> = node.children(&mut cursor).collect();
    for child in kids {
        if child.kind() == block_kind {
            ctx.state.track_cogc_flat();
            push_nested(child, ctx, h.walk_body);
        } else if child.kind() == if_kind {
            apply_else_if(child, ctx, h);
        }
    }
}

fn apply_else_if(child: Node, ctx: &mut BlockWalkCtx, h: &ElseHandlers) {
    ctx.state.cc += 1;
    ctx.state.track_cogc_branch();
    count_boolean_ops(child, &mut ctx.state.cc, h.cfg.bool_ops, h.cfg.bool_stops);
    count_cogc_sequences(child, &mut ctx.state.cogc, h.cfg.bool_ops, h.cfg.bool_stops);
    check_condition_complexity_text(child, ctx.source, &mut ctx.state.compound_condition_count, h.cfg.cond_kinds);
    (h.walk_children)(child, ctx.source, ctx.depth, ctx.state);
}

fn push_nested(child: Node, ctx: &mut BlockWalkCtx, walk_body_fn: WalkFn) {
    let saved = ctx.state.cogc_nesting;
    ctx.state.cogc_nesting += 1;
    walk_body_fn(child, ctx.source, ctx.depth, ctx.state);
    ctx.state.cogc_nesting = saved;
}

fn handle_catch_block(child: Node, ctx: &mut BlockWalkCtx, body_kind: &str, walk_body_fn: WalkFn) {
    ctx.state.cc += 1;
    ctx.state.track_cogc_branch();
    if is_catch_body_empty(child, body_kind, None) {
        ctx.state.empty_catch_count += 1;
    }
    walk_block_children(child, ctx, body_kind, walk_body_fn);
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
