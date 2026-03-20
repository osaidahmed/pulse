mod common;

use common::*;
use std::process::Command;

const LANG: &str = "python";

fn pulse_check(code: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(&path, code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    String::from_utf8(out.stdout).unwrap()
}

fn pulse_debug(code: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(&path, code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    String::from_utf8(out.stderr).unwrap()
}

// ===========================================================================
// CC counting precision
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = pulse_debug("def f():\n    if x:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_elif() {
    let out = pulse_debug("def f():\n    if x:\n        pass\n    elif y:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = pulse_debug("def f():\n    for x in y:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = pulse_debug("def f():\n    while x:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_except() {
    let out = pulse_debug("def f():\n    try:\n        pass\n    except:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and_operator() {
    let out = pulse_debug("def f():\n    if a and b:\n        pass\n");
    // base(1) + if(1) + and(1) = 3
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or_operator() {
    let out = pulse_debug("def f():\n    if a or b:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_not_operator() {
    let out = pulse_debug("def f():\n    if not a:\n        pass\n");
    // base(1) + if(1) + not(1) = 3
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_ternary() {
    let out = pulse_debug("def f():\n    x = 1 if a else 2\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_chained_boolean() {
    let out = pulse_debug("def f():\n    if a and b and c and d:\n        pass\n");
    // base(1) + if(1) + 3 boolean_operators (a and b) and c) and d) = 5
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "chained boolean should increase cc, got: {}", cc);
}

#[test]
fn cc_multiple_except_clauses() {
    let out = pulse_debug(
        "def f():\n    try:\n        pass\n    except ValueError:\n        pass\n    except TypeError:\n        pass\n    except:\n        pass\n",
    );
    // base(1) + 3 excepts = 4
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_nested_if_in_for() {
    let out = pulse_debug("def f():\n    for x in y:\n        if x:\n            pass\n");
    // base(1) + for(1) + if(1) = 3
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// ===========================================================================
// Nesting depth precision
// ===========================================================================

#[test]
fn nesting_depth_0_for_flat_function() {
    let out = pulse_debug("def f():\n    return 1\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_depth_1_for_single_if() {
    let out = pulse_debug("def f():\n    if x:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_2_for_nested_if() {
    let out = pulse_debug("def f():\n    if x:\n        if y:\n            pass\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_depth_tracks_for_in_if() {
    let out = pulse_debug("def f():\n    if x:\n        for i in y:\n            if z:\n                pass\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_depth_with_counts_depth() {
    let out = pulse_debug("def f():\n    with open('f') as fh:\n        if x:\n            pass\n");
    // with(1) + if(1) = 2
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

// ===========================================================================
// Argument counting precision
// ===========================================================================

#[test]
fn args_counts_positional() {
    let out = pulse_debug("def f(a, b, c):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_default_values() {
    let out = pulse_debug("def f(a, b=1, c=None):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_splats() {
    let out = pulse_debug("def f(a, *args, **kwargs):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_typed_params() {
    let out = pulse_debug("def f(a: int, b: str, c: float):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_typed_with_defaults() {
    let out = pulse_debug("def f(a: int = 0, b: str = ''):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

#[test]
fn args_excludes_self_in_method() {
    let out = pulse_debug("class C:\n    def m(self, a, b):\n        pass\n");
    assert_eq!(function_metric(&out, "C.m", "args"), Some(2));
}

#[test]
fn args_excludes_cls_in_classmethod() {
    let out = pulse_debug("class C:\n    @classmethod\n    def m(cls, a):\n        pass\n");
    assert_eq!(function_metric(&out, "C.m", "args"), Some(1));
}

#[test]
fn args_zero_for_no_params() {
    let out = pulse_debug("def f():\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

// ===========================================================================
// Primitive obsession precision
// ===========================================================================

#[test]
fn primitive_obsession_all_str() {
    let out = pulse_check("def f(a: str, b: str, c: str, d: str, e: str):\n    pass\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_mixed_below_threshold() {
    // 2 out of 4 typed = 50%, below 70% threshold
    let out = pulse_check("def f(a: str, b: str, c: MyObj, d: OtherObj):\n    pass\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_untyped_params_not_counted() {
    // untyped params don't count toward the ratio
    let out = pulse_check("def f(a, b, c, d, e):\n    pass\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_below_min_typed_params() {
    // only 3 typed params (threshold is 4), even though all are primitive
    let out = pulse_check("def f(a: str, b: int, c: float):\n    pass\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_recognizes_bool() {
    let out = pulse_check("def f(a: bool, b: bool, c: bool, d: bool):\n    pass\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_recognizes_int_float_bytes() {
    let out = pulse_check("def f(a: int, b: float, c: bytes, d: complex):\n    pass\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4 precision
// ===========================================================================

#[test]
fn lcom4_single_method_class_not_flagged() {
    let out = pulse_check(
        "class Tiny:\n    def __init__(self):\n        self.x = 1\n    def get(self):\n        return self.x\n",
    );
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_all_methods_share_field() {
    let out = pulse_check(
        "class Cohesive:\n    def __init__(self):\n        self.data = []\n    def add(self, x):\n        self.data.append(x)\n    def get(self):\n        return self.data\n    def clear(self):\n        self.data = []\n",
    );
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_two_disconnected_groups() {
    // methods split into 2 groups: (a_work, a_read) share field_a, (b_work, b_read) share field_b
    let out = pulse_check(
        "class Split:\n    def __init__(self):\n        self.field_a = 1\n        self.field_b = 2\n    def a_work(self):\n        return self.field_a\n    def a_read(self):\n        return self.field_a + 1\n    def b_work(self):\n        return self.field_b\n    def b_read(self):\n        return self.field_b + 1\n",
    );
    // 2 components, threshold is 3 — should NOT trigger
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_three_disconnected_groups() {
    let out = pulse_check(
        "class Messy:\n    def __init__(self):\n        self.x = 1\n        self.y = 2\n        self.z = 3\n    def use_x(self):\n        return self.x\n    def use_y(self):\n        return self.y\n    def use_z(self):\n        return self.z\n",
    );
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_init_excluded_from_analysis() {
    // __init__ accesses all fields but shouldn't count as connecting them
    let out = pulse_check(
        "class Init:\n    def __init__(self):\n        self.a = 1\n        self.b = 2\n        self.c = 3\n    def use_a(self):\n        return self.a\n    def use_b(self):\n        return self.b\n    def use_c(self):\n        return self.c\n",
    );
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_transitive_connection() {
    // m1 uses field_a, m2 uses field_a+field_b, m3 uses field_b
    // m1-m2 connected via field_a, m2-m3 connected via field_b -> all in one component
    let out = pulse_check(
        "class Connected:\n    def __init__(self):\n        self.a = 1\n        self.b = 2\n    def m1(self):\n        return self.a\n    def m2(self):\n        return self.a + self.b\n    def m3(self):\n        return self.b\n",
    );
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Code duplication edge cases
// ===========================================================================

#[test]
fn duplication_decorators_dont_affect_hash() {
    // same body, different decorators — should be flagged as duplicates
    let out = pulse_check(r#"
def decorator_a(f):
    return f
def decorator_b(f):
    return f

@decorator_a
def func_a(data):
    result = {}
    result["id"] = data.get("id", 0)
    result["name"] = data.get("name", "")
    result["value"] = data.get("value", 0)
    result["active"] = data.get("active", True)
    return result

@decorator_b
def func_b(data):
    result = {}
    result["id"] = data.get("id", 0)
    result["name"] = data.get("name", "")
    result["value"] = data.get("value", 0)
    result["active"] = data.get("active", True)
    return result
"#);
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_async_vs_sync_same_body() {
    // async and sync versions with same body
    let out = pulse_check(r#"
async def fetch_a(url):
    result = {}
    result["data"] = await get(url)
    result["status"] = "ok"
    result["timestamp"] = now()
    result["source"] = url
    return result

async def fetch_b(endpoint):
    result = {}
    result["data"] = await get(endpoint)
    result["status"] = "ok"
    result["timestamp"] = now()
    result["source"] = endpoint
    return result
"#);
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_two_functions_is_minimum_group() {
    let out = pulse_check(r#"
def report_a(data):
    r = {}
    r["id"] = data.get("id")
    r["name"] = data.get("name")
    r["value"] = data.get("value")
    r["status"] = "active"
    return r

def report_b(items):
    r = {}
    r["id"] = items.get("id")
    r["name"] = items.get("name")
    r["value"] = items.get("value")
    r["status"] = "active"
    return r
"#);
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_mixed_test_and_prod_still_flagged() {
    // if one function is NOT a test_, the group should still be flagged
    let out = pulse_check(r#"
def test_something(data):
    result = {}
    result["id"] = data.get("id")
    result["name"] = data.get("name")
    result["value"] = data.get("value")
    result["extra"] = data.get("extra")
    return result

def process_data(data):
    result = {}
    result["id"] = data.get("id")
    result["name"] = data.get("name")
    result["value"] = data.get("value")
    result["extra"] = data.get("extra")
    return result
"#);
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// God Method / God Class interaction
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    // large file + many functions but NO god method -> no God Class
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("def fn_{}():\n    return {}\n\n", i, i));
    }
    // pad LOC
    for i in 0..200 {
        code.push_str(&format!("VAR_{} = {}\n", i, i));
    }
    let out = pulse_check(&code);
    assert!(!has_smell(&out, "God Class"));
}

#[test]
fn god_method_triggers_god_class_when_file_is_large_with_many_functions() {
    // God method + large file + many functions = God Class
    let mut code = String::new();
    // Generate a god method (cc >= 9 AND loc >= 50)
    code.push_str("def monster():\n");
    for i in 0..10 {
        code.push_str(&format!("    if x == {}:\n        pass\n", i));
    }
    for i in 0..40 {
        code.push_str(&format!("    y_{} = {}\n", i, i));
    }
    code.push_str("    return None\n\n");
    // 20+ more functions
    for i in 0..21 {
        code.push_str(&format!("def fn_{}():\n    return {}\n\n", i, i));
    }
    // pad LOC well above 400 threshold
    for i in 0..350 {
        code.push_str(&format!("VAR_{} = {}\n", i, i));
    }
    let out = pulse_check(&code);
    assert!(has_smell(&out, "God Method"));
    assert!(has_smell(&out, "God Class"));
}

// ===========================================================================
// Assertion block edge cases
// ===========================================================================

#[test]
fn assertion_block_interrupted_by_code_resets_count() {
    // asserts broken by a non-assert statement
    let out = pulse_check(r#"
def test_interleaved():
    assert x == 1
    assert y == 2
    assert z == 3
    do_something()
    assert a == 4
    assert b == 5
"#);
    // max consecutive is 3, below threshold of 10
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_exactly_at_threshold() {
    let mut code = "def test_exact():\n".to_string();
    for i in 0..10 {
        code.push_str(&format!("    assert x_{} == {}\n", i, i));
    }
    let out = pulse_check(&code);
    // 10 consecutive, threshold is > 10 (so 11+ to trigger)
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_above_threshold() {
    let mut code = "def test_big():\n".to_string();
    for i in 0..15 {
        code.push_str(&format!("    assert x_{} == {}\n", i, i));
    }
    let out = pulse_check(&code);
    assert!(has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Overall Function Size edge cases
// ===========================================================================

#[test]
fn overall_function_size_not_triggered_by_two_large_functions() {
    // threshold is 3 large functions, having 2 should not trigger
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("def large_fn_{}():\n", i));
        for j in 0..45 {
            code.push_str(&format!("    x_{} = {}\n", j, j));
        }
        code.push_str("    return None\n\n");
    }
    let out = pulse_check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

#[test]
fn overall_function_size_triggered_by_three_large_functions() {
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("def large_fn_{}():\n", i));
        for j in 0..45 {
            code.push_str(&format!("    x_{} = {}\n", j, j));
        }
        code.push_str("    return None\n\n");
    }
    let out = pulse_check(&code);
    assert!(has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Declarations edge cases
// ===========================================================================

#[test]
fn declarations_below_threshold_not_flagged() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class T{}:\n    pass\n\n", i));
    }
    let out = pulse_check(&code);
    assert!(!has_smell(&out, "Declarations"));
}

#[test]
fn decorated_classes_counted_as_declarations() {
    let mut code = "def deco(cls):\n    return cls\n\n".to_string();
    for i in 0..25 {
        code.push_str(&format!("@deco\nclass T{}:\n    pass\n\n", i));
    }
    let out = pulse_check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// Embedded block edge cases
// ===========================================================================

#[test]
fn small_string_not_flagged_as_embedded() {
    let out = pulse_check("def f():\n    x = 'hello world'\n    return x\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn multiline_fstring_counted_as_embedded() {
    let mut code = "def f():\n    x = f\"\"\"\n".to_string();
    for i in 0..20 {
        code.push_str(&format!("        line {} of template\n", i));
    }
    code.push_str("    \"\"\"\n    return x\n");
    let out = pulse_check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Deep global nesting edge cases
// ===========================================================================

#[test]
fn shallow_global_if_not_flagged_deep() {
    let out = pulse_check("if True:\n    x = 1\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

#[test]
fn global_nesting_depth_3_flagged() {
    let out = pulse_check("if a:\n    if b:\n        if c:\n            x = 1\n");
    assert!(has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Constructor over-injection vs excess args
// ===========================================================================

#[test]
fn constructor_reports_over_injection_not_excess_args() {
    let out = pulse_check(
        "class S:\n    def __init__(self, a, b, c, d, e, f):\n        pass\n",
    );
    assert!(has_smell(&out, "Constructor Over-Injection"));
    // Should say "Constructor Over-Injection", not "Excess Arguments"
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("__init__")).collect();
    assert!(lines.iter().any(|l| l.contains("Constructor Over-Injection")));
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

#[test]
fn regular_function_reports_excess_args_not_constructor() {
    let out = pulse_check("def f(a, b, c, d, e, f, g):\n    pass\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Multiple smells on same function
// ===========================================================================

#[test]
fn function_can_have_multiple_smells() {
    let out = pulse_check(r#"
def terrible(a, b, c, d, e, f, g, h):
    query = """
        SELECT *
        FROM users
        WHERE id = 1
        AND name = 'test'
        AND email = 'x'
        AND phone = 'y'
        AND addr = 'z'
        AND city = 'c'
        AND state = 's'
        AND zip = 'z'
        AND country = 'c'
        AND role = 'r'
        AND dept = 'd'
        AND team = 't'
        AND mgr = 'm'
        AND loc = 'l'
        AND tz = 't'
        AND lang = 'l'
    """
    for row in query:
        if row:
            for col in row:
                if col:
                    for val in col:
                        if val:
                            process(val)
    return None
"""#,
    );
    // Should flag: Excess Arguments, Large Embedded Block, Deep Nested Complexity
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(has_smell(&out, "Large Embedded Block"));
    assert!(has_smell(&out, "Deep Nested"));
}

// ===========================================================================
// Real-world patterns: Django views
// ===========================================================================

#[test]
fn clean_django_view_not_flagged() {
    let out = pulse_check(r#"
class ItemListView:
    def get_queryset(self):
        return self.model.objects.filter(active=True)

    def get_context_data(self, **kwargs):
        context = super().get_context_data(**kwargs)
        context["title"] = "Items"
        return context
"#);
    assert!(out.is_empty(), "clean Django view should not be flagged, got: {}", out);
}

// ===========================================================================
// Real-world patterns: pytest fixtures
// ===========================================================================

#[test]
fn pytest_fixture_parametrize_not_flagged() {
    let out = pulse_check(r#"
import pytest

@pytest.fixture
def user():
    return {"name": "test", "email": "test@test.com"}

@pytest.fixture
def admin():
    return {"name": "admin", "email": "admin@test.com"}

def test_user_name(user):
    assert user["name"] == "test"

def test_admin_name(admin):
    assert admin["name"] == "admin"
"#);
    // Small functions, few assertions — nothing should trigger
    assert!(out.is_empty(), "pytest fixture pattern should not be flagged, got: {}", out);
}

// ===========================================================================
// Hook JSON edge cases
// ===========================================================================

#[test]
fn hook_missing_tool_input_key() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"{\"other_key\": 1}")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

#[test]
fn hook_missing_file_path_key() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"{\"tool_input\": {\"content\": \"hello\"}}")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

#[test]
fn hook_empty_stdin() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            drop(child.stdin.take());
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

// ===========================================================================
// Performance: large generated file
// ===========================================================================

#[test]
fn performance_1000_loc_file() {
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("def func_{}(data):\n", i));
        for j in 0..18 {
            code.push_str(&format!("    x_{} = data.get(\"field_{}\")\n", j, j));
        }
        code.push_str("    return data\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.py");
    std::fs::write(&path, &code).unwrap();

    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 500,
        "1000 LOC / 50 functions should complete under 500ms, took: {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn performance_deeply_nested_class_hierarchy() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class Service{}:\n", i));
        code.push_str(&format!("    def __init__(self):\n        self.data_{} = []\n\n", i));
        for j in 0..5 {
            code.push_str(&format!("    def method_{}(self):\n        return self.data_{}\n\n", j, i));
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("classes.py");
    std::fs::write(&path, &code).unwrap();

    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 500,
        "10 classes with LCOM4 analysis should complete under 500ms, took: {}ms",
        elapsed.as_millis()
    );
}
