use pulse::parse::{self, Language};
use pulse::smells::{self, Finding, Smell};
use pulse::walk::{FileMetrics, FunctionMetrics};

use crate::common::t;

fn metrics(code: &str, lang: Language) -> FileMetrics {
    parse::parse_and_walk_guarded(code, lang).expect("fixture must parse")
}

fn findings(code: &str, lang: Language) -> Vec<Finding> {
    smells::detect(&metrics(code, lang), code, &t())
}

fn has(findings: &[Finding], smell: Smell) -> bool {
    findings.iter().any(|f| f.smell == smell)
}

fn func<'a>(m: &'a FileMetrics, name: &str) -> &'a FunctionMetrics {
    m.functions.iter().find(|f| f.name == name).expect("function present")
}

fn assert_fires(code: &str, lang: Language, smell: Smell, expected: bool, label: &str) {
    let f = findings(code, lang);
    assert_eq!(has(&f, smell), expected, "{label}: expected fires={expected}, findings: {f:#?}");
}

fn seq_if_fn(name: &str, ifs: u32) -> String {
    let mut s = format!("def {name}(flag):\n");
    for _ in 0..ifs {
        s.push_str("    if flag:\n        x = 0\n");
    }
    if ifs == 0 {
        s.push_str("    x = 0\n");
    }
    s
}

fn nested_chain_fn(depth: usize, elses: usize) -> String {
    let mut s = String::from("def f(flag):\n");
    for d in 0..depth {
        s.push_str(&"    ".repeat(d + 1));
        s.push_str("if flag:\n");
    }
    s.push_str(&"    ".repeat(depth + 1));
    s.push_str("x = 0\n");
    for d in (0..depth).rev() {
        if depth - d <= elses {
            s.push_str(&"    ".repeat(d + 1));
            s.push_str("else:\n");
            s.push_str(&"    ".repeat(d + 2));
            s.push_str("y = 0\n");
        }
    }
    s
}

fn fn_with_cc_and_loc(name: &str, cc: u32, loc: u32) -> String {
    let mut s = seq_if_fn(name, cc - 1);
    let used = 1 + 2 * (cc - 1) + u32::from(cc == 1);
    for i in used..loc {
        s.push_str(&format!("    pad{i} = 0\n"));
    }
    s
}

// ── cc: ComplexMethod fires AT warning (>=) ──

#[test]
fn cc_warning_boundary() {
    let w = t().function.cc_warning;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let code = seq_if_fn("f", target - 1);
        assert_eq!(func(&metrics(&code, Language::Python), "f").cc, target, "cc fixture calibration");
        assert_fires(&code, Language::Python, Smell::ComplexMethod, expected, "cc boundary");
    }
}

// ── cogc: ComplexMethod fires AT warning (>=) while cc stays below its warning ──

#[test]
fn cogc_warning_boundary() {
    let w = t().function.cogc_warning;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let code = cogc_fixture(target);
        let m = metrics(&code, Language::Python);
        let f = func(&m, "f");
        assert_eq!(f.cognitive_complexity, target, "cogc fixture calibration");
        assert!(f.cc < t().function.cc_warning, "cogc fixture must keep cc below warning, got {}", f.cc);
        assert_fires(&code, Language::Python, Smell::ComplexMethod, expected, "cogc boundary");
    }
}

fn cogc_fixture(target: u32) -> String {
    let mut depth = 0u32;
    while (depth + 1) * (depth + 2) / 2 <= target {
        depth += 1;
    }
    let base = depth * (depth + 1) / 2;
    nested_chain_fn(depth as usize, (target - base) as usize)
}

// ── fn loc: LargeMethod fires AT warning (>=) ──

#[test]
fn fn_loc_warning_boundary() {
    let w = t().function.fn_loc_warning;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let code = fn_with_cc_and_loc("f", 2, target);
        let m = metrics(&code, Language::Python);
        let f = func(&m, "f");
        assert_eq!(f.loc, target, "loc fixture calibration");
        assert!(f.cc < t().function.cc_warning, "loc fixture must keep cc low");
        assert_fires(&code, Language::Python, Smell::LargeMethod, expected, "fn_loc boundary");
    }
}

// ── god method: conjunction of (cc >= warning) and (loc >= warning) ──

