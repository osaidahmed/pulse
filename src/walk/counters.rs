use tree_sitter::Node;

use super::{find_child_by_kind, node_text};

const SHORT_VAR_EXEMPT: &[&str] = &["_", "i", "j", "k"];

pub fn max_same_primitive(types: &[&str]) -> u32 {
    types
        .iter()
        .map(|ty| types.iter().filter(|x| *x == ty).count() as u32)
        .max()
        .unwrap_or(0)
}

pub fn count_short_variables(body: Node, source: &str, binding_kinds: &[&str]) -> u32 {
    let mut count: u32 = 0;
    walk_for_short_vars(body, source, binding_kinds, &mut count);
    count
}

fn walk_for_short_vars(node: Node, source: &str, kinds: &[&str], count: &mut u32) {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        if kinds.contains(&child.kind()) {
            let id = find_first_identifier(child);
            let short = id
                .map(|n| node_text(n, source))
                .is_some_and(|name| name.len() == 1 && !SHORT_VAR_EXEMPT.contains(&name));
            *count += u32::from(short);
        }
        walk_for_short_vars(child, source, kinds, count);
    }
}

fn find_first_identifier(node: Node) -> Option<Node> {
    if let Some(id) = find_child_by_kind(node, "identifier") { return Some(id); }
    if let Some(id) = find_child_by_kind(node, "simple_identifier") { return Some(id); }
    if let Some(id) = find_child_by_kind(node, "id") { return Some(id); }
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).find_map(find_first_identifier);
    result
}

struct MatchKinds<'a> {
    case: &'a str,
    strings: &'a [&'a str],
    defaults: &'a [&'a str],
}

pub fn count_string_match_arms(
    body: Node,
    match_kind: &str,
    case_kind: &str,
    string_kinds: &[&str],
    default_kinds: &[&str],
) -> u32 {
    let kinds = MatchKinds { case: case_kind, strings: string_kinds, defaults: default_kinds };
    let mut max_arms: u32 = 0;
    walk_for_string_matches(body, match_kind, &kinds, &mut max_arms);
    max_arms
}

fn walk_for_string_matches(node: Node, match_kind: &str, kinds: &MatchKinds, max_arms: &mut u32) {
    let mut child_opt = node.child(0);
    while let Some(child) = child_opt {
        if child.kind() != match_kind {
            walk_for_string_matches(child, match_kind, kinds, max_arms);
        } else if !switch_has_default(child, kinds) {
            let arms = tally_string_cases(child, kinds.case, kinds.strings);
            if arms > *max_arms {
                *max_arms = arms;
            }
        }
        child_opt = child.next_sibling();
    }
}

fn switch_has_default(switch_node: Node, kinds: &MatchKinds) -> bool {
    if kinds.defaults.is_empty() {
        has_valueless_case(switch_node, kinds.case, kinds.strings)
    } else {
        has_kind_in(switch_node, kinds.defaults)
    }
}

fn has_kind_in(node: Node, kinds: &[&str]) -> bool {
    let mut cursor = node.walk();
    let result = node
        .children(&mut cursor)
        .any(|child| kinds.contains(&child.kind()) || has_kind_in(child, kinds));
    result
}

fn has_valueless_case(node: Node, case_kind: &str, string_kinds: &[&str]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == case_kind {
            if !case_has_string(&child, string_kinds) {
                return true;
            }
        } else if has_valueless_case(child, case_kind, string_kinds) {
            return true;
        }
    }
    false
}

fn tally_string_cases(match_node: Node, case_kind: &str, string_kinds: &[&str]) -> u32 {
    let mut count: u32 = 0;
    tally_recursive(match_node, case_kind, string_kinds, &mut count);
    count
}

fn tally_recursive(node: Node, case_kind: &str, string_kinds: &[&str], count: &mut u32) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == case_kind && case_has_string(&child, string_kinds) {
            *count += 1;
        } else {
            tally_recursive(child, case_kind, string_kinds, count);
        }
    }
}

fn case_has_string(node: &Node, string_kinds: &[&str]) -> bool {
    if string_kinds.contains(&node.kind()) { return true; }
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).any(|c| case_has_string(&c, string_kinds));
    result
}
