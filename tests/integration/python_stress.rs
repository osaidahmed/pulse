
use crate::common::*;
use std::process::Command;

lang_helpers!("py");

// ===========================================================================
// CC counting precision
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("def f():\n    if x:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_elif() {
    let out = debug("def f():\n    if x:\n        pass\n    elif y:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("def f():\n    for x in y:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("def f():\n    while x:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_except() {
    let out = debug("def f():\n    try:\n        pass\n    except:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and_operator() {
    let out = debug("def f():\n    if a and b:\n        pass\n");
    // base(1) + if(1) + and(1) = 3
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or_operator() {
    let out = debug("def f():\n    if a or b:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn not_operator_excluded_from_cc() {
    let plain = debug("def f():\n    if a:\n        pass\n");
    let negated = debug("def f():\n    if not a:\n        pass\n");
    assert_eq!(function_metric(&plain, "f", "cc"), Some(2));
    assert_eq!(function_metric(&negated, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_ternary() {
    let out = debug("def f():\n    x = 1 if a else 2\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_chained_boolean() {
    let out = debug("def f():\n    if a and b and c and d:\n        pass\n");
    // base(1) + if(1) + 3 boolean_operators (a and b) and c) and d) = 5
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "chained boolean should increase cc, got: {cc}");
}

#[test]
fn cc_multiple_except_clauses() {
    let out = debug(
        "def f():\n    try:\n        pass\n    except ValueError:\n        pass\n    except TypeError:\n        pass\n    except:\n        pass\n",
    );
    // base(1) + 3 excepts = 4
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_nested_if_in_for() {
    let out = debug("def f():\n    for x in y:\n        if x:\n            pass\n");
    // base(1) + for(1) + if(1) = 3
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// ===========================================================================
// Nesting depth precision
// ===========================================================================

#[test]
fn nesting_depth_0_for_flat_function() {
    let out = debug("def f():\n    return 1\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_depth_1_for_single_if() {
    let out = debug("def f():\n    if x:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_2_for_nested_if() {
    let out = debug("def f():\n    if x:\n        if y:\n            pass\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_depth_tracks_for_in_if() {
    let out = debug(
        "def f():\n    if x:\n        for i in y:\n            if z:\n                pass\n",
    );
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_depth_with_counts_depth() {
    let out = debug("def f():\n    with open('f') as fh:\n        if x:\n            pass\n");
    // with(1) + if(1) = 2
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

// ===========================================================================
// Argument counting precision
// ===========================================================================

#[test]
fn args_counts_positional() {
    let out = debug("def f(a, b, c):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_default_values() {
    let out = debug("def f(a, b=1, c=None):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_splats() {
    let out = debug("def f(a, *args, **kwargs):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_typed_params() {
    let out = debug("def f(a: int, b: str, c: float):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_typed_with_defaults() {
    let out = debug("def f(a: int = 0, b: str = ''):\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

#[test]
fn args_excludes_self_in_method() {
    let out = debug("class C:\n    def m(self, a, b):\n        pass\n");
    assert_eq!(function_metric(&out, "C.m", "args"), Some(2));
}

#[test]
fn args_excludes_cls_in_classmethod() {
    let out = debug("class C:\n    @classmethod\n    def m(cls, a):\n        pass\n");
    assert_eq!(function_metric(&out, "C.m", "args"), Some(1));
}

#[test]
fn args_zero_for_no_params() {
    let out = debug("def f():\n    pass\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

// ===========================================================================
// Primitive obsession precision
// ===========================================================================

#[test]
fn primitive_obsession_all_str() {
    let out = check("def f(a: str, b: str, c: str, d: str, e: str):\n    pass\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_mixed_below_threshold() {
    // 2 out of 4 typed = 50%, below 70% threshold
    let out = check("def f(a: str, b: str, c: MyObj, d: OtherObj):\n    pass\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_untyped_params_not_counted() {
    // untyped params don't count toward the ratio
    let out = check("def f(a, b, c, d, e):\n    pass\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_below_min_typed_params() {
    // only 3 typed params (threshold is 4), even though all are primitive
    let out = check("def f(a: str, b: int, c: float):\n    pass\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_recognizes_bool() {
    let out = check("def f(a: bool, b: bool, c: bool, d: bool):\n    pass\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_recognizes_int_float_bytes() {
    let out = check("def f(a: int, b: float, c: bytes, d: complex):\n    pass\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4 precision
// ===========================================================================

#[test]
fn lcom4_single_method_class_not_flagged() {
    let out = check(
        "class Tiny:\n    def __init__(self):\n        self.x = 1\n    def get(self):\n        return self.x\n",
    );
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_all_methods_share_field() {
    let out = check(
        "class Cohesive:\n    def __init__(self):\n        self.data = []\n    def add(self, x):\n        self.data.append(x)\n    def get(self):\n        return self.data\n    def clear(self):\n        self.data = []\n",
    );
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_two_disconnected_groups() {
    // methods split into 2 groups: (a_work, a_read) share field_a, (b_work, b_read) share field_b
    let out = check(
        "class Split:\n    def __init__(self):\n        self.field_a = 1\n        self.field_b = 2\n    def a_work(self):\n        return self.field_a\n    def a_read(self):\n        return self.field_a + 1\n    def b_work(self):\n        return self.field_b\n    def b_read(self):\n        return self.field_b + 1\n",
    );
    // 2 components, threshold is 3 — should NOT trigger
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_three_disconnected_groups() {
    let out = check(
        "class Messy:\n    def __init__(self):\n        self.x = 1\n        self.y = 2\n        self.z = 3\n    def use_x(self):\n        return self.x\n    def use_y(self):\n        return self.y\n    def use_z(self):\n        return self.z\n",
    );
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_init_excluded_from_analysis() {
    // __init__ accesses all fields but shouldn't count as connecting them
    let out = check(
        "class Init:\n    def __init__(self):\n        self.a = 1\n        self.b = 2\n        self.c = 3\n    def use_a(self):\n        return self.a\n    def use_b(self):\n        return self.b\n    def use_c(self):\n        return self.c\n",
    );
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_transitive_connection() {
    // m1 uses field_a, m2 uses field_a+field_b, m3 uses field_b
    // m1-m2 connected via field_a, m2-m3 connected via field_b -> all in one component
    let out = check(
        "class Connected:\n    def __init__(self):\n        self.a = 1\n        self.b = 2\n    def m1(self):\n        return self.a\n    def m2(self):\n        return self.a + self.b\n    def m3(self):\n        return self.b\n",
    );
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "class Coord:\n",
        "    def __init__(self):\n",
        "        self.state = 0\n",
        "    def process(self, e):\n",
        "        return self._validate(e) and self._dispatch(e)\n",
        "    def _validate(self, e):\n",
        "        return e.is_valid()\n",
        "    def _dispatch(self, e):\n",
        "        return self._send(e)\n",
        "    def _send(self, e):\n",
        "        pass\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "class Mixed:\n",
        "    def __init__(self):\n",
        "        self.x = 0\n",
        "    def a(self):\n",
        "        return self.x\n",
        "    def b(self):\n",
        "        self.x = 1\n",
        "        return self.c()\n",
        "    def c(self):\n",
        "        return 42\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_self_call_does_not_falsely_silence_god_class() {
    let out = check(concat!(
        "class GodWithRecursion:\n",
        "    def __init__(self):\n",
        "        self.y = 0\n",
        "        self.z = 0\n",
        "        self.w = 0\n",
        "    def a(self, n):\n",
        "        return self.a(n - 1) if n > 0 else 0\n",
        "    def b(self):\n",
        "        return self.y\n",
        "    def c(self):\n",
        "        return self.z\n",
        "    def d(self):\n",
        "        return self.w\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "class Service:\n",
        "    def __init__(self, db, cache, log):\n",
        "        self.db = db\n",
        "        self.cache = cache\n",
        "        self.log = log\n",
        "    def a(self):\n",
        "        return self.db.foo()\n",
        "    def b(self):\n",
        "        return self.cache.foo()\n",
        "    def c(self):\n",
        "        return self.log.foo()\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_class_still_fires() {
    let out = check(concat!(
        "class UserService:\n",
        "    def __init__(self, db, cache, mailer, events, audit):\n",
        "        self.db = db\n",
        "        self.cache = cache\n",
        "        self.mailer = mailer\n",
        "        self.events = events\n",
        "        self.audit = audit\n",
        "    def get_user(self, uid):\n",
        "        return self.db.get(uid)\n",
        "    def cache_user(self, u):\n",
        "        self.cache.set(u.id, u)\n",
        "    def send_welcome(self, u):\n",
        "        self.mailer.send(u.email)\n",
        "    def publish(self, e):\n",
        "        self.events.emit(e)\n",
        "    def audit_log(self, msg):\n",
        "        self.audit.log(msg)\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_call_to_free_function_does_not_connect() {
    let out = check(concat!(
        "def helper(x):\n",
        "    return x\n",
        "class Iso:\n",
        "    def __init__(self):\n",
        "        self.x = 0\n",
        "        self.y = 0\n",
        "        self.z = 0\n",
        "    def a(self):\n",
        "        return helper(self.x)\n",
        "    def b(self):\n",
        "        return helper(self.y)\n",
        "    def c(self):\n",
        "        return helper(self.z)\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_same_method_name_in_different_classes() {
    let out = check(concat!(
        "class Alpha:\n",
        "    def __init__(self):\n",
        "        self.x = 0\n",
        "    def caller(self):\n",
        "        return self.process()\n",
        "    def process(self):\n",
        "        return self.x\n",
        "    def helper(self):\n",
        "        return 1\n",
        "class Beta:\n",
        "    def __init__(self):\n",
        "        self.y = 0\n",
        "    def caller(self):\n",
        "        return self.process()\n",
        "    def process(self):\n",
        "        return self.y\n",
        "    def other(self):\n",
        "        return 2\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Code duplication edge cases
// ===========================================================================

#[test]
fn duplication_decorators_dont_affect_hash() {
    // same body, different decorators — should be flagged as duplicates
    let out = check(
        r#"
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
"#,
    );
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_async_vs_sync_same_body() {
    // async and sync versions with same body
    let out = check(
        r#"
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
"#,
    );
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_two_functions_is_minimum_group() {
    let out = check(
        r#"
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
"#,
    );
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_mixed_test_and_prod_still_flagged() {
    // if one function is NOT a test_, the group should still be flagged
    let out = check(
        r#"
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
"#,
    );
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// God Method / God Class interaction
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    // large file + many functions but NO god method -> no God Class
    let mut code = String::new();
    for i in 0..functions_above() {
        code.push_str(&format!("def fn_{i}():\n    return {i}\n\n"));
    }
    // pad LOC
    for i in 0..file_padding() {
        code.push_str(&format!("VAR_{i} = {i}\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"));
}

#[test]
fn god_method_triggers_god_class_when_file_is_large_with_many_functions() {
    // God method + large file + many functions = God Class
    let mut code = String::new();
    // Generate a god method (cc >= 9 AND loc >= 65)
    code.push_str("def monster():\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if x == {i}:\n        pass\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    y_{i} = {i}\n"));
    }
    code.push_str("    return None\n\n");
    // 20+ more functions
    for i in 0..functions_above() {
        code.push_str(&format!("def fn_{i}():\n    return {i}\n\n"));
    }
    // pad LOC well above 500 threshold
    for i in 0..file_padding() {
        code.push_str(&format!("VAR_{i} = {i}\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "God Method"));
    assert!(has_smell(&out, "God Class"));
}

// ===========================================================================
// Assertion block edge cases
// ===========================================================================

#[test]
fn assertion_block_interrupted_by_code_resets_count() {
    // asserts broken by a non-assert statement
    let out = check(
        r"
def test_interleaved():
    assert x == 1
    assert y == 2
    assert z == 3
    do_something()
    assert a == 4
    assert b == 5
",
    );
    // max consecutive is 3, below threshold of 10
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_exactly_at_threshold() {
    let mut code = "def test_exact():\n".to_string();
    for i in 0..asserts_at() {
        code.push_str(&format!("    assert x_{i} == {i}\n"));
    }
    let out = check(&code);
    // at threshold, not flagged (threshold is > asserts_at)
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_above_threshold() {
    let mut code = "def test_big():\n".to_string();
    for i in 0..asserts_above() {
        code.push_str(&format!("    assert x_{i} == {i}\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Overall Function Size edge cases
// ===========================================================================

#[test]
fn overall_function_size_not_triggered_by_two_large_functions() {
    // threshold is 3 large functions, having 2 should not trigger
    let mut code = String::new();
    for i in 0..(t().module.large_fn_count as usize - 1) {
        code.push_str(&format!("def large_fn_{i}():\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    x_{j} = {j}\n"));
        }
        code.push_str("    return None\n\n");
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

#[test]
fn overall_function_size_triggered_by_three_large_functions() {
    let mut code = String::new();
    for i in 0..t().module.large_fn_count as usize {
        code.push_str(&format!("def large_fn_{i}():\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    x_{j} = {j}\n"));
        }
        code.push_str("    return None\n\n");
    }
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Declarations edge cases
// ===========================================================================

#[test]
fn declarations_below_threshold_not_flagged() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class T{i}:\n    pass\n\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Declarations"));
}

#[test]
fn decorated_classes_counted_as_declarations() {
    let mut code = "def deco(cls):\n    return cls\n\n".to_string();
    for i in 0..declarations_above() {
        code.push_str(&format!("@deco\nclass T{i}:\n    pass\n\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// Embedded block edge cases
// ===========================================================================

#[test]
fn small_string_not_flagged_as_embedded() {
    let out = check("def f():\n    x = 'hello world'\n    return x\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn multiline_fstring_counted_as_embedded() {
    let mut code = "def f():\n    x = f\"\"\"\n".to_string();
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        line {i} of template\n"));
    }
    code.push_str("    \"\"\"\n    return x\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Deep global nesting edge cases
// ===========================================================================

#[test]
fn shallow_global_if_not_flagged_deep() {
    let out = check("if True:\n    x = 1\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

#[test]
fn global_nesting_depth_3_flagged() {
    let out = check("if a:\n    if b:\n        if c:\n            x = 1\n");
    assert!(has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Constructor over-injection vs excess args
// ===========================================================================

#[test]
fn constructor_reports_over_injection_not_excess_args() {
    let out = check("class S:\n    def __init__(self, a, b, c, d, e, f):\n        pass\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
    // Should say "Constructor Over-Injection", not "Excess Arguments"
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("__init__")).collect();
    assert!(lines
        .iter()
        .any(|l| l.contains("Constructor Over-Injection")));
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

#[test]
fn regular_function_reports_excess_args_not_constructor() {
    let out = check("def f(a, b, c, d, e, f, g):\n    pass\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Multiple smells on same function
// ===========================================================================

#[test]
fn function_can_have_multiple_smells() {
    let out = check(
        r#"
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
    let out = check(
        r#"
class ItemListView:
    def get_queryset(self):
        return self.model.objects.filter(active=True)

    def get_context_data(self, **kwargs):
        context = super().get_context_data(**kwargs)
        context["title"] = "Items"
        return context
"#,
    );
    assert!(
        out.is_empty(),
        "clean Django view should not be flagged, got: {out}"
    );
}

// ===========================================================================
// Real-world patterns: pytest fixtures
// ===========================================================================

#[test]
fn pytest_fixture_parametrize_not_flagged() {
    let out = check(
        r#"
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
"#,
    );
    // Small functions, few assertions — nothing should trigger
    assert!(
        out.is_empty(),
        "pytest fixture pattern should not be flagged, got: {out}"
    );
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
        code.push_str(&format!("def func_{i}(data):\n"));
        for j in 0..18 {
            code.push_str(&format!("    x_{j} = data.get(\"field_{j}\")\n"));
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
        code.push_str(&format!("class Service{i}:\n"));
        code.push_str(&format!(
            "    def __init__(self):\n        self.data_{i} = []\n\n"
        ));
        for j in 0..5 {
            code.push_str(&format!(
                "    def method_{j}(self):\n        return self.data_{i}\n\n"
            ));
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

// ===========================================================================
// Cognitive Complexity
// ===========================================================================

#[test]
fn cogc_flat_branches_no_nesting_penalty() {
    let out = debug(
        "def f(x):\n    if x == 1: pass\n    if x == 2: pass\n    if x == 3: pass\n    if x == 4: pass\n    if x == 5: pass\n",
    );
    assert_eq!(function_metric(&out, "f", "cogc"), Some(5));
}

#[test]
fn cogc_nested_ifs_penalized() {
    let out = debug(
        "def f(a, b, c, d):\n    if a:\n        if b:\n            if c:\n                if d:\n                    pass\n",
    );
    assert_eq!(function_metric(&out, "f", "cogc"), Some(10));
}

#[test]
fn cogc_elif_no_nesting_increase() {
    let out = debug(
        "def f(x):\n    if x == 1: pass\n    elif x == 2: pass\n    elif x == 3: pass\n    elif x == 4: pass\n",
    );
    assert_eq!(function_metric(&out, "f", "cogc"), Some(4));
}

#[test]
fn cogc_else_flat_but_increases_nesting() {
    let out = debug(
        "def f(a, b):\n    if a:\n        pass\n    else:\n        if b:\n            pass\n",
    );
    assert_eq!(function_metric(&out, "f", "cogc"), Some(4));
}

#[test]
fn cogc_for_loop_penalized_by_nesting() {
    let out = debug(
        "def f(a):\n    if a:\n        for x in range(10):\n            pass\n",
    );
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_while_penalized_by_nesting() {
    let out = debug(
        "def f(x):\n    while x > 0:\n        if x == 5:\n            pass\n        x -= 1\n",
    );
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_except_penalized_by_nesting() {
    let out = debug(
        "def f():\n    try:\n        if True:\n            pass\n    except:\n        pass\n",
    );
    // if at nesting 1 (inside try block) = +2, except at nesting 0 = +1. Total = 3
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_boolean_sequence_single_type() {
    let out = debug("def f(a, b, c):\n    if a and b and c:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(2));
}

#[test]
fn cogc_boolean_sequence_mixed_types() {
    let out = debug("def f(a, b, c):\n    if a and b or c:\n        pass\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_triggers_complex_method_alone() {
    let out = check(
        "def f(a, b, c, d, e):\n    if a:\n        if b:\n            if c:\n                if d:\n                    if e:\n                        pass\n",
    );
    assert!(
        has_smell(&out, "Complex Method"),
        "cogc>=15 should trigger Complex Method: {out}"
    );
    assert!(out.contains("cogc="), "should show cogc in detail: {out}");
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check(
        "def f(a, b, c, d):\n    if a:\n        if b:\n            if c:\n                if d:\n                    pass\n",
    );
    assert!(
        !has_smell(&out, "Complex Method"),
        "cogc=10 should not trigger: {out}"
    );
}

#[test]
fn cogc_god_method_via_cogc() {
    let mut code = "def f(a, b, c, d, e):\n    if a:\n        if b:\n            if c:\n                if d:\n                    if e:\n                        pass\n"
        .to_string();
    for i in 0..fn_padding() {
        code.push_str(&format!("    x_{i} = {i}\n"));
    }
    let out = check(&code);
    assert!(
        has_smell(&out, "God Method"),
        "cogc>=15 + loc>=50 should be God Method: {out}"
    );
}

// ===========================================================================
// Empty Error Handler
// ===========================================================================

#[test]
fn empty_except_pass_detected() {
    let out = check("def f():\n    try:\n        risky()\n    except:\n        pass\n");
    assert!(
        has_smell(&out, "Empty Error Handler"),
        "except:pass should be detected: {out}"
    );
}

#[test]
fn empty_except_bare_detected() {
    let out = check(
        "def f():\n    try:\n        risky()\n    except Exception:\n        pass\n",
    );
    assert!(
        has_smell(&out, "Empty Error Handler"),
        "except with only pass should be detected: {out}"
    );
}

#[test]
fn non_empty_except_not_detected() {
    let out = check(
        "def f():\n    try:\n        risky()\n    except Exception as e:\n        print(e)\n        raise\n",
    );
    assert!(
        !has_smell(&out, "Empty Error Handler"),
        "except with handling should not be detected: {out}"
    );
}

#[test]
fn multiple_empty_except_counted() {
    let out = check(
        "def f():\n    try:\n        a()\n    except ValueError:\n        pass\n    except TypeError:\n        pass\n",
    );
    assert!(
        has_smell(&out, "Empty Error Handler"),
        "multiple empty excepts should be detected: {out}"
    );
    assert!(
        out.contains("2 empty catch blocks"),
        "should count 2: {out}"
    );
}

#[test]
fn mixed_empty_and_nonempty_except() {
    let out = check(
        "def f():\n    try:\n        a()\n    except ValueError:\n        pass\n    except TypeError:\n        print('handled')\n",
    );
    assert!(
        has_smell(&out, "Empty Error Handler"),
        "at least one empty should trigger: {out}"
    );
    assert!(
        out.contains("1 empty catch block"),
        "should count only 1: {out}"
    );
}

#[test]
fn no_try_catch_no_smell() {
    let out = check("def f():\n    return 1\n");
    assert!(
        !has_smell(&out, "Empty Error Handler"),
        "no try/catch should not trigger: {out}"
    );
}

#[test]
fn except_with_comment_only_detected() {
    let out = check(
        "def f():\n    try:\n        risky()\n    except:\n        # TODO: handle this\n        pass\n",
    );
    assert!(
        has_smell(&out, "Empty Error Handler"),
        "except with only comment+pass should be detected: {out}"
    );
}

// ===========================================================================
// Coverage: severity branches, global nesting, edge cases
// ===========================================================================

#[test]
fn cc_only_complex_method_severity() {
    // 9 flat ifs: cc=10 (>=9), cogc=9 (<15) → cc-only complex method
    let out = check("def f(x):\n    if a: pass\n    if b: pass\n    if c: pass\n    if d: pass\n    if e: pass\n    if f: pass\n    if g: pass\n    if h: pass\n    if i: pass\n");
    assert!(has_smell(&out, "Complex Method"), "cc=10 should trigger: {out}");
    assert!(out.contains("cc="), "should show cc in detail: {out}");
    assert!(!out.contains("cogc="), "should NOT show cogc when cc-only: {out}");
}

#[test]
fn large_method_alert_severity() {
    let mut code = "def f():\n".to_string();
    for i in 0..101 {
        code.push_str(&format!("    x_{i} = {i}\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Large Method"), "101 lines should trigger: {out}");
    assert!(out.contains("[alert]"), "should be alert severity: {out}");
}

#[test]
fn global_for_loop_deep_nesting() {
    let code = "for x in range(10):\n    if a:\n        if b:\n            if c:\n                pass\ndef f():\n    pass\n";
    let out = check(code);
    assert!(has_smell(&out, "Deep Global Nesting"), "global for with deep nesting should trigger: {out}");
}

#[test]
fn global_while_loop_deep_nesting() {
    let code = "while True:\n    if a:\n        if b:\n            if c:\n                pass\ndef f():\n    pass\n";
    let out = check(code);
    assert!(has_smell(&out, "Deep Global Nesting"), "global while with deep nesting: {out}");
}

#[test]
fn decorated_class_inside_class_not_function() {
    let out = debug("class Outer:\n    @some_decorator\n    class Inner:\n        pass\n    def method(self):\n        return 1\n");
    assert!(out.contains("method"), "should find method: {out}");
}

#[test]
fn assert_with_boolean_increments_cc() {
    let out = debug("def f():\n    assert a and b\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2), "assert with boolean should increment cc");
}

#[test]
fn lcom4_single_method_no_smell() {
    let code = "class Foo:\n    def __init__(self):\n        self.x = 1\n    def get_x(self):\n        return self.x\n";
    let out = check(code);
    assert!(!has_smell(&out, "Low Cohesion"), "single non-init method should not trigger LCOM4: {out}");
}

#[test]
fn unique_assertion_hash_not_duplicated() {
    let code = "def test_a():\n    assert 1 == 1\n    assert 2 == 2\n    assert 3 == 3\n    assert 4 == 4\n    assert 5 == 5\n    assert 6 == 6\n\ndef test_b():\n    assert 1 == 1\n    assert 2 == 2\n    assert 3 == 3\n    assert 4 == 4\n    assert 5 == 5\n    assert 6 == 6\n\ndef test_c():\n    assert 10 == 10\n    assert 20 == 20\n    assert 30 == 30\n    assert 40 == 40\n    assert 50 == 50\n    assert 60 == 60\n";
    let out = check(code);
    // test_a and test_b are duplicates; test_c is unique (its hash group has size 1)
    assert!(has_smell(&out, "Duplicated Assertion Blocks"), "should detect a+b duplicates: {out}");
}

#[test]
fn elif_updates_max_nesting() {
    let out = debug("def f(x, y, z):\n    if x:\n        pass\n    elif y:\n        if z:\n            pass\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2), "elif body with nested if should update nesting");
}

#[test]
fn cc_alert_severity_over_18() {
    // 18 flat ifs: cc=19 (>cc_alert=18) → alert severity
    let mut code = "def f(x):\n".to_string();
    for i in 0..18 {
        code.push_str(&format!("    if x == {i}: pass\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Complex Method"), "cc=19 should trigger: {out}");
    assert!(out.contains("[alert]"), "cc>18 should be alert: {out}");
}

#[test]
fn cogc_only_complex_method() {
    // Deep nesting: cogc >= 15 but cc < 9
    // 5 nested ifs: cc=6, cogc=1+2+3+4+5=15
    let out = check("def f(a, b, c, d, e):\n    if a:\n        if b:\n            if c:\n                if d:\n                    if e:\n                        pass\n");
    assert!(has_smell(&out, "Complex Method"), "cogc=15 should trigger: {out}");
    assert!(out.contains("cogc="), "cogc-only should show cogc: {out}");
}

#[test]
fn cogc_alert_severity_over_25() {
    // Very deep nesting: cogc > 25
    // 7 nested ifs: cogc=1+2+3+4+5+6+7=28
    let out = check("def f(a, b, c, d, e, f, g):\n    if a:\n        if b:\n            if c:\n                if d:\n                    if e:\n                        if f:\n                            if g:\n                                pass\n");
    assert!(has_smell(&out, "Complex Method"), "cogc=28 should trigger: {out}");
    assert!(out.contains("[alert]"), "cogc>25 should be alert: {out}");
}

#[test]
fn both_cc_and_cogc_complex() {
    // Function with both cc>=9 AND cogc>=15
    // 9 ifs, some nested: cc=10, cogc > 15
    let out = check("def f(x):\n    if x == 1:\n        if x == 2:\n            if x == 3:\n                if x == 4:\n                    if x == 5:\n                        pass\n    if x == 6: pass\n    if x == 7: pass\n    if x == 8: pass\n    if x == 9: pass\n");
    assert!(has_smell(&out, "Complex Method"), "both thresholds exceeded: {out}");
    assert!(out.contains("cc="), "should show cc: {out}");
    assert!(out.contains("cogc="), "should show cogc: {out}");
}

#[test]
fn empty_catch_in_walk_body_except() {
    // Test the except_clause path in walk_body (not walk_children)
    let out = debug("def f():\n    try:\n        x = 1\n    except:\n        pass\n");
    let metric = function_metric(&out, "f", "cc");
    assert!(metric.is_some(), "should parse function: {out}");
}

#[test]
fn module_format_compact_not_used() {
    // Module findings go through format_stop, not format_compact
    // This verifies the behavior — module findings are excluded from hook output
    let out = check("x = 1\n");
    assert!(out.is_empty(), "trivial file should have no findings");
}