#[test]
fn god_method_requires_both_dimensions_at_threshold() {
    let cc_w = t().function.cc_warning;
    let loc_w = t().function.fn_loc_warning;

    let both = fn_with_cc_and_loc("f", cc_w, loc_w);
    let f_both = findings(&both, Language::Python);
    assert!(has(&f_both, Smell::GodMethod), "both at threshold => god method: {f_both:#?}");
    assert!(!has(&f_both, Smell::ComplexMethod), "god method subsumes complex method");
    assert!(!has(&f_both, Smell::LargeMethod), "god method subsumes large method");

    let cc_only = fn_with_cc_and_loc("f", cc_w, loc_w - 1);
    let f_cc = findings(&cc_only, Language::Python);
    assert!(!has(&f_cc, Smell::GodMethod) && has(&f_cc, Smell::ComplexMethod), "cc-only: {f_cc:#?}");

    let loc_only = fn_with_cc_and_loc("f", cc_w - 1, loc_w);
    let f_loc = findings(&loc_only, Language::Python);
    assert!(!has(&f_loc, Smell::GodMethod) && has(&f_loc, Smell::LargeMethod), "loc-only: {f_loc:#?}");
}

// ── nesting: DeepNestedComplexity fires AT threshold (>=) ──

#[test]
fn nesting_depth_boundary() {
    let w = t().function.nesting_depth;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let code = nested_chain_fn(target as usize, 0);
        assert_eq!(func(&metrics(&code, Language::Python), "f").max_nesting, target, "nesting fixture calibration");
        assert_fires(&code, Language::Python, Smell::DeepNestedComplexity, expected, "nesting boundary");
    }
}

// ── bump count: NestedConditionalChunks fires AT threshold (>=) ──

#[test]
fn bump_count_boundary() {
    let w = t().function.bump_count;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let mut code = String::from("def f(flag):\n");
        for _ in 0..target {
            code.push_str("    if flag:\n        if flag:\n            if flag:\n                x = 0\n");
        }
        if target == 0 {
            code.push_str("    x = 0\n");
        }
        assert_eq!(func(&metrics(&code, Language::Python), "f").bump_count, target, "bump fixture calibration");
        assert_fires(&code, Language::Python, Smell::NestedConditionalChunks, expected, "bump boundary");
    }
}

// ── compound conditions: ComplexConditional fires only ABOVE threshold (strict >) ──

#[test]
fn compound_conditions_boundary() {
    let w = t().function.compound_conditions;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let mut code = String::from("def f(a, b, c):\n");
        for _ in 0..target {
            code.push_str("    if a and b and c:\n        x = 0\n");
        }
        if target == 0 {
            code.push_str("    x = 0\n");
        }
        let measured = func(&metrics(&code, Language::Python), "f").compound_condition_count;
        assert_eq!(measured, target, "compound fixture calibration");
        assert_fires(&code, Language::Python, Smell::ComplexConditional, expected, "compound boundary");
    }
}

// ── args: ExcessArguments fires only ABOVE threshold (strict >) ──

#[test]
fn arg_max_boundary() {
    let w = t().function.arg_max;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let args: Vec<String> = (0..target).map(|i| format!("a{i}")).collect();
        let code = format!("def f({}):\n    x = 0\n", args.join(", "));
        assert_eq!(func(&metrics(&code, Language::Python), "f").arg_count, target, "arg fixture calibration");
        assert_fires(&code, Language::Python, Smell::ExcessArguments, expected, "arg boundary");
    }
}

// ── constructor args: ConstructorOverInjection fires only ABOVE arg threshold (strict >) ──

#[test]
fn constructor_arg_max_boundary() {
    let w = t().function.constructor_arg_max;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let args: Vec<String> = (0..target).map(|i| format!("a{i}")).collect();
        let code = format!("class C:\n    def __init__(self, {}):\n        self.x = 0\n", args.join(", "));
        let m = metrics(&code, Language::Python);
        assert_eq!(func(&m, "C.__init__").arg_count, target, "ctor arg fixture calibration");
        assert_fires(&code, Language::Python, Smell::ConstructorOverInjection, expected, "ctor arg boundary");
    }
}

// ── constructor injected deps: fires AT threshold (>=) ──

