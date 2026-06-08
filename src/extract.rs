use tree_sitter::{Node, Point};

use crate::parse::{self, Language};

pub struct ExtractRegion {
    pub start_line: u32,
    pub end_line: u32,
    pub nesting: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct LineSpan {
    pub start: u32,
    pub end: u32,
}

pub fn suggest_extract(source: &str, lang: Language, span: LineSpan) -> Option<ExtractRegion> {
    let kinds = control_flow_kinds(lang)?;
    let tree = parse::parse_guarded(source, lang)?;
    let func = enclosing_function(tree.root_node(), span)?;
    deepest_region(func, kinds)
}

fn control_flow_kinds(lang: Language) -> Option<&'static [&'static str]> {
    let kinds = crate::langkinds::control_flow_kinds(lang);
    (!kinds.is_empty()).then_some(kinds)
}

fn enclosing_function(root: Node, span: LineSpan) -> Option<Node> {
    let start = Point { row: span.start.saturating_sub(1) as usize, column: 0 };
    let end = Point { row: span.end.saturating_sub(1) as usize, column: 0 };
    root.descendant_for_point_range(start, end)
}

fn deepest_region(func: Node, kinds: &[&str]) -> Option<ExtractRegion> {
    let mut best: Option<ExtractRegion> = None;
    let mut best_depth = 1u32;
    let mut stack: Vec<(Node, u32)> = vec![(func, 0)];
    while let Some((node, depth)) = stack.pop() {
        let depth = if kinds.contains(&node.kind()) { depth + 1 } else { depth };
        if depth > best_depth {
            best_depth = depth;
            best = Some(region_of(node, depth));
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push((child, depth));
        }
    }
    best
}

fn region_of(node: Node, nesting: u32) -> ExtractRegion {
    ExtractRegion {
        start_line: node.start_position().row as u32 + 1,
        end_line: node.end_position().row as u32 + 1,
        nesting,
    }
}
