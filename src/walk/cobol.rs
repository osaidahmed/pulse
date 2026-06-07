use tree_sitter::{Node, Tree};

use super::counters::count_short_variables;
use super::{
    compute_skeleton_hash, compute_structural_fingerprint, count_code_lines, count_distinct_node_kinds_multi,
    find_child_by_kind, node_text, track_embedded_block, FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["*>", "*"];

struct ParaInfo<'a> {
    body: &'a [Node<'a>],
    source: &'a str,
    name: &'a str,
    start_line: u32,
    end_line: u32,
}

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);
    let prog = find_child_by_kind(root, "program_definition").unwrap_or(root);

    let declaration_count =
        find_child_by_kind(prog, "data_division").map_or(0, |dd| count_descendants(dd, "data_description"));

    let mut functions = Vec::new();
    let mut global_cond: u32 = 0;
    let mut global_nest: u32 = 0;

    if let Some(pd) = find_child_by_kind(prog, "procedure_division") {
        let flat = flatten_children(pd);
        collect_paragraphs(&flat, source, &mut functions, &mut global_cond, &mut global_nest);
    }

    let module = ModuleMetrics {
        total_loc,
        total_functions: functions.len() as u32,
        sum_cc: functions.iter().map(|f| f.cc).sum(),
        global_conditional_count: global_cond,
        global_max_nesting: global_nest,
        declaration_count,
        struct_fields: Vec::new(),
    };
    FileMetrics { functions, module }
}

fn flatten_children(node: Node) -> Vec<Node> {
    let mut out = Vec::new();
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        let mut cursor = current.walk();
        let children: Vec<Node> = current.children(&mut cursor).filter(Node::is_named).collect();
        for child in children.into_iter().rev() {
            if child.kind().ends_with("_statement") || is_control_node(child.kind()) {
                out.push(child);
            } else {
                stack.push(child);
            }
        }
    }
    out.sort_by_key(Node::start_byte);
    out
}

fn is_control_node(kind: &str) -> bool {
    matches!(
        kind,
        "paragraph_header"
            | "section_header"
            | "if_header"
            | "else_if_header"
            | "else_header"
            | "END_IF"
            | "END_EVALUATE"
            | "END_PERFORM"
            | "evaluate_header"
            | "when"
            | "when_other"
            | "perform_statement_loop"
            | "perform_statement_call_proc"
            | "period"
    )
}

fn collect_paragraphs(
    children: &[Node],
    source: &str,
    functions: &mut Vec<FunctionMetrics>,
    global_cond: &mut u32,
    global_nest: &mut u32,
) {
    let mut section: Option<String> = None;
    let mut para: Option<(String, u32, usize)> = None; // (name, start_line, body_start_idx)

    for (i, child) in children.iter().enumerate() {
        match child.kind() {
            "section_header" => {
                finalize_para(&mut para, (children, i, source, section.as_deref()), functions);
                section = header_name(*child, source);
            }
            "paragraph_header" => {
                finalize_para(&mut para, (children, i, source, section.as_deref()), functions);
                let name = header_name(*child, source).unwrap_or_else(|| "<anon>".into());
                para = Some((name, child.start_position().row as u32 + 1, i + 1));
            }
            "if_header" if para.is_none() => {
                *global_cond += 1;
                *global_nest = (*global_nest).max(scan_nesting(children, i));
            }
            _ => {}
        }
    }
    finalize_para(&mut para, (children, children.len(), source, section.as_deref()), functions);
}

fn finalize_para(
    para: &mut Option<(String, u32, usize)>,
    ctx: (&[Node], usize, &str, Option<&str>),
    out: &mut Vec<FunctionMetrics>,
) {
    let Some((name, start, body_idx)) = para.take() else { return };
    let (children, boundary, source, section) = ctx;
    let end = end_line_of(children, body_idx, boundary);
    let body = &children[body_idx..boundary.min(children.len())];
    let info = ParaInfo { body, source, name: &name, start_line: start, end_line: end };
    build_paragraph(&info, section, out);
}

fn end_line_of(children: &[Node], body_start: usize, body_end: usize) -> u32 {
    if body_end > body_start {
        children[body_end - 1].end_position().row as u32 + 1
    } else if body_start > 0 {
        children[body_start - 1].end_position().row as u32 + 1
    } else {
        1
    }
}