#[test]
fn constructor_dep_injection_boundary() {
    let w = t().analysis.constructor_dep_injection_min;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let args: Vec<String> = (0..target).map(|i| format!("a{i}: Widget{i}")).collect();
        let code = format!("class C:\n    def __init__(self, {}):\n        self.x = 0\n", args.join(", "));
        let m = metrics(&code, Language::Python);
        let f = func(&m, "C.__init__");
        assert_eq!(f.typed_param_count - f.primitive_type_count, target, "ctor dep fixture calibration");
        assert_fires(&code, Language::Python, Smell::ConstructorOverInjection, expected, "ctor dep boundary");
    }
}

// ── embedded block: LargeEmbeddedBlock fires only ABOVE threshold (strict >) ──

#[test]
fn embedded_block_boundary() {
    let w = t().function.embedded_block_loc;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let content = "L\n".repeat((target - 2) as usize);
        let code = format!("def f():\n    s = \"\"\"\n{content}\"\"\"\n    x = 0\n");
        let measured = func(&metrics(&code, Language::Python), "f").max_embedded_block_loc;
        assert_eq!(measured, target, "embedded fixture calibration");
        assert_fires(&code, Language::Python, Smell::LargeEmbeddedBlock, expected, "embedded boundary");
    }
}

// ── consecutive asserts: LargeAssertionBlock fires only ABOVE threshold (strict >) ──

#[test]
fn consecutive_asserts_boundary() {
    let w = t().analysis.consecutive_asserts_max;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let mut code = String::from("def checker(x):\n");
        for i in 0..target {
            code.push_str(&format!("    assert x == {i}\n"));
        }
        let measured = func(&metrics(&code, Language::Python), "checker").consecutive_asserts;
        assert_eq!(measured, target, "assert fixture calibration");
        assert_fires(&code, Language::Python, Smell::LargeAssertionBlock, expected, "assert boundary");
    }
}

// ── string match arms: StringlyTypedSwitch fires only ABOVE threshold (strict >) ──

#[test]
fn string_match_arms_boundary() {
    let w = t().analysis.max_string_match_arms;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let mut code = String::from("def f(s):\n    match s:\n");
        for i in 0..target {
            code.push_str(&format!("        case \"k{i}\":\n            x = {i}\n"));
        }
        let measured = func(&metrics(&code, Language::Python), "f").string_match_arms;
        assert_eq!(measured, target, "match arm fixture calibration");
        assert_fires(&code, Language::Python, Smell::StringlyTypedSwitch, expected, "match arm boundary");
    }
}

// ── short variable names: count must EXCEED max (strict >) while loc gate is AT-inclusive (>=) ──

fn short_var_fn(short_count: u32, loc: u32) -> String {
    let names = ["a", "b", "c", "d", "e", "f", "g", "h"];
    let mut code = String::from("def fun():\n");
    for name in names.iter().take(short_count as usize) {
        code.push_str(&format!("    {name} = 0\n"));
    }
    let used = 1 + short_count;
    for i in used..loc {
        code.push_str(&format!("    pad{i} = 0\n"));
    }
    code
}

#[test]
fn short_var_count_boundary() {
    let max = t().analysis.short_var_max_count;
    let loc = t().analysis.short_var_min_fn_loc + 5;
    for (target, expected) in [(max - 1, false), (max, false), (max + 1, true)] {
        let code = short_var_fn(target, loc);
        let m = metrics(&code, Language::Python);
        assert_eq!(func(&m, "fun").short_var_count, target, "short var fixture calibration");
        assert_fires(&code, Language::Python, Smell::ShortVariableNames, expected, "short var count boundary");
    }
}

#[test]
fn short_var_loc_gate_boundary() {
    let min_loc = t().analysis.short_var_min_fn_loc;
    let count = t().analysis.short_var_max_count + 1;
    for (target, expected) in [(min_loc - 1, false), (min_loc, true), (min_loc + 1, true)] {
        let code = short_var_fn(count, target);
        let m = metrics(&code, Language::Python);
        assert_eq!(func(&m, "fun").loc, target, "short var loc fixture calibration");
        assert_fires(&code, Language::Python, Smell::ShortVariableNames, expected, "short var loc gate boundary");
    }
}

// ── primitive obsession: typed-param and same-primitive gates fire AT threshold (>=) ──

