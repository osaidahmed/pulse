
use crate::common::*;

// ===========================================================================
// Fuzzy (similar structure) detection — multiset-based skeleton hash
// Functions must be ≥20 LOC and have the same statement KIND COUNTS
// ===========================================================================

#[test]
fn similar_functions_same_shape_detected() {
    // Two Python functions with identical statement distributions
    // but different call targets and variable names
    let mut code = String::from("def func_a(data):\n");
    code.push_str("    path = get_path(data)\n    if not path:\n        return\n");
    for i in 0..15 { code.push_str(&format!("    step_{i} = transform_a(path, {i})\n")); }
    code.push_str("    return format_a(path)\n\n");
    code.push_str("def func_b(info):\n");
    code.push_str("    route = get_route(info)\n    if not route:\n        return\n");
    for i in 0..15 { code.push_str(&format!("    phase_{i} = transform_b(route, {i})\n")); }
    code.push_str("    return format_b(route)\n");
    let out = pulse_check_code(&code, "py");
    assert!(has_smell(&out, "Code Duplication"), "similar shape should be detected, got: {out}");
}

#[test]
fn exact_clones_say_identical_structure() {
    let out = pulse_check_code(
        concat!(
            "def func_a(data):\n",
            "    result = {}\n",
            "    result['id'] = data.get('id')\n",
            "    result['name'] = data.get('name')\n",
            "    result['value'] = data.get('value')\n",
            "    result['extra'] = data.get('extra')\n",
            "    return result\n",
            "\n",
            "def func_b(data):\n",
            "    result = {}\n",
            "    result['id'] = data.get('id')\n",
            "    result['name'] = data.get('name')\n",
            "    result['value'] = data.get('value')\n",
            "    result['extra'] = data.get('extra')\n",
            "    return result\n",
        ),
        "py",
    );
    assert!(has_smell(&out, "Code Duplication"), "exact clones should match, got: {out}");
    assert!(out.contains("identical structure"), "should say identical, got: {out}");
}

#[test]
fn different_shaped_functions_no_match() {
    let out = pulse_check_code(
        concat!(
            "def func_a(data):\n",
            "    if data:\n",
            "        return process(data)\n",
            "    return None\n",
            "\n",
            "def func_b(items):\n",
            "    for item in items:\n",
            "        handle(item)\n",
            "    return len(items)\n",
        ),
        "py",
    );
    assert!(!has_smell(&out, "Code Duplication"), "different shapes should not match, got: {out}");
}

#[test]
fn very_different_sizes_no_match() {
    let mut code = String::from("def big(data):\n");
    for i in 0..30 { code.push_str(&format!("    x{i} = process(data, {i})\n")); }
    code.push_str("    return x0\n\ndef small(d):\n    return process(d, 0)\n");
    let out = pulse_check_code(&code, "py");
    assert!(!out.contains("similar structure"), "very different sizes should not match");
}

#[test]
fn similar_functions_in_typescript() {
    let mut code = String::from("function processA(data: any): any {\n    const result: any = {};\n    if (!data) return null;\n");
    for i in 0..17 { code.push_str(&format!("    result.f{i} = transform_a(data, {i});\n")); }
    code.push_str("    return result;\n}\n\n");
    code.push_str("function processB(info: any): any {\n    const output: any = {};\n    if (!info) return null;\n");
    for i in 0..17 { code.push_str(&format!("    output.f{i} = transform_b(info, {i});\n")); }
    code.push_str("    return output;\n}\n");
    let out = pulse_check_code(&code, "ts");
    assert!(has_smell(&out, "Code Duplication"), "should detect similar TS functions, got: {out}");
}

#[test]
fn similar_functions_in_rust() {
    let mut code = String::from("fn process_a(data: &[i32]) -> i32 {\n    if data.is_empty() { return 0; }\n");
    for i in 0..18 { code.push_str(&format!("    let s{i} = data[{i}];\n")); }
    code.push_str("    s0\n}\n\n");
    code.push_str("fn process_b(input: &[i32]) -> i32 {\n    if input.is_empty() { return 0; }\n");
    for i in 0..18 { code.push_str(&format!("    let v{i} = input[{i}];\n")); }
    code.push_str("    v0\n}\n");
    let out = pulse_check_code(&code, "rs");
    assert!(has_smell(&out, "Code Duplication"), "should detect similar Rust functions, got: {out}");
}

