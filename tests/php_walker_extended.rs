use pulse::parse::{parse_and_walk, Language};

fn analyze(source: &str) -> pulse::walk::FileMetrics {
    parse_and_walk(source, Language::Php).expect("parse PHP")
}

#[test]
fn match_expression_handled() {
    let source = "<?php\nfunction f($x) {\n    return match($x) {\n        1 => 'one',\n        2 => 'two',\n        default => 'other',\n    };\n}\n";
    let _ = analyze(source);
}

#[test]
fn union_type_parameter_handled() {
    let source = "<?php\nfunction f(int|string $x): mixed {\n    return $x;\n}\n";
    let _ = analyze(source);
}

#[test]
fn intersection_type_parameter_handled() {
    let source = "<?php\nfunction f(Countable&Iterator $x): int {\n    return count($x);\n}\n";
    let _ = analyze(source);
}

#[test]
fn arrow_function_handled() {
    let source = "<?php\n$double = fn($x) => $x * 2;\n";
    let _ = analyze(source);
}

#[test]
fn anonymous_function_with_use_clause_handled() {
    let source = "<?php\n$captured = 10;\n$fn = function($x) use ($captured) { return $x + $captured; };\n";
    let _ = analyze(source);
}

#[test]
fn property_promotion_in_constructor_handled() {
    let source = "<?php\nclass Point {\n    public function __construct(\n        public readonly int $x,\n        public readonly int $y,\n    ) {}\n}\n";
    let _ = analyze(source);
}

#[test]
fn nullable_type_handled() {
    let source = "<?php\nfunction f(?string $x): ?int {\n    return $x === null ? null : strlen($x);\n}\n";
    let _ = analyze(source);
}

#[test]
fn switch_with_cases_and_default_handled() {
    let source = "<?php\nfunction f($x) {\n    switch ($x) {\n        case 1: return 'one';\n        case 2: return 'two';\n        default: return 'other';\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn try_catch_finally_handled() {
    let source = "<?php\nfunction f() {\n    try {\n        risky();\n    } catch (\\TypeError $e) {\n        log($e);\n    } catch (\\Exception $e) {\n        rethrow($e);\n    } finally {\n        cleanup();\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn enum_declaration_handled() {
    let source = "<?php\nenum Status: string {\n    case Active = 'active';\n    case Inactive = 'inactive';\n}\n";
    let _ = analyze(source);
}

#[test]
fn class_with_traits_handled() {
    let source = "<?php\ntrait Greeter {\n    public function hello() { return 'hi'; }\n}\nclass M {\n    use Greeter;\n}\n";
    let _ = analyze(source);
}

#[test]
fn class_with_interfaces_handled() {
    let source = "<?php\ninterface A {}\ninterface B {}\nclass M implements A, B {}\n";
    let _ = analyze(source);
}

#[test]
fn complex_method_yields_high_cc() {
    let source = "<?php\nfunction f($x, $y) {\n    if ($x > 0) {\n        if ($y > 0) {\n            for ($i = 0; $i < $y; $i++) {\n                if ($i % 2 == 0) echo $i;\n            }\n        } elseif ($y < 0) {\n            $x = -$x;\n        }\n    }\n    return $x;\n}\n";
    let metrics = analyze(source);
    let max_cc = metrics.functions.iter().map(|f| f.cc).max().unwrap_or(0);
    assert!(max_cc >= 3);
}

#[test]
fn ternary_operator_handled() {
    let source = "<?php\nfunction f($x) { return $x > 0 ? 1 : -1; }\n";
    let _ = analyze(source);
}

#[test]
fn null_coalescing_operator_handled() {
    let source = "<?php\nfunction f($x) { return $x ?? 'default'; }\n";
    let _ = analyze(source);
}

#[test]
fn spread_operator_in_args_handled() {
    let source = "<?php\nfunction sum(int ...$nums): int { return array_sum($nums); }\n";
    let _ = analyze(source);
}

#[test]
fn class_with_abstract_methods_handled() {
    let source = "<?php\nabstract class Base {\n    abstract public function process();\n}\n";
    let _ = analyze(source);
}

#[test]
fn class_with_final_methods_handled() {
    let source = "<?php\nclass M {\n    final public function f() { return 1; }\n}\n";
    let _ = analyze(source);
}

#[test]
fn static_method_handled() {
    let source = "<?php\nclass M {\n    public static function create() { return new self(); }\n}\n";
    let _ = analyze(source);
}

#[test]
fn while_and_do_while_loops_handled() {
    let source = "<?php\nfunction f($n) {\n    $i = 0;\n    while ($i < $n) { $i++; }\n    do { $i--; } while ($i > 0);\n}\n";
    let _ = analyze(source);
}

#[test]
fn foreach_loop_handled() {
    let source = "<?php\nfunction f($arr) {\n    foreach ($arr as $k => $v) {\n        echo \"$k=$v\\n\";\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn empty_php_file_no_panic() {
    let _ = analyze("<?php\n");
}

#[test]
fn malformed_php_no_panic() {
    let _ = analyze("<?php\nfunction f(.\n");
}

#[test]
fn class_with_const_handled() {
    let source = "<?php\nclass M {\n    const PI = 3.14;\n    const E = 2.71;\n}\n";
    let _ = analyze(source);
}

#[test]
fn interface_with_methods_handled() {
    let source = "<?php\ninterface Runnable {\n    public function run(): void;\n}\n";
    let _ = analyze(source);
}
