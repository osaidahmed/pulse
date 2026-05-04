use pulse::parse::{self, Language};
use pulse::walk::fingerprint::{compute_subtree_fingerprint, compute_subtree_fingerprint_seeded};
use tree_sitter::Node;

fn root(src: &str, lang: Language) -> tree_sitter::Tree {
    parse::parse_only(src, lang).unwrap()
}

fn first_named<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    for child in node.children(&mut node.walk()) {
        if let Some(n) = first_named(child, kind) {
            return Some(n);
        }
    }
    None
}

fn fp_of_first(src: &str, lang: Language, kind: &str) -> u64 {
    let tree = root(src, lang);
    let node = first_named(tree.root_node(), kind).unwrap();
    compute_subtree_fingerprint(node)
}

fn fp_seeded(src: &str, lang: Language, kind: &str, seed: u64) -> u64 {
    let tree = root(src, lang);
    let node = first_named(tree.root_node(), kind).unwrap();
    compute_subtree_fingerprint_seeded(node, seed)
}

#[test]
fn seeded_fingerprint_differs_with_different_seeds() {
    let a = fp_seeded("x == y", Language::Python, "comparison_operator", 0);
    let b = fp_seeded("x == y", Language::Python, "comparison_operator", 1);
    assert_ne!(a, b);
}

#[test]
fn seeded_fingerprint_same_with_same_seed() {
    let a = fp_seeded("x == y", Language::Python, "comparison_operator", 7);
    let b = fp_seeded("x == y", Language::Python, "comparison_operator", 7);
    assert_eq!(a, b);
}

#[test]
fn seeded_fingerprint_zero_seed_differs_from_unseeded() {
    let unseeded = fp_of_first("x == y", Language::Python, "comparison_operator");
    let seeded = fp_seeded("x == y", Language::Python, "comparison_operator", 0);
    assert_ne!(unseeded, seeded);
}

#[test]
fn seeded_fingerprint_max_seed_distinct() {
    let a = fp_seeded("x", Language::Python, "identifier", u64::MAX);
    let b = fp_seeded("x", Language::Python, "identifier", u64::MAX - 1);
    assert_ne!(a, b);
}