#[test]
fn similar_functions_in_java() {
    let mut code = String::from("class Svc {\n    int processA(int[] d) {\n        if (d == null) return 0;\n");
    for i in 0..17 { code.push_str(&format!("        int s{i} = d[{i}];\n")); }
    code.push_str("        return s0;\n    }\n\n    int processB(int[] e) {\n        if (e == null) return 0;\n");
    for i in 0..17 { code.push_str(&format!("        int v{i} = e[{i}];\n")); }
    code.push_str("        return v0;\n    }\n}\n");
    let out = pulse_check_code(&code, "java");
    assert!(has_smell(&out, "Code Duplication"), "should detect similar Java functions, got: {out}");
}

#[test]
fn similar_functions_in_c() {
    let mut code = String::from("int process_a(int* d, int n) {\n    if (d == 0) return -1;\n");
    for i in 0..18 { code.push_str(&format!("    int s{i} = d[{i}];\n")); }
    code.push_str("    return s0;\n}\n\nint process_b(int* e, int n) {\n    if (e == 0) return -1;\n");
    for i in 0..18 { code.push_str(&format!("    int v{i} = e[{i}];\n")); }
    code.push_str("    return v0;\n}\n");
    let out = pulse_check_code(&code, "c");
    assert!(has_smell(&out, "Code Duplication"), "should detect similar C functions, got: {out}");
}

// ===========================================================================
// Test-function suppression still works
// ===========================================================================

#[test]
fn test_function_exact_duplication_suppressed() {
    let out = pulse_check_code(
        concat!(
            "def test_create_user():\n",
            "    user = create('a')\n",
            "    assert user.name == 'a'\n",
            "    assert user.active\n",
            "    assert user.role == 'user'\n",
            "    assert user.id > 0\n",
            "\n",
            "def test_create_admin():\n",
            "    user = create('b')\n",
            "    assert user.name == 'b'\n",
            "    assert user.active\n",
            "    assert user.role == 'user'\n",
            "    assert user.id > 0\n",
        ),
        "py",
    );
    assert!(!has_smell(&out, "Code Duplication"), "test duplication should be suppressed, got: {out}");
}

#[test]
fn mixed_test_and_prod_duplication_still_flagged() {
    let out = pulse_check_code(
        concat!(
            "def test_something(data):\n",
            "    result = {}\n",
            "    result['id'] = data.get('id')\n",
            "    result['name'] = data.get('name')\n",
            "    result['value'] = data.get('value')\n",
            "    result['extra'] = data.get('extra')\n",
            "    return result\n",
            "\n",
            "def process_data(data):\n",
            "    result = {}\n",
            "    result['id'] = data.get('id')\n",
            "    result['name'] = data.get('name')\n",
            "    result['value'] = data.get('value')\n",
            "    result['extra'] = data.get('extra')\n",
            "    return result\n",
        ),
        "py",
    );
    assert!(has_smell(&out, "Code Duplication"), "mixed test+prod should be flagged, got: {out}");
}

// ===========================================================================
// Short functions below skeleton threshold NOT matched
// ===========================================================================

#[test]
fn short_similar_functions_not_flagged_as_similar() {
    // 8-LOC functions share the same kind vocabulary but below skeleton_duplication_min_loc (20)
    let out = pulse_check_code(
        concat!(
            "def func_a(data):\n",
            "    path = get(data)\n",
            "    if not path:\n",
            "        return\n",
            "    source = read(path)\n",
            "    if not source:\n",
            "        return\n",
            "    return format_a(source)\n",
            "\n",
            "def func_b(info):\n",
            "    route = get(info)\n",
            "    if not route:\n",
            "        return\n",
            "    target = read(route)\n",
            "    if not target:\n",
            "        return\n",
            "    return format_b(target)\n",
        ),
        "py",
    );
    // These would have matched under the old set hash, but not under multiset + LOC floor
    assert!(!out.contains("similar structure"), "short functions should not trigger similar match, got: {out}");
}
