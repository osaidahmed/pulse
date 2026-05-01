mod common;

use common::*;

lang_helpers!("kt");

// ── CC precision ──────────────────────────────────────────────────────

#[test]
fn cc_counts_if() {
    let out = debug("class T {\n    fun f() {\n        if (true) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("class T {\n    fun f(x: Int) {\n        if (x == 1) {} else if (x == 2) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("class T {\n    fun f(items: List<Int>) {\n        for (i in items) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("class T {\n    fun f() {\n        while (true) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_do_while() {
    let out = debug("class T {\n    fun f() {\n        do {} while (true)\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_catch() {
    let out = debug("class T {\n    fun f() {\n        try {} catch (e: Exception) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and() {
    let out = debug("class T {\n    fun f(a: Boolean, b: Boolean) {\n        if (a && b) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or() {
    let out = debug("class T {\n    fun f(a: Boolean, b: Boolean) {\n        if (a || b) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_elvis() {
    let out = debug("class T {\n    fun f(x: Int?): Int {\n        return x ?: 0\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("class T {\n    fun f(a: Boolean, b: Boolean, c: Boolean, d: Boolean) {\n        if (a && b && c && d) {}\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 5, "expected cc >= 5, got: {cc}");
}

#[test]
fn cc_nested_if_in_for() {
    let out = debug("class T {\n    fun f(items: List<Int>) {\n        for (i in items) { if (i > 0) {} }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_when_many_entries() {
    let out = debug("class T {\n    fun f(x: String): String {\n        return when (x) {\n            \"a\" -> \"1\"\n            \"b\" -> \"2\"\n            \"c\" -> \"3\"\n            \"d\" -> \"4\"\n            \"e\" -> \"5\"\n            \"f\" -> \"6\"\n            \"g\" -> \"7\"\n            \"h\" -> \"8\"\n            else -> \"0\"\n        }\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 9, "expected cc >= 9, got: {cc}");
}

#[test]
fn cc_multiple_catch() {
    let out = debug("class T {\n    fun f() {\n        try {} catch (e: IllegalArgumentException) {} catch (e: IllegalStateException) {} catch (e: Exception) {}\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "expected cc >= 4, got: {cc}");
}

#[test]
fn cc_else_if_chain() {
    let out = debug("class T {\n    fun f(x: Int): Int {\n        if (x == 1) { return 1 } else if (x == 2) { return 2 } else if (x == 3) { return 3 }\n        return 0\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_when_without_subject() {
    let out = debug("class T {\n    fun f(x: Int): String {\n        return when {\n            x > 0 -> \"pos\"\n            x < 0 -> \"neg\"\n            else -> \"zero\"\n        }\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "expected cc >= 3, got: {cc}");
}

#[test]
fn cc_do_while_standalone() {
    let out = debug("class T {\n    fun f(): Int {\n        var x = 0\n        do { x += 1 } while (x < 10)\n        return x\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ── Nesting ───────────────────────────────────────────────────────────

#[test]
fn nesting_0_flat() {
    let out = debug("class T {\n    fun f(): Int {\n        return 0\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_1_single_if() {
    let out = debug("class T {\n    fun f(x: Boolean) {\n        if (x) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_2_nested_if() {
    let out = debug("class T {\n    fun f(a: Boolean, b: Boolean) {\n        if (a) { if (b) {} }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_3_for_if_for() {
    let out = debug("class T {\n    fun f(items: List<List<Int>>) {\n        for (g in items) { if (g.isNotEmpty()) { for (i in g) {} } }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_deep_exceeds_4() {
    let out = debug("class T {\n    fun f(items: List<List<List<Int>>>) {\n        for (a in items) { if (a.isNotEmpty()) { for (b in a) { if (b.isNotEmpty()) { for (c in b) {} } } } }\n    }\n}\n");
    let n = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(n >= 5, "expected nesting >= 5, got: {n}");
}

#[test]
fn nesting_when_counts_depth() {
    let out = debug("class T {\n    fun f(x: Int) {\n        when (x) {\n            1 -> { if (true) {} }\n            else -> {}\n        }\n    }\n}\n");
    let n = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(n >= 2, "expected nesting >= 2, got: {n}");
}

#[test]
fn nesting_try_if() {
    let out = debug("class T {\n    fun f(x: Boolean) {\n        try { if (x) {} } catch (e: Exception) {}\n    }\n}\n");
    let n = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(n >= 1, "expected nesting >= 1, got: {n}");
}

// ── Arguments ─────────────────────────────────────────────────────────

#[test]
fn args_positional() {
    let out = debug("class T {\n    fun f(a: Int, b: String, c: Boolean) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_zero() {
    let out = debug("class T {\n    fun f() {}\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_typed_params() {
    let out = debug("class T {\n    fun f(a: Int, b: String): Int {\n        return 0\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ── Primitive obsession ──────────────────────────────────────────────

#[test]
fn primitive_obsession_all_primitives() {
    let out = check("fun f(a: Int, b: Long, c: Double, d: Boolean, e: Char): Int {\n    return 0\n}\n");
    assert!(has_smell(&out, "Primitive Obsession"), "got: {out}");
}

#[test]
fn primitive_obsession_mixed_below_threshold() {
    let out = check("fun f(a: Int, b: String, c: List<Int>, d: Map<String, Int>): Int {\n    return 0\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"), "got: {out}");
}

#[test]
fn primitive_obsession_below_min_typed() {
    let out = check("fun f(a: Int, b: Int, c: Int): Int {\n    return 0\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"), "got: {out}");
}

#[test]
fn primitive_obsession_recognizes_long_float() {
    let out = check("fun f(a: Long, b: Float, c: Double, d: Short, e: Byte): Int {\n    return 0\n}\n");
    assert!(has_smell(&out, "Primitive Obsession"), "got: {out}");
}

#[test]
fn primitive_obsession_complex_types_not_flagged() {
    let out = check("fun f(a: List<String>, b: Map<Int, String>, c: Set<Int>, d: Pair<Int, Int>, e: Triple<Int, Int, Int>): Int {\n    return 0\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"), "got: {out}");
}

// ── LCOM4 ────────────────────────────────────────────────────────────

#[test]
fn lcom4_three_groups_flagged() {
    let out = check("class C {\n    private var a: Int = 0\n    private var b: Int = 0\n    private var c: Int = 0\n    fun fa(): Int { return a }\n    fun fb(): Int { return b }\n    fun fc(): Int { return c }\n}\n");
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_cohesive_not_flagged() {
    let out = check("class C {\n    private var a: Int = 0\n    fun fa(): Int { return this.a }\n    fun fb(): Int { return this.a + 1 }\n    fun fc(): Int { return this.a + 2 }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_single_method_not_flagged() {
    let out = check("class C {\n    private var a: Int = 0\n    fun fa(): Int { return a }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_transitive_connection() {
    let out = check("class C {\n    private var a: Int = 0\n    private var b: Int = 0\n    fun fa(): Int { return this.a + this.b }\n    fun fb(): Int { return this.a }\n    fun fc(): Int { return this.b }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "class Coord {\n",
        "    var state: Int = 0\n",
        "    fun process(e: Int): Boolean = this.validate(e) && this.dispatch(e)\n",
        "    fun validate(e: Int): Boolean = e > 0\n",
        "    fun dispatch(e: Int): Boolean = this.send(e)\n",
        "    fun send(e: Int): Boolean = true\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "class Mixed {\n",
        "    var x: Int = 0\n",
        "    fun a(): Int = this.x\n",
        "    fun b(): Int { this.x = 1; return this.c() }\n",
        "    fun c(): Int = 42\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_class_still_fires() {
    let out = check(concat!(
        "class UserService {\n",
        "    var db: Int = 0; var cache: Int = 0; var mailer: Int = 0\n",
        "    var events: Int = 0; var audit: Int = 0\n",
        "    fun getUser(): Int = this.db\n",
        "    fun cacheUser(): Int = this.cache\n",
        "    fun sendWelcome(): Int = this.mailer\n",
        "    fun publish(): Int = this.events\n",
        "    fun auditLog(): Int = this.audit\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "class Service {\n",
        "    var db: Int = 0; var cache: Int = 0; var log: Int = 0\n",
        "    fun a(): Int = this.db\n",
        "    fun b(): Int = this.cache\n",
        "    fun c(): Int = this.log\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ── Duplication ───────────────────────────────────────────────────────

#[test]
fn duplication_detected() {
    let out = check("fun a(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\nfun b(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\n");
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn duplication_test_suppressed() {
    let out = check("fun test_a(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\nfun test_b(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\n");
    assert!(!has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn duplication_mixed_test_and_prod_flagged() {
    let out = check("fun test_a(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\nfun prod(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\n");
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn duplication_two_is_minimum() {
    let out = check("fun a(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\nfun b(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\n");
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

// ── Constructor / Injection ──────────────────────────────────────────

#[test]
fn constructor_reports_over_injection() {
    let out = check("class S(\n    val a: String,\n    val b: String,\n    val c: String,\n    val d: String,\n    val e: String,\n    val f: String\n) {\n    fun get(): String { return a }\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

#[test]
fn regular_method_reports_excess_not_injection() {
    let out = check("fun f(a: String, b: String, c: String, d: String, e: String, f: String): String {\n    return a\n}\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
    assert!(!has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

#[test]
fn constructor_reports_injection_not_excess() {
    let out = check("class S(\n    val a: String,\n    val b: String,\n    val c: String,\n    val d: String,\n    val e: String,\n    val f: String\n) {\n    fun get(): String { return a }\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
    assert!(!has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ── Assertion blocks ─────────────────────────────────────────────────

#[test]
fn assertion_block_at_threshold() {
    let mut code = String::from("fun f() {\n");
    for i in 0..asserts_at() {
        code.push_str(&format!("    assert({i} > 0)\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"), "got: {out}");
}

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("fun f() {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("    assert({i} > 0)\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"), "got: {out}");
}

#[test]
fn assertion_block_interrupted_resets() {
    let mut code = String::from("fun f() {\n");
    for i in 0..8 {
        code.push_str(&format!("    assert({i} > 0)\n"));
    }
    code.push_str("    val x = 1\n");
    for i in 0..8 {
        code.push_str(&format!("    assert({i} > 0)\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"), "got: {out}");
}

// ── Overall function size ────────────────────────────────────────────

#[test]
fn overall_function_size_below_threshold() {
    let mut code = String::new();
    for i in 0..(t().large_fn_count as usize - 1) {
        code.push_str(&format!("fun f{i}(): Int {{\n"));
        for j in 0..45 {
            code.push_str(&format!("    val x{j} = {j}\n"));
        }
        code.push_str("    return 0\n}\n\n");
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"), "got: {out}");
}

#[test]
fn overall_function_size_at_threshold() {
    let mut code = String::new();
    for i in 0..t().large_fn_count as usize {
        code.push_str(&format!("fun f{i}(): Int {{\n"));
        for j in 0..45 {
            code.push_str(&format!("    val x{j} = {j}\n"));
        }
        code.push_str("    return 0\n}\n\n");
    }
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"), "got: {out}");
}

// ── Declarations ─────────────────────────────────────────────────────

#[test]
fn declarations_below_threshold() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class C{i}\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Excessive Declarations"), "got: {out}");
}

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("class C{i}\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Excessive Declarations"), "got: {out}");
}

// ── Embedded blocks ──────────────────────────────────────────────────

#[test]
fn small_string_not_flagged() {
    let out = check("class T {\n    fun f(): String {\n        return \"hello\"\n    }\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"), "got: {out}");
}

#[test]
fn multiline_string_flagged() {
    let mut code = String::from("class T {\n    fun f(): String {\n        return \"\"\"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("            line {i}\n"));
    }
    code.push_str("        \"\"\"\n    }\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

// ── Cognitive complexity ─────────────────────────────────────────────

#[test]
fn cogc_flat_branches() {
    let out = debug("class T {\n    fun f(x: Int) {\n        if (x > 1) {}\n        if (x > 2) {}\n        if (x > 3) {}\n        if (x > 4) {}\n        if (x > 5) {}\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 5, "got: {cogc}");
}

#[test]
fn cogc_nested_ifs() {
    let out = debug("class T {\n    fun f(a: Boolean, b: Boolean, c: Boolean, d: Boolean) {\n        if (a) {\n            if (b) {\n                if (c) {\n                    if (d) {}\n                }\n            }\n        }\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 10, "got: {cogc}");
}

// ── Deep nesting + when ──────────────────────────────────────────────

#[test]
fn deep_nesting_with_when() {
    let out = debug("class T {\n    fun f(items: List<Int>, x: String) {\n        for (i in items) {\n            if (i > 0) {\n                when (x) {\n                    \"a\" -> {\n                        for (j in items) {\n                            if (j > 0) {}\n                        }\n                    }\n                    else -> {}\n                }\n            }\n        }\n    }\n}\n");
    let n = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(n >= 5, "expected deep nesting, got: {n}");
}

// ── Nested conditional chunks ────────────────────────────────────────

#[test]
fn nested_conditional_chunks_detected() {
    let out = check("class T {\n    fun f(x: Int): Int {\n        for (i in 0..10) { if (x > 0) { if (x > 1) {} } }\n        for (i in 0..10) { if (x > 2) { if (x > 3) {} } }\n        for (i in 0..10) { if (x > 4) { if (x > 5) {} } }\n        return x\n    }\n}\n");
    assert!(has_smell(&out, "Nested Conditional"), "got: {out}");
}

// ── Module-level ─────────────────────────────────────────────────────

#[test]
fn god_class_requires_god_method() {
    let mut code = String::from("class Big {\n");
    for i in 0..declarations_above() {
        code.push_str(&format!("    fun f{i}(): Int {{\n"));
        for j in 0..20 {
            code.push_str(&format!("        val x{j} = {j}\n"));
        }
        code.push_str("        return 0\n    }\n\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"), "got: {out}");
}

#[test]
fn god_class_triggers_with_god_method() {
    let mut code = String::from("class Big {\n");
    code.push_str("    fun god(x: Int): Int {\n");
    for _ in 0..cc_branches() {
        code.push_str("        if (x > 0) {}\n");
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("        val v{i} = {i}\n"));
    }
    code.push_str("        return 0\n    }\n\n");
    for i in 1..functions_above() {
        code.push_str(&format!("    fun f{i}(): Int {{\n"));
        for j in 0..20 {
            code.push_str(&format!("        val x{j} = {j}\n"));
        }
        code.push_str("        return 0\n    }\n\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "God Class"), "got: {out}");
}

#[test]
fn shallow_global_not_flagged() {
    let out = check("class T {\n    fun f(): Int { return 0 }\n}\n");
    assert!(!has_smell(&out, "Global Conditionals"), "got: {out}");
}

#[test]
fn output_has_module_prefix() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("fun f{i}(): Int {{ return {i} }}\n"));
    }
    let out = check(&code);
    assert!(out.contains("Module:"), "got: {out}");
}

// ── Multiple smells ──────────────────────────────────────────────────

#[test]
fn multiple_smells_same_function() {
    let mut code = String::from("class T {\n    fun f(a: String, b: String, c: String, d: String, e: String, f: String): String {\n        val big = \"\"\"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("            line {i}\n"));
    }
    code.push_str("        \"\"\"\n");
    code.push_str("        for (i in 0..10) {\n            if (true) {\n                for (j in 0..10) {\n                    if (true) {\n                        for (k in 0..10) {}\n                    }\n                }\n            }\n        }\n");
    code.push_str("        return a\n    }\n}\n");
    let out = check(&code);
    let func_lines: Vec<_> = out.lines().filter(|l| l.contains("T.f")).collect();
    assert!(func_lines.len() >= 2, "expected multiple smells, got: {out}");
}

#[test]
fn function_can_have_excess_and_embedded() {
    let mut code = String::from("class T {\n    fun f(a: String, b: String, c: String, d: String, e: String, f: String): String {\n        val big = \"\"\"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("            line {i}\n"));
    }
    code.push_str("        \"\"\"\n        return a\n    }\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

// ── Output format ────────────────────────────────────────────────────

#[test]
fn output_starts_with_pulse() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("fun f{i}(): Int {{ return {i} }}\n"));
    }
    let out = check(&code);
    assert!(out.starts_with("pulse:"), "got: {out}");
}

#[test]
fn output_has_line_numbers() {
    let out = check("class T {\n    fun f(x: Int): Int {\n        if (x==1) {} else if (x==2) {} else if (x==3) {} else if (x==4) {} else if (x==5) {} else if (x==6) {} else if (x==7) {} else if (x==8) {}\n        return x\n    }\n}\n");
    assert!(out.contains("(L"), "got: {out}");
}

#[test]
fn issue_count_matches() {
    let out = check("class T {\n    fun f(x: Int): Int {\n        if (x==1) {} else if (x==2) {} else if (x==3) {} else if (x==4) {} else if (x==5) {} else if (x==6) {} else if (x==7) {} else if (x==8) {}\n        return x\n    }\n}\n");
    let first_line = out.lines().next().unwrap_or("");
    let header_count: usize = first_line
        .split("issue").next().unwrap_or("0")
        .trim().rsplit(' ').next().unwrap_or("0")
        .parse().unwrap_or(0);
    let finding_count = out.lines().filter(|l| l.starts_with("  ")).count();
    assert_eq!(header_count, finding_count);
}

// ── Hook edge cases ──────────────────────────────────────────────────

#[test]
fn hook_missing_file_path() {
    let binary = env!("CARGO_BIN_EXE_pulse");
    let out = std::process::Command::new(binary)
        .arg("--hook")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(b"{\"tool_input\":{}}");
            }
            child.wait_with_output()
        })
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).is_empty());
}

#[test]
fn hook_empty_stdin() {
    let binary = env!("CARGO_BIN_EXE_pulse");
    let out = std::process::Command::new(binary)
        .arg("--hook")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(b"");
            }
            child.wait_with_output()
        })
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).is_empty());
}

// ── Clean/edge cases ─────────────────────────────────────────────────

#[test]
fn clean_kotlin_module_not_flagged() {
    let out = check("class T {\n    fun f(): Int { return 0 }\n    fun g(): Int { return 1 }\n}\n");
    assert!(out.is_empty(), "got: {out}");
}

#[test]
fn comments_only() {
    let out = check("// just a comment\n/* block */\n");
    assert!(out.is_empty(), "got: {out}");
}

#[test]
fn empty_file() {
    let out = check("");
    assert!(out.is_empty(), "got: {out}");
}

// ── Performance ──────────────────────────────────────────────────────

#[test]
fn performance_1000_loc() {
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("fun f{i}(): Int {{\n"));
        for j in 0..18 {
            code.push_str(&format!("    val x{j} = {j}\n"));
        }
        code.push_str("    return 0\n}\n\n");
    }
    let start = std::time::Instant::now();
    let _ = check(&code);
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took {}ms", elapsed.as_millis());
}

#[test]
fn performance_class_hierarchy() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class C{i} {{\n"));
        for j in 0..5 {
            code.push_str(&format!("    fun m{j}(): Int {{\n        val x = {j}\n        return x\n    }}\n"));
        }
        code.push_str("}\n\n");
    }
    let start = std::time::Instant::now();
    let _ = check(&code);
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took {}ms", elapsed.as_millis());
}

// ── Kotlin-specific ──────────────────────────────────────────────────

#[test]
fn when_expression_increments_cc() {
    let out = debug("class T {\n    fun f(x: String): String {\n        return when (x) {\n            \"a\" -> \"1\"\n            \"b\" -> \"2\"\n            \"c\" -> \"3\"\n            else -> \"0\"\n        }\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 4, "expected cc=4 (1+3 branches), got: {cc}");
}

#[test]
fn when_else_no_cc() {
    let out = debug("class T {\n    fun f(x: String): String {\n        return when (x) {\n            \"a\" -> \"1\"\n            else -> \"0\"\n        }\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2, "expected cc=2 (1+1 branch, else is free), got: {cc}");
}

#[test]
fn when_nested_in_for_counts_nesting() {
    let out = debug("class T {\n    fun f(items: List<Int>) {\n        for (i in items) {\n            when (i) {\n                1 -> {}\n                2 -> {}\n                else -> {}\n            }\n        }\n    }\n}\n");
    let n = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(n >= 2, "expected nesting >= 2, got: {n}");
}

#[test]
fn elvis_increments_cc() {
    let out = debug("class T {\n    fun f(x: Int?): Int {\n        return x ?: 0\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn expression_body_analyzed() {
    let out = debug("fun f(a: Int, b: Int): Int = a + b\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "expression body should have cc=1, got: {cc}");
}

#[test]
fn expression_body_with_if() {
    let out = debug("fun f(x: Int): Int = if (x > 0) x else -x\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2, "expected cc=2 (1+if), got: {cc}");
}

#[test]
fn data_class_methods_analyzed() {
    let out = debug("data class P(val x: Int, val y: Int) {\n    fun sum(): Int { return x + y }\n}\n");
    assert!(out.contains("P.sum"), "expected data class method, got: {out}");
}

#[test]
fn object_declaration_methods() {
    let out = debug("object Singleton {\n    fun doStuff(): Int { return 1 }\n}\n");
    assert!(out.contains("Singleton.doStuff"), "got: {out}");
}

#[test]
fn companion_object_methods_prefixed() {
    let out = debug("class C {\n    companion object {\n        fun create(): C { return C() }\n    }\n}\n");
    assert!(out.contains("C.Companion.create"), "got: {out}");
}

#[test]
fn init_block_analyzed() {
    let out = debug("class C {\n    init {\n        println(\"init\")\n    }\n    fun f(): Int { return 0 }\n}\n");
    assert!(out.contains("C.init"), "got: {out}");
}

#[test]
fn extension_function_analyzed() {
    let out = debug("fun String.isEmail(): Boolean {\n    return this.contains(\"@\")\n}\n");
    assert!(out.contains("String.isEmail"), "got: {out}");
}

#[test]
fn lambda_not_recursed() {
    let out = debug("class T {\n    fun f(items: List<Int>): List<Int> {\n        return items.filter { it > 0 }.map { it * 2 }\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "lambda should not add CC, got: {cc}");
}

#[test]
fn nested_function_not_recursed() {
    let out = debug("class T {\n    fun f(): Int {\n        fun inner(): Int { if (true) {} ; return 0 }\n        return inner()\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "nested function should not add CC, got: {cc}");
}

#[test]
fn nullable_type_param_counted() {
    let out = debug("class T {\n    fun f(a: Int?, b: String?, c: Boolean?): Int {\n        return 0\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}