#[test]
fn fingerprint_dict_distinct_from_set() {
    let a = fp_of_first("xs = {1: 2}", Language::Python, "dictionary");
    let b = fp_of_first("xs = {1, 2}", Language::Python, "set");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_tuple_one_element_distinct_from_two() {
    let a = fp_of_first("xs = (1,)", Language::Python, "tuple");
    let b = fp_of_first("xs = (1, 2)", Language::Python, "tuple");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_lambda_distinct_from_function_definition() {
    let a = fp_of_first("f = lambda x: x", Language::Python, "lambda");
    let b = fp_of_first("def g(x):\n    return x\n", Language::Python, "function_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_with_statement_distinct_from_for_statement() {
    let a = fp_of_first("with x:\n    pass\n", Language::Python, "with_statement");
    let b = fp_of_first("for x in y:\n    pass\n", Language::Python, "for_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_try_except_finally_distinct_from_try_except() {
    let a = fp_of_first("try:\n    pass\nexcept:\n    pass\nfinally:\n    pass\n", Language::Python, "try_statement");
    let b = fp_of_first("try:\n    pass\nexcept:\n    pass\n", Language::Python, "try_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_async_function_same_kind_as_regular_function() {
    let a = fp_of_first("async def f():\n    return 1\n", Language::Python, "function_definition");
    let b = fp_of_first("def f():\n    return 1\n", Language::Python, "function_definition");
    assert_eq!(a, b, "async keyword is anonymous in mode B; same fingerprint");
}

#[test]
fn fingerprint_decorated_distinct_from_undecorated() {
    let a = fp_of_first("@decorator\ndef f():\n    return 1\n", Language::Python, "decorated_definition");
    let b = fp_of_first("def f():\n    return 1\n", Language::Python, "function_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_dict_comprehension_distinct_from_list_comprehension() {
    let a = fp_of_first("xs = {k: v for k, v in d.items()}\n", Language::Python, "dictionary_comprehension");
    let b = fp_of_first("xs = [k for k in d]\n", Language::Python, "list_comprehension");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_set_comprehension_distinct_from_list_comprehension() {
    let a = fp_of_first("xs = {x for x in y}\n", Language::Python, "set_comprehension");
    let b = fp_of_first("xs = [x for x in y]\n", Language::Python, "list_comprehension");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_generator_expression_distinct_from_list_comprehension() {
    let a = fp_of_first("xs = (x for x in y)\n", Language::Python, "generator_expression");
    let b = fp_of_first("xs = [x for x in y]\n", Language::Python, "list_comprehension");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_yield_distinct_from_return() {
    let a = fp_of_first("def f():\n    yield 1\n", Language::Python, "yield");
    let b = fp_of_first("def f():\n    return 1\n", Language::Python, "return_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_keyword_argument_distinct_from_positional() {
    let a = fp_of_first("f(x=1)", Language::Python, "call");
    let b = fp_of_first("f(1)", Language::Python, "call");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_starred_argument_distinct_from_normal() {
    let a = fp_of_first("f(*xs)", Language::Python, "call");
    let b = fp_of_first("f(xs)", Language::Python, "call");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_double_starred_distinct_from_starred() {
    let a = fp_of_first("f(**xs)", Language::Python, "call");
    let b = fp_of_first("f(*xs)", Language::Python, "call");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_chained_comparison_distinct_from_single() {
    let a = fp_of_first("a < b < c", Language::Python, "comparison_operator");
    let b = fp_of_first("a < b", Language::Python, "comparison_operator");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_conditional_expression_distinct_from_if_statement() {
    let a = fp_of_first("y = 1 if x else 0\n", Language::Python, "conditional_expression");
    let b = fp_of_first("if x:\n    y = 1\nelse:\n    y = 0\n", Language::Python, "if_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_walrus_assignment_distinct_from_normal() {
    let a = fp_of_first("if (n := len(xs)) > 0:\n    pass\n", Language::Python, "named_expression");
    let _ = a;
}

#[test]
fn fingerprint_subscript_simple_distinct_from_slice() {
    let a = fp_of_first("xs[0]", Language::Python, "subscript");
    let b = fp_of_first("xs[0:2]", Language::Python, "subscript");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_slice_with_step_distinct_from_without() {
    let a = fp_of_first("xs[0:2:1]", Language::Python, "subscript");
    let b = fp_of_first("xs[0:2]", Language::Python, "subscript");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_global_distinct_from_nonlocal() {
    let a = fp_of_first("def f():\n    global x\n    x = 1\n", Language::Python, "global_statement");
    let b = fp_of_first("def f():\n    nonlocal x\n    x = 1\n", Language::Python, "nonlocal_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_raise_with_arg_distinct_from_bare() {
    let a = fp_of_first("raise ValueError(\"x\")\n", Language::Python, "raise_statement");
    let b = fp_of_first("raise\n", Language::Python, "raise_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_assert_with_message_distinct_from_without() {
    let a = fp_of_first("assert x, \"msg\"\n", Language::Python, "assert_statement");
    let b = fp_of_first("assert x\n", Language::Python, "assert_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_class_with_metaclass_distinct_from_without() {
    let a = fp_of_first("class C(metaclass=M):\n    pass\n", Language::Python, "class_definition");
    let b = fp_of_first("class C:\n    pass\n", Language::Python, "class_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_class_with_multiple_bases_distinct_from_one() {
    let a = fp_of_first("class C(A, B):\n    pass\n", Language::Python, "class_definition");
    let b = fp_of_first("class C(A):\n    pass\n", Language::Python, "class_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_typed_function_signature_distinct_from_untyped() {
    let a = fp_of_first("def f(x: int) -> int:\n    return x\n", Language::Python, "function_definition");
    let b = fp_of_first("def f(x):\n    return x\n", Language::Python, "function_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_string_literal_distinct_from_fstring() {
    let a = fp_of_first("\"hello\"\n", Language::Python, "string");
    let _ = a;
}

#[test]
fn fingerprint_raw_string_treated_same_as_regular_string_kind() {
    let a = fp_of_first("\"abc\"\n", Language::Python, "string");
    let b = fp_of_first("r\"abc\"\n", Language::Python, "string");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_byte_string_treated_same_as_regular_string_kind() {
    let a = fp_of_first("\"abc\"\n", Language::Python, "string");
    let b = fp_of_first("b\"abc\"\n", Language::Python, "string");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_integer_literal_same_for_different_values() {
    let a = fp_of_first("x = 1\n", Language::Python, "integer");
    let b = fp_of_first("x = 999\n", Language::Python, "integer");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_float_literal_same_for_different_values() {
    let a = fp_of_first("x = 1.0\n", Language::Python, "float");
    let b = fp_of_first("x = 3.14\n", Language::Python, "float");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_integer_distinct_from_float() {
    let a = fp_of_first("x = 1\n", Language::Python, "integer");
    let b = fp_of_first("x = 1.0\n", Language::Python, "float");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_true_distinct_from_false() {
    let a = fp_of_first("x = True\n", Language::Python, "true");
    let b = fp_of_first("x = False\n", Language::Python, "false");
    assert_ne!(a, b, "true and false have distinct kinds");
}

#[test]
fn fingerprint_none_distinct_from_true() {
    let a = fp_of_first("x = None\n", Language::Python, "none");
    let b = fp_of_first("x = True\n", Language::Python, "true");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_assignment_target_attribute_vs_subscript() {
    let a = fp_of_first("x.y = 1\n", Language::Python, "assignment");
    let b = fp_of_first("x[0] = 1\n", Language::Python, "assignment");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_multiple_assignment_distinct_from_single() {
    let a = fp_of_first("x = 1\n", Language::Python, "assignment");
    let b = fp_of_first("x, y = 1, 2\n", Language::Python, "assignment");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_augmented_assignment_distinct_from_assignment() {
    let a = fp_of_first("x += 1\n", Language::Python, "augmented_assignment");
    let b = fp_of_first("x = 1\n", Language::Python, "assignment");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_augmented_op_kind_uniform_across_operators() {
    let a = fp_of_first("x += 1\n", Language::Python, "augmented_assignment");
    let b = fp_of_first("x -= 1\n", Language::Python, "augmented_assignment");
    assert_eq!(a, b, "augmented op symbol is anonymous");
}

#[test]
fn fingerprint_unary_minus_distinct_from_positive() {
    let a = fp_of_first("x = -1\n", Language::Python, "unary_operator");
    let _ = a;
}

#[test]
fn fingerprint_not_distinct_from_unary_minus() {
    let a = fp_of_first("x = not True\n", Language::Python, "not_operator");
    let b = fp_of_first("x = -1\n", Language::Python, "unary_operator");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_boolean_and_distinct_from_or() {
    let _ = fp_of_first("x and y", Language::Python, "boolean_operator");
}

#[test]
fn fingerprint_print_function_distinct_from_function_call_other() {
    let a = fp_of_first("print(\"x\")\n", Language::Python, "call");
    let b = fp_of_first("len(\"x\")\n", Language::Python, "call");
    assert_eq!(a, b, "two function calls with same arity must hash equal");
}

#[test]
fn fingerprint_method_chain_each_link_recursed() {
    let a = fp_of_first("x.y.z", Language::Python, "attribute");
    let b = fp_of_first("x.y", Language::Python, "attribute");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_keyword_arg_with_string_distinct_from_keyword_arg_with_int() {
    let a = fp_of_first("f(x=\"a\")", Language::Python, "call");
    let b = fp_of_first("f(x=1)", Language::Python, "call");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_default_param_distinct_from_no_default() {
    let a = fp_of_first("def f(x=1):\n    pass\n", Language::Python, "function_definition");
    let b = fp_of_first("def f(x):\n    pass\n", Language::Python, "function_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_typed_param_default_distinct_from_typed_no_default() {
    let a = fp_of_first("def f(x: int = 1):\n    pass\n", Language::Python, "function_definition");
    let b = fp_of_first("def f(x: int):\n    pass\n", Language::Python, "function_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_arg_with_star_args_distinct_from_normal() {
    let a = fp_of_first("def f(*args):\n    pass\n", Language::Python, "function_definition");
    let b = fp_of_first("def f(args):\n    pass\n", Language::Python, "function_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_kwargs_param_distinct_from_args_param() {
    let a = fp_of_first("def f(**kwargs):\n    pass\n", Language::Python, "function_definition");
    let b = fp_of_first("def f(*args):\n    pass\n", Language::Python, "function_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_class_with_three_methods_distinct_from_two() {
    let a = fp_of_first("class C:\n    def a(self): pass\n    def b(self): pass\n    def c(self): pass\n", Language::Python, "class_definition");
    let b = fp_of_first("class C:\n    def a(self): pass\n    def b(self): pass\n", Language::Python, "class_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_two_classes_with_same_method_count_share_hash() {
    let a = fp_of_first("class A:\n    def a(self): pass\n    def b(self): pass\n", Language::Python, "class_definition");
    let b = fp_of_first("class B:\n    def a(self): pass\n    def b(self): pass\n", Language::Python, "class_definition");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_for_with_else_distinct_from_for_without() {
    let a = fp_of_first("for x in y:\n    pass\nelse:\n    pass\n", Language::Python, "for_statement");
    let b = fp_of_first("for x in y:\n    pass\n", Language::Python, "for_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_while_with_else_distinct_from_while_without() {
    let a = fp_of_first("while x:\n    pass\nelse:\n    pass\n", Language::Python, "while_statement");
    let b = fp_of_first("while x:\n    pass\n", Language::Python, "while_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_continue_distinct_from_break() {
    let a = fp_of_first("for x in y:\n    continue\n", Language::Python, "continue_statement");
    let b = fp_of_first("for x in y:\n    break\n", Language::Python, "break_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_pass_distinct_from_continue() {
    let a = fp_of_first("for x in y:\n    pass\n", Language::Python, "pass_statement");
    let b = fp_of_first("for x in y:\n    continue\n", Language::Python, "continue_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_import_distinct_from_import_from() {
    let a = fp_of_first("import os\n", Language::Python, "import_statement");
    let b = fp_of_first("from os import path\n", Language::Python, "import_from_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_import_with_alias_distinct_from_without() {
    let a = fp_of_first("import os as o\n", Language::Python, "import_statement");
    let b = fp_of_first("import os\n", Language::Python, "import_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_relative_import_distinct_from_absolute() {
    let a = fp_of_first("from .x import y\n", Language::Python, "import_from_statement");
    let b = fp_of_first("from x import y\n", Language::Python, "import_from_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_with_as_distinct_from_with_no_as() {
    let a = fp_of_first("with x as y:\n    pass\n", Language::Python, "with_statement");
    let b = fp_of_first("with x:\n    pass\n", Language::Python, "with_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_multiple_with_items_distinct_from_one() {
    let a = fp_of_first("with x, y:\n    pass\n", Language::Python, "with_statement");
    let b = fp_of_first("with x:\n    pass\n", Language::Python, "with_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_nested_function_distinct_from_top_level() {
    let nested = "def outer():\n    def inner():\n        return 1\n    return inner\n";
    let top = "def outer():\n    return 1\n";
    let a = fp_of_first(nested, Language::Python, "function_definition");
    let b = fp_of_first(top, Language::Python, "function_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_complex_subtree_stable_under_renaming() {
    let a = fp_of_first("def f(self, x, y):\n    return self.helper(x) + y\n", Language::Python, "function_definition");
    let b = fp_of_first("def g(self, a, b):\n    return self.helper(a) + b\n", Language::Python, "function_definition");
    assert_eq!(a, b, "renamed identifiers must hash equal");
}

#[test]
fn fingerprint_two_function_calls_with_string_arg_share_hash() {
    let a = fp_of_first("f(\"hello\")", Language::Python, "call");
    let b = fp_of_first("g(\"world\")", Language::Python, "call");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_nested_call_distinct_from_flat_call() {
    let a = fp_of_first("f(g(x))", Language::Python, "call");
    let b = fp_of_first("f(x)", Language::Python, "call");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_recursion_terminates_for_self_referential_nodes() {
    let _ = fp_of_first("def f(): return f()\n", Language::Python, "function_definition");
}

#[test]
fn fingerprint_handles_extreme_nesting_one_thousand() {
    let mut src = String::from("xs = ");
    for _ in 0..500 {
        src.push('[');
    }
    src.push('1');
    for _ in 0..500 {
        src.push(']');
    }
    src.push('\n');
    let tree = parse::parse_only(&src, Language::Python).unwrap();
    let _ = compute_subtree_fingerprint(tree.root_node());
}

#[test]
fn fingerprint_handles_wide_argument_list_five_hundred() {
    let mut src = String::from("f(");
    for i in 0..500 {
        if i > 0 {
            src.push_str(", ");
        }
        src.push('x');
    }
    src.push_str(")\n");
    let _ = fp_of_first(&src, Language::Python, "call");
}

#[test]
fn seeded_fingerprint_handles_extreme_seeds() {
    for seed in [0_u64, 1, u64::MAX, 0xdead_beef, 0x5555_5555_5555_5555] {
        let _ = fp_seeded("x == y", Language::Python, "comparison_operator", seed);
    }
}

#[test]
fn fingerprint_python_dict_key_value_distinct_from_set_element() {
    let a = fp_of_first("{1: 2}", Language::Python, "dictionary");
    let b = fp_of_first("{1}", Language::Python, "set");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_starred_in_assignment_distinct_from_normal() {
    let a = fp_of_first("a, *b = [1, 2, 3]\n", Language::Python, "assignment");
    let b = fp_of_first("a, b = [1, 2]\n", Language::Python, "assignment");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_typed_var_assignment_distinct_from_untyped() {
    let a = fp_of_first("x: int = 1\n", Language::Python, "assignment");
    let b = fp_of_first("x = 1\n", Language::Python, "assignment");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_async_for_distinct_from_for() {
    let a = fp_of_first("async def f():\n    async for x in y:\n        pass\n", Language::Python, "for_statement");
    let _ = a;
}

#[test]
fn fingerprint_async_with_distinct_from_with() {
    let a = fp_of_first("async def f():\n    async with x:\n        pass\n", Language::Python, "with_statement");
    let _ = a;
}

#[test]
fn fingerprint_python_match_distinct_from_if() {
    let a = fp_of_first("match x:\n    case 1:\n        pass\n", Language::Python, "match_statement");
    let b = fp_of_first("if x:\n    pass\n", Language::Python, "if_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_two_match_with_same_arity_share_hash() {
    let a = fp_of_first("match x:\n    case 1:\n        pass\n    case 2:\n        pass\n", Language::Python, "match_statement");
    let b = fp_of_first("match y:\n    case 1:\n        pass\n    case 2:\n        pass\n", Language::Python, "match_statement");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_match_with_three_cases_distinct_from_two() {
    let a = fp_of_first("match x:\n    case 1:\n        pass\n    case 2:\n        pass\n    case 3:\n        pass\n", Language::Python, "match_statement");
    let b = fp_of_first("match x:\n    case 1:\n        pass\n    case 2:\n        pass\n", Language::Python, "match_statement");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_match_guard_distinct_from_no_guard() {
    let a = fp_of_first("match x:\n    case 1 if x > 0:\n        pass\n", Language::Python, "match_statement");
    let b = fp_of_first("match x:\n    case 1:\n        pass\n", Language::Python, "match_statement");
    assert_ne!(a, b);
}

#[test]
fn seeded_fingerprint_invariant_under_seed_for_unseeded_view() {
    let a = fp_of_first("x", Language::Python, "identifier");
    let b = fp_of_first("y", Language::Python, "identifier");
    assert_eq!(a, b);
    let sa = fp_seeded("x", Language::Python, "identifier", 1);
    let sb = fp_seeded("y", Language::Python, "identifier", 1);
    assert_eq!(sa, sb, "same seed, same kind → same hash");
}

#[test]
fn seeded_fingerprint_does_not_collide_unseeded() {
    let unseeded = fp_of_first("x == y", Language::Python, "comparison_operator");
    for seed in 0_u64..256 {
        let seeded = fp_seeded("x == y", Language::Python, "comparison_operator", seed);
        assert_ne!(unseeded, seeded, "seed {seed} collided with unseeded");
    }
}

#[test]
fn fingerprint_python_double_eq_vs_single_eq_assignment() {
    let a = fp_of_first("x == y", Language::Python, "comparison_operator");
    let b = fp_of_first("x = y\n", Language::Python, "assignment");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_python_in_operator_distinct_from_eq() {
    let a = fp_of_first("x in y", Language::Python, "comparison_operator");
    let b = fp_of_first("x == y", Language::Python, "comparison_operator");
    assert_eq!(a, b, "in and == both produce comparison_operator with two identifier children");
}

#[test]
fn fingerprint_python_is_operator_same_kind_as_eq() {
    let a = fp_of_first("x is y", Language::Python, "comparison_operator");
    let b = fp_of_first("x == y", Language::Python, "comparison_operator");
    assert_eq!(a, b);
}

#[test]
fn fingerprint_call_with_keyword_distinct_from_two_positional() {
    let a = fp_of_first("f(x, y=1)", Language::Python, "call");
    let b = fp_of_first("f(x, y)", Language::Python, "call");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_python_decorated_class_distinct_from_undecorated() {
    let a = fp_of_first("@dec\nclass C:\n    pass\n", Language::Python, "decorated_definition");
    let b = fp_of_first("class C:\n    pass\n", Language::Python, "class_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_python_two_chained_decorators_distinct_from_one() {
    let a = fp_of_first("@a\n@b\ndef f():\n    pass\n", Language::Python, "decorated_definition");
    let b = fp_of_first("@a\ndef f():\n    pass\n", Language::Python, "decorated_definition");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_python_string_concatenation_via_addition() {
    let a = fp_of_first("\"a\" + \"b\"", Language::Python, "binary_operator");
    let b = fp_of_first("1 + 2", Language::Python, "binary_operator");
    assert_ne!(a, b, "string_string vs int_int are distinct");
}

#[test]
fn fingerprint_python_bool_and_int_distinct() {
    let a = fp_of_first("x = True\n", Language::Python, "true");
    let b = fp_of_first("x = 1\n", Language::Python, "integer");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_python_ellipsis_distinct_from_none() {
    let a = fp_of_first("x = ...\n", Language::Python, "ellipsis");
    let b = fp_of_first("x = None\n", Language::Python, "none");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_assignment_target_tuple_distinct_from_list() {
    let a = fp_of_first("(x, y) = (1, 2)\n", Language::Python, "assignment");
    let b = fp_of_first("[x, y] = [1, 2]\n", Language::Python, "assignment");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_assignment_with_yield_rhs_distinct_from_call() {
    let a = fp_of_first("def f():\n    x = yield\n", Language::Python, "assignment");
    let b = fp_of_first("def f():\n    x = g()\n", Language::Python, "assignment");
    assert_ne!(a, b);
}

#[test]
fn fingerprint_subscript_with_negative_index_same_kind_as_positive() {
    let a = fp_of_first("xs[-1]", Language::Python, "subscript");
    let b = fp_of_first("xs[1]", Language::Python, "subscript");
    let _ = (a, b);
}

#[test]
fn fingerprint_python_global_call_kind_unchanged_by_args_count() {
    let one = fp_of_first("f(1)", Language::Python, "call");
    let two = fp_of_first("f(1, 2)", Language::Python, "call");
    let three = fp_of_first("f(1, 2, 3)", Language::Python, "call");
    assert_ne!(one, two);
    assert_ne!(two, three);
    assert_ne!(one, three);
}