fn header_name(node: Node, source: &str) -> Option<String> {
    let text = node_text(node, source).trim().trim_end_matches('.');
    // Section headers include " SECTION" suffix — strip it
    let text = text.strip_suffix(" SECTION").unwrap_or(text).trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn build_paragraph(info: &ParaInfo, section: Option<&str>, out: &mut Vec<FunctionMetrics>) {
    let mut bw = BodyWalker { nodes: info.body, source: info.source, s: WalkState::new(), match_arms: 0 };
    bw.walk_range(0, info.body.len(), 0);

    let struct_hash = info.body.first().map_or(0, |n| compute_structural_fingerprint(*n));
    let distinct_kinds = count_distinct_node_kinds_multi(info.body);
    let skel_hash = info.body.iter().fold(0u64, |acc, n| acc.wrapping_mul(31).wrapping_add(compute_skeleton_hash(*n)));
    let assign_kinds = &["move_statement", "set_statement", "compute_statement"];
    let short_vars = info.body.iter().map(|n| count_short_variables(*n, info.source, assign_kinds)).sum();

    out.push(FunctionMetrics {
        name: info.name.to_string(),
        start_line: info.start_line,
        end_line: info.end_line,
        loc: info.end_line.saturating_sub(info.start_line) + 1,
        cc: bw.s.cc,
        cognitive_complexity: bw.s.cogc,
        max_nesting: bw.s.max_nesting,
        bump_count: bw.s.bump_count,
        arg_count: 0,
        compound_condition_count: bw.s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: bw.s.max_embedded_block_loc,
        structural_hash: struct_hash,
        distinct_node_kinds: distinct_kinds,
        skeleton_hash: skel_hash,
        consecutive_asserts: 0,
        assert_hash: 0,
        primitive_type_count: 0,
        typed_param_count: 0,
        max_same_primitive_count: 0,
        empty_catch_count: 0,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: section.map(String::from),
        parent_class: None,
        short_var_count: short_vars,
        string_match_arms: bw.match_arms,
        cpg: None,
    });
}

struct BodyWalker<'a, 'b> {
    nodes: &'a [Node<'b>],
    source: &'a str,
    s: WalkState,
    match_arms: u32,
}

impl BodyWalker<'_, '_> {
    fn walk_range(&mut self, start: usize, end: usize, depth: u32) {
        let mut i = start;
        let cap = self.nodes.len().saturating_add(1);
        let mut steps: usize = 0;
        while i < end && steps < cap {
            let prev = i;
            let next = self.dispatch(i, depth);
            i = if next > prev { next } else { prev + 1 };
            steps += 1;
        }
    }

    fn dispatch(&mut self, idx: usize, depth: u32) -> usize {
        match self.nodes[idx].kind() {
            "if_header" => self.handle_if(idx, depth),
            "evaluate_header" => self.handle_evaluate(idx, depth),
            "perform_statement_loop" => self.handle_perform(idx, depth),
            "string" | "h_string" | "n_string" | "x_string" => {
                track_embedded_block(&mut self.s.max_embedded_block_loc, self.nodes[idx]);
                idx + 1
            }
            _ => idx + 1,
        }
    }

    fn scan_block(&mut self, start: usize, end_marker: &str, depth: u32) -> usize {
        let saved = self.s.cogc_nesting;
        self.s.cogc_nesting += 1;
        let mut i = start;
        let cap = self.nodes.len().saturating_add(1);
        let mut steps: usize = 0;
        while i < self.nodes.len() && steps < cap {
            if self.nodes[i].kind() == end_marker {
                self.s.cogc_nesting = saved;
                return i + 1;
            }
            let prev = i;
            let next = self.dispatch(i, depth);
            i = if next > prev { next } else { prev + 1 };
            steps += 1;
        }
        self.s.cogc_nesting = saved;
        i
    }