#[test]
fn primitive_min_typed_params_boundary() {
    let w = t().analysis.primitive_min_typed_params;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let args: Vec<String> = (0..target).map(|i| format!("a{i}: int")).collect();
        let code = format!("def f({}):\n    x = 0\n", args.join(", "));
        let m = metrics(&code, Language::Python);
        assert_eq!(func(&m, "f").typed_param_count, target, "typed param fixture calibration");
        assert_fires(&code, Language::Python, Smell::PrimitiveObsession, expected, "min typed params boundary");
    }
}

#[test]
fn primitive_min_same_count_boundary() {
    let w = t().analysis.primitive_min_same_count;
    let total = t().analysis.primitive_min_typed_params.max(w + 2);
    let prim_kinds = ["int", "float", "bool", "bytes", "str", "complex"];
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let mut args: Vec<String> = (0..target).map(|i| format!("a{i}: int")).collect();
        for (i, kind) in prim_kinds.iter().skip(1).take((total - target) as usize).enumerate() {
            args.push(format!("b{i}: {kind}"));
        }
        let code = format!("def f({}):\n    x = 0\n", args.join(", "));
        let m = metrics(&code, Language::Python);
        let f = func(&m, "f");
        assert_eq!(f.max_same_primitive_count, target, "same primitive fixture calibration");
        assert_eq!(f.typed_param_count, total, "same primitive fixture total");
        assert_fires(&code, Language::Python, Smell::PrimitiveObsession, expected, "min same count boundary");
    }
}

// ── module loc: FileTooLarge fires AT warning (>=) ──

#[test]
fn file_loc_boundary() {
    let w = t().module.file_loc_warning;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let mut code = String::new();
        for i in 0..target {
            code.push_str(&format!("x{i} = {i}\n"));
        }
        let m = metrics(&code, Language::Python);
        assert_eq!(m.module.total_loc, target, "file loc fixture calibration");
        assert_fires(&code, Language::Python, Smell::FileTooLarge, expected, "file loc boundary");
    }
}

// ── function count: TooManyFunctions fires only ABOVE threshold (strict >) ──

#[test]
fn file_function_count_boundary() {
    let w = t().module.file_function_count;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let mut code = String::new();
        for i in 0..target {
            code.push_str(&format!("def g{i}():\n    return {i}\n"));
        }
        let m = metrics(&code, Language::Python);
        assert_eq!(m.module.total_functions, target, "fn count fixture calibration");
        assert_fires(&code, Language::Python, Smell::TooManyFunctions, expected, "fn count boundary");
    }
}

// ── total cc: OverallCodeComplexity fires only ABOVE threshold (strict >) ──

#[test]
fn file_total_cc_boundary() {
    let w = t().module.file_total_cc;
    let unit = t().function.cc_warning - 2;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let full = target / unit;
        let rest = target % unit;
        let mut code = String::new();
        for i in 0..full {
            code.push_str(&seq_if_fn(&format!("g{i}"), unit - 1));
        }
        if rest > 0 {
            code.push_str(&seq_if_fn("gr", rest - 1));
        }
        let m = metrics(&code, Language::Python);
        assert_eq!(m.module.sum_cc, target, "total cc fixture calibration");
        assert_fires(&code, Language::Python, Smell::OverallCodeComplexity, expected, "total cc boundary");
    }
}

// ── declarations: ExcessiveDeclarations fires only ABOVE threshold (strict >) ──

#[test]
fn max_declarations_boundary() {
    let w = t().module.max_declarations;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let mut code = String::new();
        for i in 0..target {
            code.push_str(&format!("class K{i}:\n    n = {i}\n"));
        }
        let m = metrics(&code, Language::Python);
        assert_eq!(m.module.declaration_count, target, "declaration fixture calibration");
        assert_fires(&code, Language::Python, Smell::ExcessiveDeclarations, expected, "declaration boundary");
    }
}

// ── struct fields: LargeStruct fires only ABOVE threshold (strict >); Rust carries struct fields ──

#[test]
fn max_struct_fields_boundary() {
    let w = t().module.max_struct_fields;
    for (target, expected) in [(w - 1, false), (w, false), (w + 1, true)] {
        let fields: Vec<String> = (0..target).map(|i| format!("    f{i}: u32,")).collect();
        let code = format!("struct S {{\n{}\n}}\n", fields.join("\n"));
        let m = metrics(&code, Language::Rust);
        assert_eq!(m.module.struct_fields[0].1, target, "struct field fixture calibration");
        assert_fires(&code, Language::Rust, Smell::LargeStruct, expected, "struct field boundary");
    }
}

