mod common;

use common::*;

// ===========================================================================
// Fuzzy (similar structure) detection
// ===========================================================================

#[test]
fn similar_functions_with_extra_line_detected() {
    let out = pulse_check_code(concat!(
        "def func_a(data):\n",
        "    path = get(data)\n",
        "    if not path:\n",
        "        return\n",
        "    source = read(path)\n",
        "    if not source:\n",
        "        return\n",
        "    result = process(source)\n",
        "    return format_a(result)\n",
        "\n",
        "def func_b(data):\n",
        "    path = get(data)\n",
        "    if not path:\n",
        "        return\n",
        "    source = read(path)\n",
        "    if not source:\n",
        "        return\n",
        "    result = process(source)\n",
        "    extra = filter(result)\n",
        "    return format_b(extra)\n",
    ), "py");
    assert!(has_smell(&out, "Code Duplication"), "similar functions should be detected, got: {}", out);
    assert!(out.contains("similar structure"), "should say 'similar structure', got: {}", out);
}

#[test]
fn exact_clones_say_identical_structure() {
    let out = pulse_check_code(concat!(
        "def func_a(data):\n",
        "    r = {}\n",
        "    r['id'] = data.get('id')\n",
        "    r['name'] = data.get('name')\n",
        "    r['val'] = data.get('val')\n",
        "    r['ok'] = data.get('ok')\n",
        "    return r\n",
        "\n",
        "def func_b(items):\n",
        "    r = {}\n",
        "    r['id'] = items.get('id')\n",
        "    r['name'] = items.get('name')\n",
        "    r['val'] = items.get('val')\n",
        "    r['ok'] = items.get('ok')\n",
        "    return r\n",
    ), "py");
    assert!(out.contains("identical structure"), "exact clones should say 'identical structure', got: {}", out);
}

#[test]
fn similar_functions_with_different_sizes_not_flagged() {
    // func_a is 8 lines, func_b is 20 lines — ratio > 1.3, should NOT match as similar
    let out = pulse_check_code(concat!(
        "def func_a(data):\n",
        "    path = get(data)\n",
        "    if not path:\n",
        "        return\n",
        "    source = read(path)\n",
        "    if not source:\n",
        "        return\n",
        "    return source\n",
        "\n",
        "def func_b(data):\n",
        "    path = get(data)\n",
        "    if not path:\n",
        "        return\n",
        "    source = read(path)\n",
        "    if not source:\n",
        "        return\n",
        "    result = process(source)\n",
        "    validated = validate(result)\n",
        "    transformed = transform(validated)\n",
        "    enriched = enrich(transformed)\n",
        "    formatted = fmt(enriched)\n",
        "    logged = log(formatted)\n",
        "    cached = cache(logged)\n",
        "    return cached\n",
    ), "py");
    assert!(!out.contains("similar structure"), "very different sizes should not match, got: {}", out);
}

#[test]
fn similar_functions_in_typescript() {
    let out = pulse_check_code(concat!(
        "function processA(data: any): any {\n",
        "    const result: any = {};\n",
        "    if (!data) return null;\n",
        "    result.id = data.id;\n",
        "    result.name = data.name;\n",
        "    return result;\n",
        "}\n",
        "\n",
        "function processB(data: any): any {\n",
        "    const result: any = {};\n",
        "    if (!data) return null;\n",
        "    result.id = data.id;\n",
        "    result.name = data.name;\n",
        "    result.extra = data.extra;\n",
        "    return result;\n",
        "}\n",
    ), "ts");
    assert!(has_smell(&out, "Code Duplication"), "should detect similar TS functions, got: {}", out);
}

#[test]
fn similar_functions_in_rust() {
    let out = pulse_check_code(concat!(
        "fn process_a(data: &Data) -> Result<Output, Error> {\n",
        "    let raw = data.fetch()?;\n",
        "    if raw.is_empty() {\n",
        "        return Err(Error::Empty);\n",
        "    }\n",
        "    let parsed = parse(raw)?;\n",
        "    Ok(format_a(parsed))\n",
        "}\n",
        "\n",
        "fn process_b(data: &Data) -> Result<Output, Error> {\n",
        "    let raw = data.fetch()?;\n",
        "    if raw.is_empty() {\n",
        "        return Err(Error::Empty);\n",
        "    }\n",
        "    let parsed = parse(raw)?;\n",
        "    let filtered = filter(parsed);\n",
        "    Ok(format_b(filtered))\n",
        "}\n",
    ), "rs");
    assert!(has_smell(&out, "Code Duplication"), "should detect similar Rust functions, got: {}", out);
}

#[test]
fn similar_functions_in_java() {
    let out = pulse_check_code(concat!(
        "class Service {\n",
        "    String processA(Data data) {\n",
        "        if (data == null) return null;\n",
        "        String raw = data.fetch();\n",
        "        if (raw.isEmpty()) return null;\n",
        "        String parsed = parse(raw);\n",
        "        return formatA(parsed);\n",
        "    }\n",
        "\n",
        "    String processB(Data data) {\n",
        "        if (data == null) return null;\n",
        "        String raw = data.fetch();\n",
        "        if (raw.isEmpty()) return null;\n",
        "        String parsed = parse(raw);\n",
        "        String filtered = filter(parsed);\n",
        "        return formatB(filtered);\n",
        "    }\n",
        "}\n",
    ), "java");
    assert!(has_smell(&out, "Code Duplication"), "should detect similar Java functions, got: {}", out);
}

#[test]
fn similar_functions_in_c() {
    let out = pulse_check_code(concat!(
        "int process_a(int* data, int n) {\n",
        "    if (data == NULL) return -1;\n",
        "    int sum = 0;\n",
        "    for (int i = 0; i < n; i++) {\n",
        "        sum += data[i];\n",
        "    }\n",
        "    return format_a(sum);\n",
        "}\n",
        "\n",
        "int process_b(int* data, int n) {\n",
        "    if (data == NULL) return -1;\n",
        "    int sum = 0;\n",
        "    for (int i = 0; i < n; i++) {\n",
        "        sum += data[i];\n",
        "    }\n",
        "    int filtered = filter(sum);\n",
        "    return format_b(filtered);\n",
        "}\n",
    ), "c");
    assert!(has_smell(&out, "Code Duplication"), "should detect similar C functions, got: {}", out);
}

// ===========================================================================
// Test-function suppression still works for both exact and similar
// ===========================================================================

#[test]
fn test_function_exact_duplication_suppressed() {
    let out = pulse_check_code(concat!(
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
    ), "py");
    assert!(!has_smell(&out, "Code Duplication"), "test function duplication should be suppressed, got: {}", out);
}

#[test]
fn mixed_test_and_prod_duplication_still_flagged() {
    let out = pulse_check_code(concat!(
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
    ), "py");
    assert!(has_smell(&out, "Code Duplication"), "mixed test+prod duplication should be flagged, got: {}", out);
}
