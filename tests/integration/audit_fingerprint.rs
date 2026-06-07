use pulse::parse::{self, Language};
use pulse::walk::fingerprint;
use tree_sitter::Node;

fn parse_python(src: &str) -> tree_sitter::Tree {
    parse::parse_only(src, Language::Python).expect("parse failed")
}

fn first_named_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    for child in node.children(&mut node.walk()) {
        if let Some(n) = first_named_child(child, kind) {
            return Some(n);
        }
    }
    None
}

fn fingerprint_of_first(src: &str, kind: &str) -> u64 {
    let tree = parse_python(src);
    let node = first_named_child(tree.root_node(), kind).unwrap_or_else(|| panic!("no {kind} in source"));
    fingerprint::compute_subtree_fingerprint(node)
}

#[test]
fn fingerprint_of_identical_python_subtree_is_equal_across_runs() {
    let a = fingerprint_of_first("def f():\n    return 1\n", "function_definition");
    let b = fingerprint_of_first("def f():\n    return 1\n", "function_definition");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_of_minor_whitespace_edit_is_unchanged() {
    let a = fingerprint_of_first("def f():\n    return 1\n", "function_definition");
    let b = fingerprint_of_first("def  f():\n        return  1\n", "function_definition");
    assert_eq!(a, b, "whitespace differences must not change fingerprint");
}

#[test]
fn fingerprint_unchanged_when_blank_lines_added() {
    let a = fingerprint_of_first("def f():\n    return 1\n", "function_definition");
    let b = fingerprint_of_first("\n\ndef f():\n\n    return 1\n\n", "function_definition");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_unchanged_when_inline_comment_added() {
    let a = fingerprint_of_first("def f():\n    return 1\n", "function_definition");
    let b = fingerprint_of_first("def f():\n    return 1  # ok\n", "function_definition");
    assert_eq!(a, b, "inline comments are anonymous; fingerprint should not change");
}

#[test]
fn fingerprint_unchanged_when_identifier_text_changes() {
    let a = fingerprint_of_first("x == y", "comparison_operator");
    let b = fingerprint_of_first("foo == bar", "comparison_operator");
    assert_eq!(a, b, "identifier text is not preserved in mode B");
}

#[test]
fn fingerprint_changes_when_identifier_replaced_by_string() {
    let a = fingerprint_of_first("x == y", "comparison_operator");
    let b = fingerprint_of_first("x == \"y\"", "comparison_operator");
    assert_ne!(a, b, "identifier vs string kind must differ");
}

#[test]
fn fingerprint_changes_when_identifier_replaced_by_integer() {
    let a = fingerprint_of_first("x == y", "comparison_operator");
    let b = fingerprint_of_first("x == 1", "comparison_operator");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_does_not_distinguish_eq_from_neq() {
    let a = fingerprint_of_first("x == y", "comparison_operator");
    let b = fingerprint_of_first("x != y", "comparison_operator");
    assert_eq!(a, b, "operator symbol is anonymous; mode B does not distinguish");
}

#[test]
fn fingerprint_distinguishes_attribute_from_identifier() {
    let a = fingerprint_of_first("x == y", "comparison_operator");
    let b = fingerprint_of_first("x == y.z", "comparison_operator");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_distinguishes_call_from_attribute() {
    let a = fingerprint_of_first("x == y.z", "comparison_operator");
    let b = fingerprint_of_first("x == y.z()", "comparison_operator");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_recurses_through_nested_attribute_chain() {
    let a = fingerprint_of_first("x == y.z", "comparison_operator");
    let b = fingerprint_of_first("x == y.z.w", "comparison_operator");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_of_python_function_def_is_distinct_from_class_def() {
    let a = fingerprint_of_first("def f():\n    pass\n", "function_definition");
    let b = fingerprint_of_first("class C:\n    pass\n", "class_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_of_for_loop_distinct_from_while_loop() {
    let a = fingerprint_of_first("for x in y:\n    pass\n", "for_statement");
    let b = fingerprint_of_first("while x:\n    pass\n", "while_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_of_list_comprehension_distinct_from_list_literal() {
    let a = fingerprint_of_first("xs = [x for x in y]\n", "list_comprehension");
    let b = fingerprint_of_first("xs = [1, 2, 3]\n", "list");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_of_if_with_one_branch_distinct_from_if_else() {
    let a = fingerprint_of_first("if x:\n    pass\n", "if_statement");
    let b = fingerprint_of_first("if x:\n    pass\nelse:\n    pass\n", "if_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_distinct_from_compute_structural_fingerprint() {
    let tree = parse_python("def f():\n    return 1\n");
    let func = first_named_child(tree.root_node(), "function_definition").unwrap();
    let body = first_named_child(func, "block").unwrap();
    let mode_b = fingerprint::compute_subtree_fingerprint(body);
    let existing = fingerprint::compute_structural_fingerprint(body);
    assert_ne!(mode_b, existing, "Mode B and existing fingerprint must differ");
}

#[test]
fn fingerprint_handles_deeply_nested_subtree() {
    let mut src = String::from("xs = ");
    for _ in 0..50 {
        src.push('[');
    }
    src.push('1');
    for _ in 0..50 {
        src.push(']');
    }
    src.push('\n');
    let tree = parse_python(&src);
    let _hash = fingerprint::compute_subtree_fingerprint(tree.root_node());
}

#[test]
fn fingerprint_handles_subtree_with_many_children() {
    let mut src = String::from("xs = (");
    for i in 0..200 {
        if i > 0 {
            src.push_str(", ");
        }
        src.push_str(&format!("{i}"));
    }
    src.push_str(")\n");
    let tree = parse_python(&src);
    let _hash = fingerprint::compute_subtree_fingerprint(tree.root_node());
}

#[test]
fn fingerprint_of_syntactically_invalid_python_returns_value() {
    let tree = parse_python("def f(:\n    x = \n");
    let _hash = fingerprint::compute_subtree_fingerprint(tree.root_node());
}

#[test]
fn fingerprint_of_empty_input_returns_value() {
    let tree = parse_python("");
    let _hash = fingerprint::compute_subtree_fingerprint(tree.root_node());
}

#[test]
fn fingerprint_unchanged_when_unicode_identifier_replaced_with_ascii() {
    let a = fingerprint_of_first("δ == 1", "comparison_operator");
    let b = fingerprint_of_first("d == 1", "comparison_operator");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_distinct_for_eq_string_vs_eq_concatenated_string() {
    let a = fingerprint_of_first("x == \"a\"", "comparison_operator");
    let b = fingerprint_of_first("x == \"a\" \"b\"", "comparison_operator");
    assert_ne!(a, b, "concatenated_string kind differs from string kind");
}

#[test]
fn fingerprint_skips_anonymous_punctuation() {
    let a = fingerprint_of_first("x == (y)", "comparison_operator");
    let b = fingerprint_of_first("x == y", "comparison_operator");
    assert_eq!(a, b, "parenthesized expressions wrap an identifier; named structure equal");
}

#[test]
fn fingerprint_walks_only_named_children() {
    let tree = parse_python("def f(): return 1\n");
    let func = first_named_child(tree.root_node(), "function_definition").unwrap();
    let h1 = fingerprint::compute_subtree_fingerprint(func);
    let same = parse_python("def f(): return 1\n");
    let func2 = first_named_child(same.root_node(), "function_definition").unwrap();
    let h2 = fingerprint::compute_subtree_fingerprint(func2);
    assert_eq!(h1, h2);
}

#[test]
fn fingerprint_terminator_bytes_actually_contribute() {
    let a = fingerprint_of_first("x == y", "comparison_operator");
    let b = fingerprint_of_first("x", "identifier");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_two_calls_on_same_node_equal() {
    let tree = parse_python("def f():\n    return 1\n");
    let node = tree.root_node();
    let h1 = fingerprint::compute_subtree_fingerprint(node);
    let h2 = fingerprint::compute_subtree_fingerprint(node);
    assert_eq!(h1, h2);
}

#[test]
fn fingerprint_changes_when_arithmetic_operator_replaces_comparison() {
    let a = fingerprint_of_first("x == y", "comparison_operator");
    let b = fingerprint_of_first("x + y", "binary_operator");
    assert_ne!(a, b, "comparison_operator and binary_operator have different kinds");
}

#[test]
fn fingerprint_class_with_no_base_distinct_from_class_with_base() {
    let a = fingerprint_of_first("class C:\n    pass\n", "class_definition");
    let b = fingerprint_of_first("class C(B):\n    pass\n", "class_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_of_two_class_definitions_with_same_arity_equal() {
    let a = fingerprint_of_first("class A:\n    pass\n", "class_definition");
    let b = fingerprint_of_first("class B:\n    pass\n", "class_definition");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_of_two_distinct_function_signatures_with_different_arity_differ() {
    let a = fingerprint_of_first("def f(x):\n    pass\n", "function_definition");
    let b = fingerprint_of_first("def f(x, y):\n    pass\n", "function_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_of_self_dot_user_assignment_equal_across_files() {
    let a = fingerprint_of_first("self.user = user\n", "assignment");
    let b = fingerprint_of_first("self.user = user\n", "assignment");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_of_self_attribute_assignment_distinct_from_local_assignment() {
    let a = fingerprint_of_first("self.user = user\n", "assignment");
    let b = fingerprint_of_first("user = user\n", "assignment");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_media_type_comparison_clusters_across_files() {
    let a = fingerprint_of_first("media_type == \"tv\"", "comparison_operator");
    let b = fingerprint_of_first("media_type == \"season\"", "comparison_operator");
    let c = fingerprint_of_first("foo == \"x\"", "comparison_operator");
    assert_eq!(a, b, "two media_type == string sites must hash equal");
    assert_eq!(a, c, "any identifier == string has the same shape under mode B");
}

#[test]
fn fingerprint_dict_get_calls_cluster() {
    let a = fingerprint_of_first("d.get(k, default)", "call");
    let b = fingerprint_of_first("meta.get(name, fallback)", "call");
    assert_eq!(a, b, "two-arg method calls on identifiers cluster identically");
}

#[test]
fn fingerprint_list_comprehensions_with_same_shape_cluster() {
    let a = fingerprint_of_first("[x for x in y if x]\n", "list_comprehension");
    let b = fingerprint_of_first("[a for a in b if a]\n", "list_comprehension");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_list_comprehension_with_filter_distinct_from_without() {
    let a = fingerprint_of_first("[x for x in y if x]\n", "list_comprehension");
    let b = fingerprint_of_first("[x for x in y]\n", "list_comprehension");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_function_call_one_arg_distinct_from_two_args() {
    let a = fingerprint_of_first("f(x)", "call");
    let b = fingerprint_of_first("f(x, y)", "call");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_method_call_distinct_from_function_call() {
    let a = fingerprint_of_first("f(x)", "call");
    let b = fingerprint_of_first("o.f(x)", "call");
    assert_ne!(a, b);
}