// ── global conditionals: fires only ABOVE threshold (strict >, default 0) ──

#[test]
fn global_conditionals_boundary() {
    let w = t().module.global_conditionals_max;
    for (target, expected) in [(w, false), (w + 1, true), (w + 2, true)] {
        let mut code = String::from("flag = 1\n");
        for _ in 0..target {
            code.push_str("if flag:\n    y = 0\n");
        }
        let m = metrics(&code, Language::Python);
        assert_eq!(m.module.global_conditional_count, target, "global conditional fixture calibration");
        assert_fires(&code, Language::Python, Smell::GlobalConditionals, expected, "global conditional boundary");
    }
}

// ── global nesting: DeepGlobalNesting fires AT threshold (>=) ──

#[test]
fn global_nesting_boundary() {
    let w = t().module.global_nesting_depth;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let mut code = String::from("flag = 1\n");
        for d in 0..target {
            code.push_str(&"    ".repeat(d as usize));
            code.push_str("if flag:\n");
        }
        code.push_str(&"    ".repeat(target as usize));
        code.push_str("y = 0\n");
        let m = metrics(&code, Language::Python);
        assert_eq!(m.module.global_max_nesting, target, "global nesting fixture calibration");
        assert_fires(&code, Language::Python, Smell::DeepGlobalNesting, expected, "global nesting boundary");
    }
}

// ── lcom4: LowCohesion fires AT threshold (>=) ──

#[test]
fn lcom4_boundary() {
    let w = t().analysis.lcom4_warning;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let mut code = String::from("class C:\n    def m0a(self):\n        self.g0 = 0\n");
        code.push_str("    def m0b(self):\n        return self.g0\n");
        for g in 1..target {
            code.push_str(&format!("    def m{g}(self):\n        return self.g{g}\n"));
        }
        let f = findings(&code, Language::Python);
        let fired = has(&f, Smell::LowCohesion);
        assert_eq!(fired, expected, "lcom4 boundary at {target}: {f:#?}");
        if expected {
            let detail = f.iter().find(|x| x.smell == Smell::LowCohesion).map(|x| x.detail.clone()).unwrap_or_default();
            assert!(detail.contains(&format!("LCOM4={target}")), "lcom4 fixture calibration: {detail}");
        }
    }
}

// ── overall function size: both count (>=) and per-fn loc (>=) gates ──

#[test]
fn overall_function_size_count_boundary() {
    let count_w = t().module.large_fn_count;
    let loc = t().module.large_fn_loc + 5;
    for (target, expected) in [(count_w - 1, false), (count_w, true), (count_w + 1, true)] {
        let mut code = String::new();
        for i in 0..target {
            code.push_str(&fn_with_cc_and_loc(&format!("g{i}"), 2, loc));
        }
        if target == 0 {
            code.push_str("x = 0\n");
        }
        assert_fires(&code, Language::Python, Smell::OverallFunctionSize, expected, "overall size count boundary");
    }
}

#[test]
fn overall_function_size_loc_boundary() {
    let loc_w = t().module.large_fn_loc;
    let count = t().module.large_fn_count;
    for (target, expected) in [(loc_w - 1, false), (loc_w, true), (loc_w + 1, true)] {
        let mut code = String::new();
        for i in 0..count {
            code.push_str(&fn_with_cc_and_loc(&format!("g{i}"), 2, target));
        }
        let m = metrics(&code, Language::Python);
        assert_eq!(func(&m, "g0").loc, target, "overall size loc fixture calibration");
        assert_fires(&code, Language::Python, Smell::OverallFunctionSize, expected, "overall size loc boundary");
    }
}

// ── duplication: exact-clone eligibility loc gate fires AT threshold (>=) ──

fn clone_body(lines: u32) -> String {
    let mut body = String::from("    if x:\n        x = x + 1\n");
    for i in 3..lines {
        body.push_str(&format!("    v{i} = x + {i}\n"));
    }
    body.push_str("    return x\n");
    body
}

fn clone_pair(loc: u32) -> String {
    let body = clone_body(loc - 1);
    format!("def alpha(x):\n{body}def beta(x):\n{body}")
}

