mod common;

use common::*;

lang_helpers!("php");

// ===========================================================================
// CC counting
// ===========================================================================

#[test]
fn cc_base() {
    let out = debug("<?php\nfunction f(): int { return 1; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn cc_if() {
    let out = debug("<?php\nfunction f(bool $x): int { if ($x) { return 1; } return 0; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_elseif() {
    let out = debug("<?php\nfunction f(int $x): int { if ($x > 2) { return 2; } elseif ($x > 1) { return 1; } return 0; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_else_no_increment() {
    let out = debug("<?php\nfunction f(bool $x): int { if ($x) { return 1; } else { return 0; } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_for() {
    let out = debug("<?php\nfunction f(): void { for ($i = 0; $i < 10; $i++) { echo $i; } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_foreach() {
    let out = debug("<?php\nfunction f(array $a): void { foreach ($a as $v) { echo $v; } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_while() {
    let out = debug("<?php\nfunction f(): void { $i = 0; while ($i < 10) { $i++; } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_do_while() {
    let out = debug("<?php\nfunction f(): void { $i = 0; do { $i++; } while ($i < 10); }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_switch_cases() {
    let out = debug("<?php\nfunction f(int $x): string {\n  switch ($x) {\n    case 1: return 'a';\n    case 2: return 'b';\n    case 3: return 'c';\n    default: return 'd';\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_match_arms() {
    let out = debug("<?php\nfunction f(int $x): string {\n  return match($x) { 1 => 'a', 2 => 'b', 3 => 'c', default => 'd' };\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_catch() {
    let out = debug("<?php\nfunction f(): int { try { return 1; } catch (Exception $e) { return 0; } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_finally_no_increment() {
    let out = debug("<?php\nfunction f(): void { try { echo 'a'; } finally { echo 'b'; } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn cc_ternary() {
    let out = debug("<?php\nfunction f(int $x): int { return $x > 0 ? 1 : 0; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_and_operator() {
    let out = debug("<?php\nfunction f(bool $a, bool $b): bool { if ($a && $b) { return true; } return false; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_or_operator() {
    let out = debug("<?php\nfunction f(bool $a, bool $b): bool { if ($a || $b) { return true; } return false; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_keyword_and() {
    let out = debug("<?php\nfunction f(bool $a, bool $b): bool { if ($a and $b) { return true; } return false; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_keyword_or() {
    let out = debug("<?php\nfunction f(bool $a, bool $b): bool { if ($a or $b) { return true; } return false; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("<?php\nfunction f(bool $a, bool $b, bool $c): bool { if ($a && $b || $c) { return true; } return false; }\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {}", cc);
}

#[test]
fn cc_if_with_else() {
    let out = debug("<?php\nfunction f(int $x): int { if ($x > 0) { return 1; } else { return 0; } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_case_else_not_counted() {
    let out = debug("<?php\nfunction f(int $x): string {\n  switch ($x) {\n    case 1: return 'a';\n    default: return '?';\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_sequential_ifs() {
    let out = debug("<?php\nfunction f(int $a, int $b, int $c): void { if ($a > 0) {} if ($b > 0) {} if ($c > 0) {} }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_nested_if() {
    let out = debug("<?php\nfunction f(bool $a, bool $b): void { if ($a) { if ($b) { echo 'x'; } } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_nested_loop_in_if() {
    let out = debug("<?php\nfunction f(bool $a): void { if ($a) { for ($i=0; $i<5; $i++) {} } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// ===========================================================================
// Cognitive complexity
// ===========================================================================

#[test]
fn cogc_flat_if() {
    let out = debug("<?php\nfunction f(bool $x): void { if ($x) { echo 'y'; } }\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_nested_if() {
    let out = debug("<?php\nfunction f(bool $a, bool $b): void { if ($a) { if ($b) { echo 'x'; } } }\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc > 2, "cogc={cogc}");
}

#[test]
fn cogc_elseif() {
    let out = debug("<?php\nfunction f(int $x): void { if ($x > 2) { echo '2'; } elseif ($x > 1) { echo '1'; } }\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 2, "cogc={cogc}");
}

#[test]
fn cogc_else() {
    let out = debug("<?php\nfunction f(bool $x): void { if ($x) { echo 'y'; } else { echo 'n'; } }\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 2, "cogc={cogc}");
}

#[test]
fn cogc_loop_with_nested_if() {
    let out = debug("<?php\nfunction f(array $a): void { foreach ($a as $v) { if ($v > 0) { echo $v; } } }\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc > 2, "cogc={cogc}");
}

#[test]
fn cogc_switch() {
    let out = debug("<?php\nfunction f(int $x): void { switch ($x) { case 1: echo 'a'; break; case 2: echo 'b'; break; } }\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 1, "cogc={cogc}");
}

#[test]
fn cogc_deep_nesting() {
    let out = debug("<?php\nfunction f(bool $a, bool $b, bool $c, bool $d): void {\n  if ($a) {\n    if ($b) {\n      if ($c) {\n        if ($d) { echo 'deep'; }\n      }\n    }\n  }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 10, "cogc={cogc}");
}

#[test]
fn cogc_sequential_low() {
    let out = debug("<?php\nfunction f(bool $a, bool $b): void { if ($a) { echo '1'; } if ($b) { echo '2'; } }\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert_eq!(cogc, 2);
}

// ===========================================================================
// Nesting depth
// ===========================================================================

#[test]
fn nesting_simple_if() {
    let out = debug("<?php\nfunction f(bool $x): void { if ($x) { echo 'y'; } }\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_three_deep() {
    let out = debug("<?php\nfunction f(bool $a, bool $b, bool $c): void { if ($a) { if ($b) { if ($c) { echo 'x'; } } } }\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_five_deep() {
    let out = debug("<?php\nfunction f(bool $a, bool $b, bool $c, bool $d, bool $e): void {\n  if ($a) { if ($b) { if ($c) { if ($d) { if ($e) { echo 'x'; } } } } }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

#[test]
fn nesting_sequential_not_accumulated() {
    let out = debug("<?php\nfunction f(bool $a, bool $b): void { if ($a) { echo '1'; } if ($b) { echo '2'; } }\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_loop_with_if() {
    let out = debug("<?php\nfunction f(array $a): void { for ($i=0; $i<count($a); $i++) { if ($a[$i] > 0) { echo $a[$i]; } } }\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

// ===========================================================================
// Bumpy road
// ===========================================================================

#[test]
fn bumpy_road_two_chunks() {
    let out = check("<?php\nfunction f(int $a, int $b, int $c): void {\n  if ($a > 0) { if ($b > 0) { if ($c > 0) { echo 'x'; } } }\n  echo 'mid';\n  if ($a > 0) { if ($b > 0) { if ($c > 0) { echo 'y'; } } }\n}\n");
    assert!(has_smell(&out, "Nested Conditional Chunks"), "got: {}", out);
}

#[test]
fn bumpy_road_single_chunk_not_flagged() {
    let out = check("<?php\nfunction f(int $a, int $b, int $c): void {\n  if ($a > 0) { if ($b > 0) { if ($c > 0) { echo 'x'; } } }\n}\n");
    assert!(!has_smell(&out, "Nested Conditional Chunks"), "got: {}", out);
}

// ===========================================================================
// Argument counting
// ===========================================================================

#[test]
fn args_zero() {
    let out = debug("<?php\nfunction f(): void { echo 'hi'; }\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_three() {
    let out = debug("<?php\nfunction f(int $a, int $b, int $c): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_above_threshold_flagged() {
    let n = args_above();
    let params: Vec<String> = (0..n).map(|i| format!("int $p{i}")).collect();
    let code = format!("<?php\nfunction f({}): void {{}}\n", params.join(", "));
    let out = check(&code);
    assert!(has_smell(&out, "Excess Arguments"), "got: {}", out);
}

#[test]
fn args_default_counted() {
    let out = debug("<?php\nfunction f(int $a, int $b = 0, int $c = 1): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_variadic_counted() {
    let out = debug("<?php\nfunction f(int $a, string ...$rest): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

#[test]
fn args_typed_counted() {
    let out = debug("<?php\nfunction f(int $a, string $b): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ===========================================================================
// LOC counting
// ===========================================================================

#[test]
fn loc_single_line() {
    let out = debug("<?php\nfunction f(): int { return 1; }\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(1));
}

#[test]
fn loc_multiline() {
    let out = debug("<?php\nfunction f(): int {\n  $x = 1;\n  $y = 2;\n  return $x + $y;\n}\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(5));
}

#[test]
fn loc_comments_excluded_from_code_lines() {
    let out = debug("<?php\n// top comment\nfunction f(): int {\n  // inner comment\n  return 1;\n}\n");
    assert!(out.contains("LOC"), "debug output should contain module LOC");
}

// ===========================================================================
// Embedded blocks
// ===========================================================================

#[test]
fn heredoc_above_threshold() {
    let n = embedded_lines_above();
    let lines: Vec<String> = (0..n).map(|i| format!("    line {i}")).collect();
    let code = format!("<?php\nfunction f(): string {{\n  return <<<EOT\n{}\n  EOT;\n}}\n", lines.join("\n"));
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {}", out);
}

#[test]
fn small_string_not_flagged() {
    let out = check("<?php\nfunction f(): string { return \"hello\"; }\n");
    assert!(!has_smell(&out, "Large Embedded Block"), "got: {}", out);
}

// ===========================================================================
// Duplication
// ===========================================================================

#[test]
fn exact_duplication() {
    let body = "  $t = 0;\n  foreach ($a as $v) {\n    if ($v > 0) { $t += $v; }\n  }\n  return $t;\n";
    let code = format!("<?php\nfunction fa(array $a): int {{\n{body}}}\nfunction fb(array $a): int {{\n{body}}}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

#[test]
fn below_min_loc_no_duplication() {
    let code = "<?php\nfunction fa(): int { return 1; }\nfunction fb(): int { return 1; }\n";
    let out = check(code);
    assert!(!has_smell(&out, "Code Duplication"), "got: {}", out);
}

#[test]
fn test_prefix_duplication_suppressed() {
    let body = "  $t = 0;\n  foreach ($a as $v) {\n    if ($v > 0) { $t += $v; }\n  }\n  return $t;\n";
    let code = format!("<?php\nfunction test_a(array $a): int {{\n{body}}}\nfunction test_b(array $a): int {{\n{body}}}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Code Duplication"), "got: {}", out);
}

// ===========================================================================
// Assertion blocks
// ===========================================================================

#[test]
fn consecutive_asserts_above_threshold() {
    let n = asserts_above();
    let asserts: Vec<String> = (0..n).map(|i| format!("  assert({i} === {i});")).collect();
    let code = format!("<?php\nfunction f(): void {{\n{}\n}}\n", asserts.join("\n"));
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"), "got: {}", out);
}

#[test]
fn asserts_below_threshold_not_flagged() {
    let code = "<?php\nfunction f(): void {\n  assert(1 === 1);\n  assert(2 === 2);\n}\n";
    let out = check(code);
    assert!(!has_smell(&out, "Large Assertion Block"), "got: {}", out);
}

#[test]
fn assert_interruption_resets() {
    let half = asserts_at() / 2;
    let asserts: Vec<String> = (0..half).map(|i| format!("  assert({i} === {i});")).collect();
    let code = format!("<?php\nfunction f(): void {{\n{}\n  echo 'break';\n{}\n}}\n", asserts.join("\n"), asserts.join("\n"));
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"), "got: {}", out);
}

// ===========================================================================
// Compound conditions
// ===========================================================================

#[test]
fn compound_condition_detected() {
    let out = check("<?php\nfunction f(bool $a, bool $b, bool $c): void {\n  if ($a && $b || $c) { echo '1'; }\n  if ($a || $b && $c) { echo '2'; }\n  if ($a && $b || $c) { echo '3'; }\n}\n");
    assert!(has_smell(&out, "Complex Conditional"), "got: {}", out);
}

#[test]
fn simple_condition_not_flagged() {
    let out = check("<?php\nfunction f(bool $a, bool $b): void { if ($a && $b) { echo 'x'; } }\n");
    assert!(!has_smell(&out, "Complex Conditional"), "got: {}", out);
}

// ===========================================================================
// Primitive obsession
// ===========================================================================

#[test]
fn all_primitives_flagged() {
    let out = check("<?php\nfunction f(int $a, string $b, float $c, bool $d, int $e): void {}\n");
    assert!(has_smell(&out, "Primitive Obsession"), "got: {}", out);
}

#[test]
fn mixed_types_ratio() {
    let out = check("<?php\nfunction f(int $a, DateTime $b, int $c, Logger $d): void {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"), "got: {}", out);
}

#[test]
fn no_type_hints_not_flagged() {
    let out = check("<?php\nfunction f($a, $b, $c, $d, $e): void {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"), "got: {}", out);
}

// ===========================================================================
// LCOM4 / cohesion
// ===========================================================================

#[test]
fn lcom4_shared_fields_cohesive() {
    let out = check(concat!(
        "<?php\nclass C {\n",
        "  public function getA(): int { return $this->data; }\n",
        "  public function getB(): int { return $this->data + 1; }\n",
        "  public function getC(): int { return $this->data + 2; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn lcom4_disjoint_fields() {
    let out = check(concat!(
        "<?php\nclass K {\n",
        "  public function a(): int { return $this->x; }\n",
        "  public function b(): int { return $this->y; }\n",
        "  public function c(): int { return $this->z; }\n",
        "  public function d(): int { return $this->w; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn lcom4_single_method_no_smell() {
    let out = check("<?php\nclass C {\n  public function f(): int { return $this->x; }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "<?php\nclass Coord {\n",
        "    private $state = 0;\n",
        "    public function process($e) { return $this->validate($e) && $this->dispatch($e); }\n",
        "    public function validate($e) { return $e > 0; }\n",
        "    public function dispatch($e) { return $this->send($e); }\n",
        "    public function send($e) { return true; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "<?php\nclass Mixed {\n",
        "    private $x = 0;\n",
        "    public function a() { return $this->x; }\n",
        "    public function b() { $this->x = 1; return $this->c(); }\n",
        "    public function c() { return 42; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_class_still_fires() {
    let out = check(concat!(
        "<?php\nclass UserService {\n",
        "    private $db; private $cache; private $mailer; private $events; private $audit;\n",
        "    public function getUser() { return $this->db; }\n",
        "    public function cacheUser() { return $this->cache; }\n",
        "    public function sendWelcome() { return $this->mailer; }\n",
        "    public function publish() { return $this->events; }\n",
        "    public function auditLog() { return $this->audit; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "<?php\nclass Service {\n",
        "    private $db; private $cache; private $log;\n",
        "    public function a() { return $this->db; }\n",
        "    public function b() { return $this->cache; }\n",
        "    public function c() { return $this->log; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Method naming
// ===========================================================================

#[test]
fn standalone_function_no_prefix() {
    let out = debug("<?php\nfunction helper(): int { return 1; }\n");
    assert!(out.contains("helper"), "got: {}", out);
    assert!(!out.contains(".helper"), "got: {}", out);
}

#[test]
fn class_method_has_prefix() {
    let out = debug("<?php\nclass Foo {\n  public function bar(): int { return 1; }\n}\n");
    assert!(out.contains("Foo.bar"), "got: {}", out);
}

#[test]
fn constructor_has_prefix() {
    let out = debug("<?php\nclass Foo {\n  public function __construct() {}\n}\n");
    assert!(out.contains("Foo.__construct"), "got: {}", out);
}

// ===========================================================================
// PHP-specific constructs
// ===========================================================================

#[test]
fn switch_with_default_cc() {
    let out = debug("<?php\nfunction f(int $x): int { switch ($x) { case 1: return 1; default: return 0; } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn match_with_default_cc() {
    let out = debug("<?php\nfunction f(int $x): int { return match($x) { 1 => 10, 2 => 20, default => 0 }; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn catch_multiple_types() {
    let out = debug("<?php\nfunction f(): void { try { echo 'a'; } catch (RuntimeException | LogicException $e) { echo 'b'; } }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn empty_catch_flagged() {
    let out = check("<?php\nfunction f(): void { try { echo 'a'; } catch (Exception $e) {} }\n");
    assert!(has_smell(&out, "Empty Error Handler"), "got: {}", out);
}

#[test]
fn non_empty_catch_not_flagged() {
    let out = check("<?php\nfunction f(): void { try { echo 'a'; } catch (Exception $e) { echo $e->getMessage(); } }\n");
    assert!(!has_smell(&out, "Empty Error Handler"), "got: {}", out);
}

#[test]
fn arrow_fn_scope_boundary() {
    let out = debug("<?php\nfunction f(): callable { return fn(int $x): int => $x > 0 ? 1 : 0; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn closure_scope_boundary() {
    let out = debug("<?php\nfunction f(): callable { return function(): int { if (true) { return 1; } return 0; }; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn closure_with_use() {
    let out = debug("<?php\nfunction f(int $y): callable { return function(int $x) use ($y): int { return $x + $y; }; }\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn namespace_function_found() {
    let out = debug("<?php\nnamespace App\\Util;\nfunction calc(int $x): int { return $x * 2; }\n");
    assert!(out.contains("calc"), "got: {}", out);
    assert_eq!(function_metric(&out, "calc", "cc"), Some(1));
}

#[test]
fn constructor_promotion_counted() {
    let out = debug("<?php\nclass S {\n  public function __construct(\n    private int $a,\n    private string $b,\n    private float $c\n  ) {}\n}\n");
    assert_eq!(function_metric(&out, "__construct", "args"), Some(3));
}

#[test]
fn field_access_this() {
    let out = debug("<?php\nclass C {\n  public function getX(): int { return $this->x; }\n  public function getY(): int { return $this->y; }\n}\n");
    assert!(out.contains("fields=[\"x\"]"), "got: {}", out);
    assert!(out.contains("fields=[\"y\"]"), "got: {}", out);
}

#[test]
fn heredoc_embedded() {
    let n = embedded_lines_above();
    let lines: Vec<String> = (0..n).map(|i| format!("  line {i}")).collect();
    let code = format!("<?php\nfunction f(): string {{\n  return <<<EOT\n{}\nEOT;\n}}\n", lines.join("\n"));
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {}", out);
}

#[test]
fn string_match_arms_flagged() {
    let out = check("<?php\nfunction f(string $s): int {\n  return match($s) { 'a' => 1, 'b' => 2, 'c' => 3, 'd' => 4, 'e' => 5, 'f' => 6, default => 0 };\n}\n");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "got: {}", out);
}

#[test]
fn trait_methods_found() {
    let out = debug("<?php\ntrait Greetable {\n  public function greet(): string { return 'hello'; }\n}\n");
    assert!(out.contains("Greetable.greet"), "got: {}", out);
}

#[test]
fn interface_methods_skipped_without_body() {
    let out = debug("<?php\ninterface I {\n  public function doThing(): void;\n}\n");
    assert!(!out.contains("doThing"), "interface method without body should not appear, got: {}", out);
}

#[test]
fn enum_methods_found() {
    let out = debug("<?php\nenum Color {\n  case Red;\n  case Blue;\n  public function label(): string { return 'color'; }\n}\n");
    assert!(out.contains("Color.label"), "got: {}", out);
}

// ===========================================================================
// Performance
// ===========================================================================

#[test]
fn performance_large_file() {
    let fns: Vec<String> = (0..50).map(|i| format!("function fn_{i}(): int {{ return {i}; }}")).collect();
    let code = format!("<?php\n{}\n", fns.join("\n"));
    let start = std::time::Instant::now();
    let _ = check(&code);
    assert!(start.elapsed().as_millis() < 500, "took {}ms", start.elapsed().as_millis());
}

#[test]
fn performance_class_hierarchy() {
    let mut classes = Vec::new();
    for c in 0..10 {
        let methods: Vec<String> = (0..5).map(|m| format!("  public function m_{m}(): int {{ return {m}; }}")).collect();
        classes.push(format!("class C_{c} {{\n{}\n}}", methods.join("\n")));
    }
    let code = format!("<?php\n{}\n", classes.join("\n"));
    let start = std::time::Instant::now();
    let _ = check(&code);
    assert!(start.elapsed().as_millis() < 500, "took {}ms", start.elapsed().as_millis());
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn clean_class_no_smells() {
    let out = check("<?php\nclass C {\n  public function f(): int { return 1; }\n  public function g(): int { return 2; }\n}\n");
    assert!(out.is_empty(), "got: {}", out);
}

#[test]
fn comments_only_silent() {
    let out = check("<?php\n// comment\n# another\n/* block */\n");
    assert!(out.is_empty(), "got: {}", out);
}

#[test]
fn empty_file_no_crash() {
    let out = check("<?php\n");
    assert!(out.is_empty(), "got: {}", out);
}

#[test]
fn empty_method_body() {
    let out = debug("<?php\nfunction f(): void {}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn multiple_independent_functions() {
    let out = debug("<?php\nfunction a(): int { return 1; }\nfunction b(): int { return 2; }\nfunction c(): int { return 3; }\n");
    assert!(out.contains("a") && out.contains("b") && out.contains("c"), "got: {}", out);
}

// ===========================================================================
// Additional cogc tests
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug("<?php\nfunction f(bool $a, bool $b, bool $c): void { if ($a) {} if ($b) {} if ($c) {} }\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_elseif_no_extra_nesting() {
    let out = debug(concat!(
        "<?php\nfunction f(int $x): void {\n",
        "  if ($x == 1) {}\n",
        "  elseif ($x == 2) {}\n",
        "  elseif ($x == 3) {}\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "elseif chain should have cogc >= 3, got: {cogc}");
}

#[test]
fn cogc_switch_counted() {
    let out = debug("<?php\nfunction f(int $x): void { switch ($x) { case 1: break; } }\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_else_increases_nesting() {
    let out = debug(concat!(
        "<?php\nfunction f(int $x): void {\n",
        "  if ($x > 0) {}\n",
        "  else {\n",
        "    if ($x < -10) {}\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "else should contribute to nesting, got: {}", cogc);
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug("<?php\nfunction f(bool $a, bool $b): bool { if ($a && $b) { return true; } return false; }\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "got: {}", cogc);
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug("<?php\nfunction f(bool $a, bool $b, bool $c): bool { if ($a && $b || $c) { return true; } return false; }\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "got: {}", cogc);
}

#[test]
fn cogc_triggers_complex_method() {
    let mut code = String::from("<?php\nfunction f(int $x): int {\n");
    for _ in 0..4 {
        code.push_str("  if ($x > 0) { if ($x > 1) { if ($x > 2) {} } }\n");
    }
    code.push_str("  return 0;\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Complex Method"), "got: {}", out);
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check("<?php\nfunction f(int $x): int { if ($x > 0) {} if ($x > 1) {} return 0; }\n");
    assert!(!has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Additional nesting tests
// ===========================================================================

#[test]
fn nesting_switch_with_nested_if() {
    let out = debug(concat!(
        "<?php\nfunction f(int $x): void {\n",
        "  switch ($x) {\n",
        "    case 1:\n",
        "      if (true) {}\n",
        "      break;\n",
        "  }\n",
        "}\n",
    ));
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 2, "got: {}", nesting);
}

// ===========================================================================
// Bumpy road metrics
// ===========================================================================

#[test]
fn bumpy_road_metric_two_bumps() {
    let out = debug(concat!(
        "<?php\nfunction f(bool $a, bool $b, bool $c, bool $d): void {\n",
        "  if ($a) { if ($b) { if (true) {} } }\n",
        "  $x = 1;\n",
        "  if ($c) { if ($d) { if (true) {} } }\n",
        "}\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "got: {}", bumps);
}

// ===========================================================================
// Additional argument tests
// ===========================================================================

#[test]
fn args_five_at_threshold_not_flagged() {
    let out = check("<?php\nfunction f(int $a, int $b, int $c, int $d, int $e): int { return $a + $b + $c + $d + $e; }\n");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn args_six_over_threshold_flagged() {
    let out = check("<?php\nfunction f(int $a, int $b, int $c, int $d, int $e, int $g): int { return $a; }\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {}", out);
}

// ===========================================================================
// Additional LOC tests
// ===========================================================================

#[test]
fn loc_empty_lines_excluded_module() {
    let out = debug("<?php\n\n\n\nfunction f(): int { return 1; }\n\n\n");
    assert!(out.contains("LOC, 1 function"));
}

// ===========================================================================
// Fuzzy duplication
// ===========================================================================

#[test]
fn fuzzy_duplication_detected() {
    let out = check(concat!(
        "<?php\nfunction process_alpha(array $data): int {\n",
        "  $result = 0;\n",
        "  foreach ($data as $item) {\n",
        "    if ($item > 100) { $result += 2; } else { $result += 1; }\n",
        "  }\n  return $result;\n",
        "}\n",
        "function process_beta(array $items): int {\n",
        "  $count = 0;\n",
        "  foreach ($items as $val) {\n",
        "    if ($val > 100) { $count += 2; } else { $count += 1; }\n",
        "  }\n  return $count;\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

// ===========================================================================
// Additional PHP-specific tests
// ===========================================================================

#[test]
fn try_catch_finally_cc() {
    let out = debug(concat!(
        "<?php\nfunction f(): int {\n",
        "  try {\n    return 1;\n",
        "  } catch (RuntimeException $e) {\n    return -1;\n",
        "  } catch (LogicException $e) {\n    return -2;\n",
        "  } finally {\n    echo 'done';\n  }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "two catches should add CC, got: {}", cc);
}

#[test]
fn nested_classes_found() {
    let out = debug(concat!(
        "<?php\nclass Outer {\n",
        "  public function outerMethod(): int { return 1; }\n",
        "}\n",
        "class Inner {\n",
        "  public function innerMethod(): int { return 2; }\n",
        "}\n",
    ));
    assert!(out.contains("Outer.outerMethod"), "got: {}", out);
    assert!(out.contains("Inner.innerMethod"), "got: {}", out);
}

#[test]
fn method_attributed_to_class() {
    let debug = debug(concat!(
        "<?php\nclass Svc {\n",
        "  public function handle(int $a, int $b, int $c, int $d, int $e, int $f, int $g, int $h): int {\n",
        "    return $a + $b;\n",
        "  }\n",
        "}\n",
    ));
    assert!(debug.contains("Svc.handle"), "method should be attributed to class, got: {}", debug);
}