    fn handle_if(&mut self, start: usize, depth: u32) -> usize {
        self.s.track_if(depth);
        self.s.track_cogc_branch();
        track_condition(self.nodes[start], self.source, &mut self.s);
        let saved = self.s.cogc_nesting;
        self.s.cogc_nesting += 1;

        let mut i = start + 1;
        let cap = self.nodes.len().saturating_add(1);
        let mut steps: usize = 0;
        while i < self.nodes.len() && steps < cap {
            match self.nodes[i].kind() {
                "else_if_header" => {
                    self.s.cc += 1;
                    self.s.track_cogc_branch();
                    track_condition(self.nodes[i], self.source, &mut self.s);
                    i += 1;
                }
                "else_header" => {
                    self.s.track_cogc_flat();
                    i += 1;
                }
                "END_IF" => {
                    self.s.cogc_nesting = saved;
                    return i + 1;
                }
                _ => {
                    let prev = i;
                    let next = self.dispatch(i, depth + 1);
                    i = if next > prev { next } else { prev + 1 };
                }
            }
            steps += 1;
        }
        self.s.cogc_nesting = saved;
        i
    }

    fn handle_evaluate(&mut self, start: usize, depth: u32) -> usize {
        self.s.track_nesting(depth);
        self.s.track_cogc_branch();
        let saved = self.s.cogc_nesting;
        self.s.cogc_nesting += 1;

        let mut i = start + 1;
        let cap = self.nodes.len().saturating_add(1);
        let mut steps: usize = 0;
        while i < self.nodes.len() && steps < cap {
            match self.nodes[i].kind() {
                "when" => {
                    self.s.cc += 1;
                    self.match_arms += 1;
                    i += 1;
                }
                "when_other" => {
                    self.s.track_cogc_flat();
                    i += 1;
                }
                "END_EVALUATE" => {
                    self.s.cogc_nesting = saved;
                    return i + 1;
                }
                _ => {
                    let prev = i;
                    let next = self.dispatch(i, depth + 1);
                    i = if next > prev { next } else { prev + 1 };
                }
            }
            steps += 1;
        }
        self.s.cogc_nesting = saved;
        i
    }

    fn handle_perform(&mut self, start: usize, depth: u32) -> usize {
        self.s.track_loop(depth);
        self.s.track_cogc_branch();
        if let Some(opt) = self.nodes[start].child_by_field_name("option") {
            track_perform_bools(opt, &mut self.s);
        }
        self.scan_block(start + 1, "END_PERFORM", depth + 1)
    }
}

fn track_condition(header: Node, source: &str, s: &mut WalkState) {
    let Some(cond) = header.child_by_field_name("condition") else { return };
    count_expr_bools(cond, &mut s.cc, &mut s.cogc);
    let text = node_text(cond, source);
    if text.matches(" AND ").count() + text.matches(" OR ").count() >= 2 {
        s.compound_condition_count += 1;
    }
}

fn track_perform_bools(opt: Node, s: &mut WalkState) {
    if let Some(expr) = opt.child_by_field_name("until") {
        count_expr_bools(expr, &mut s.cc, &mut s.cogc);
    }
    let mut cursor = opt.walk();
    for child in opt.children(&mut cursor) {
        if child.kind() != "perform_varying" {
            continue;
        }
        let mut vc = child.walk();
        for vc_child in child.children(&mut vc) {
            if vc_child.kind() == "expr" {
                count_expr_bools(vc_child, &mut s.cc, &mut s.cogc);
            }
        }
    }
}

fn count_expr_bools(node: Node, cc: &mut u32, cogc: &mut u32) {
    let mut prev_op: Option<&str> = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        let kind = child.kind();
        let is_bool = kind == "AND" || kind == "OR";
        if is_bool {
            *cc += 1;
            if prev_op != Some(kind) {
                *cogc += 1;
            }
            prev_op = Some(kind);
        } else if kind == "expr" {
            count_expr_bools(child, cc, cogc);
            prev_op = None;
        } else {
            prev_op = None;
        }
    }
}

fn scan_nesting(children: &[Node], start: usize) -> u32 {
    let mut depth: u32 = 1;
    let mut max: u32 = 1;
    for child in &children[start + 1..] {
        match child.kind() {
            "if_header" => {
                depth += 1;
                max = max.max(depth);
            }
            "END_IF" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return max;
                }
            }
            "paragraph_header" | "section_header" => return max,
            _ => {}
        }
    }
    max
}

fn count_descendants(node: Node, target: &str) -> u32 {
    let mut count: u32 = 0;
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            if child.kind() == target {
                count += 1;
            } else {
                stack.push(child);
            }
        }
    }
    count
}