#[test]
fn duplication_min_loc_boundary() {
    let w = t().analysis.duplication.min_loc;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let code = clone_pair(target);
        let m = metrics(&code, Language::Python);
        let a = func(&m, "alpha");
        let b = func(&m, "beta");
        assert_eq!(a.loc, target, "clone fixture loc calibration");
        assert_eq!(a.structural_hash, b.structural_hash, "clone fixture must be exact clones");
        assert!(
            a.distinct_node_kinds >= t().analysis.duplication.min_distinct_kinds,
            "clone fixture kind variety: {}",
            a.distinct_node_kinds
        );
        assert_fires(&code, Language::Python, Smell::CodeDuplication, expected, "duplication loc boundary");
    }
}

// ── duplication: group size fires AT threshold (>=) ──

#[test]
fn duplication_min_group_boundary() {
    let w = t().analysis.duplication.min_group;
    let loc = t().analysis.duplication.min_loc + 2;
    let body = clone_body(loc - 2);
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let mut code = String::new();
        for i in 0..target {
            code.push_str(&format!("def copy{i}(x):\n{body}"));
        }
        if target == 0 {
            code.push_str("x = 0\n");
        }
        assert_fires(&code, Language::Python, Smell::CodeDuplication, expected, "duplication group boundary");
    }
}

// ── duplication: distinct-kind variety gate fires AT threshold (>=) ──

#[test]
fn duplication_min_distinct_kinds_gate() {
    let min_kinds = t().analysis.duplication.min_distinct_kinds;
    let loc = t().analysis.duplication.min_loc + 2;

    let monotone_body = "    pass\n".repeat((loc - 2) as usize);
    let monotone = format!("def alpha(x):\n{monotone_body}def beta(x):\n{monotone_body}");
    let m = metrics(&monotone, Language::Python);
    let kinds = func(&m, "alpha").distinct_node_kinds;
    assert!(kinds < min_kinds, "monotone fixture must sit below the variety gate, got {kinds}");
    assert_fires(&monotone, Language::Python, Smell::CodeDuplication, false, "low-variety clones suppressed");

    let varied = clone_pair(loc);
    let mv = metrics(&varied, Language::Python);
    assert!(func(&mv, "alpha").distinct_node_kinds >= min_kinds, "varied fixture above gate");
    assert_fires(&varied, Language::Python, Smell::CodeDuplication, true, "varied clones reported");
}

// ── skeleton duplication: fuzzy loc gate fires AT threshold (>=) ──

#[test]
fn skeleton_duplication_min_loc_boundary() {
    let threshold = t().analysis.duplication.skeleton_min_loc;
    for (target, expected) in [(threshold - 1, false), (threshold, true), (threshold + 1, true)] {
        let pad_lines = target - 4;
        let head = "    if x:\n        x = x + 1\n";
        let mut tail = String::new();
        for i in 0..pad_lines {
            tail.push_str(&format!("    v{i} = x + {i}\n"));
        }
        let body_a = format!("{head}{tail}    return x\n");
        let body_b = format!("{tail}{head}    return x\n");
        let code = format!("def alpha(x):\n{body_a}def beta(x):\n{body_b}");
        let parsed = metrics(&code, Language::Python);
        let alpha = func(&parsed, "alpha");
        let beta = func(&parsed, "beta");
        assert_eq!(alpha.loc, target, "skeleton fixture loc calibration");
        assert_ne!(alpha.structural_hash, beta.structural_hash, "skeleton fixture must not be exact clones");
        assert_eq!(alpha.skeleton_hash, beta.skeleton_hash, "skeleton fixture must share skeletons");
        assert_fires(&code, Language::Python, Smell::CodeDuplication, expected, "skeleton loc boundary");
    }
}

// ── duplicated assertion blocks: participation begins AT the minimum (>=) ──

#[test]
fn dup_assert_min_boundary() {
    let w = t().analysis.dup_assert_min;
    for (target, expected) in [(w - 1, false), (w, true), (w + 1, true)] {
        let mut asserts = String::new();
        for i in 0..target {
            asserts.push_str(&format!("    assert x == {i}\n"));
        }
        let code = format!("def test_alpha(x):\n{asserts}def test_beta(x):\n{asserts}");
        let m = metrics(&code, Language::Python);
        assert_eq!(func(&m, "test_alpha").consecutive_asserts, target, "dup assert fixture calibration");
        assert_fires(&code, Language::Python, Smell::DuplicatedAssertionBlocks, expected, "dup assert boundary");
    }
}
