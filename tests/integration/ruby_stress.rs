use crate::common::*;
use std::process::Command;

lang_helpers!("rb");

// ===========================================================================
// CC counting (20)
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("def f(x)\n  if x > 0\n  end\n  x\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_elsif() {
    let out = debug("def f(x)\n  if x > 0\n  elsif x < 0\n  end\n  x\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "elsif should add CC, got: {cc}");
}

#[test]
fn cc_counts_unless() {
    let out = debug("def f(x)\n  unless x > 0\n    return -1\n  end\n  x\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("def f(x)\n  n = x\n  while n > 0\n    n -= 1\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_until() {
    let out = debug("def f(x)\n  n = x\n  until n <= 0\n    n -= 1\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_for() {
    let out = debug("def f(items)\n  for item in items\n    puts item\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_case_when() {
    let out = debug(concat!(
        "def f(x)\n",
        "  case x\n",
        "  when 1 then \"a\"\n",
        "  when 2 then \"b\"\n",
        "  when 3 then \"c\"\n",
        "  end\n",
        "end\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_rescue() {
    let out = debug("def f(x)\n  begin\n    Integer(x)\n  rescue\n    -1\n  end\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "rescue should add CC, got: {cc}");
}

#[test]
fn cc_counts_ternary() {
    let out = debug("def f(a)\n  a > 0 ? 1 : 0\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and_keyword() {
    let out = debug("def f(a, b)\n  if a and b\n    true\n  end\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_counts_or_keyword() {
    let out = debug("def f(a, b)\n  if a or b\n    true\n  end\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_counts_ampersand_ampersand() {
    let out = debug("def f(a, b)\n  if a && b\n    true\n  end\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_counts_pipe_pipe() {
    let out = debug("def f(a, b)\n  if a || b\n    true\n  end\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_chained_boolean() {
    let out = debug("def f(a, b, c)\n  if a && b || c\n    true\n  end\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

#[test]
fn cc_nested_if_counted_once() {
    let out = debug("def f(a, b)\n  if a\n    if b\n      true\n    end\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_case_else_not_counted() {
    let out =
        debug(concat!("def f(x)\n", "  case x\n", "  when 1 then \"a\"\n", "  else \"?\"\n", "  end\n", "end\n",));
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_base_case_is_1() {
    let out = debug("def f\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn cc_if_with_else() {
    let out = debug("def f(x)\n  if x > 0\n    1\n  else\n    0\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_multiple_if_accumulates() {
    let out = debug("def f(a, b)\n  if a > 0\n  end\n  if b > 0\n  end\n  0\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_postfix_if() {
    let out = debug("def f(x)\n  return 1 if x > 0\n  0\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// Cognitive complexity (10)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug("def f(a, b, c)\n  if a > 0\n  end\n  if b > 0\n  end\n  if c > 0\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(concat!(
        "def f(a, b, c, d)\n",
        "  if a > 0\n",
        "    if b > 0\n",
        "      if c > 0\n",
        "        if d > 0\n",
        "        end\n",
        "      end\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(10));
}

#[test]
fn cogc_elsif_no_extra_nesting() {
    let out =
        debug(concat!("def f(x)\n", "  if x == 1\n", "  elsif x == 2\n", "  elsif x == 3\n", "  end\n", "end\n",));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "elsif chain should have cogc >= 3, got: {cogc}");
}

#[test]
fn cogc_else_increases_nesting() {
    let out =
        debug(concat!("def f(x)\n", "  if x > 0\n", "  else\n", "    if x < -10\n", "    end\n", "  end\n", "end\n",));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "else should contribute to nesting, got: {cogc}");
}

#[test]
fn cogc_case_counted() {
    let out = debug(concat!("def f(x)\n", "  case x\n", "  when 1 then nil\n", "  end\n", "end\n",));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_loop_nested() {
    let out = debug("def f(items)\n  items.each do |i|\n    if i > 0\n    end\n  end\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 1, "each+if should contribute cogc, got: {cogc}");
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug("def f(a, b)\n  if a && b\n    true\n  end\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "got: {cogc}");
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug("def f(a, b, c)\n  if a && b || c\n    true\n  end\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "got: {cogc}");
}

#[test]
fn cogc_triggers_complex_method() {
    let mut code = String::from("def f(x)\n");
    for _ in 0..4 {
        code.push_str("  if x > 0\n    if x > 1\n      if x > 2\n      end\n    end\n  end\n");
    }
    code.push_str("  0\nend\n");
    let out = check(&code);
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check("def f(x)\n  if x > 0\n  end\n  if x > 1\n  end\n  0\nend\n");
    assert!(!has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Nesting depth (6)
// ===========================================================================

#[test]
fn nesting_depth_simple() {
    let out = debug("def f(x)\n  if x\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_nested() {
    let out = debug("def f(a, b, c)\n  if a\n    if b\n      if c\n      end\n    end\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_depth_sequential_not_accumulated() {
    let out = debug("def f(a, b)\n  if a\n  end\n  if b\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_deep_if_chain() {
    let out = debug(concat!(
        "def f(a, b, c, d, e)\n",
        "  if a\n",
        "    if b\n",
        "      if c\n",
        "        if d\n",
        "          if e\n",
        "          end\n",
        "        end\n",
        "      end\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

#[test]
fn nesting_loop_with_if() {
    let out = debug("def f(items)\n  items.each do |i|\n    if i > 0\n    end\n  end\nend\n");
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 1, "got: {nesting}");
}

#[test]
fn nesting_case_depth() {
    let out =
        debug(concat!("def f(x)\n", "  case x\n", "  when 1\n", "    if true\n", "    end\n", "  end\n", "end\n",));
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 2, "got: {nesting}");
}

// ===========================================================================
// Bump counting (2)
// ===========================================================================

#[test]
fn bumpy_road_two_bumps() {
    let out = debug(concat!(
        "def f(a, b, c, d)\n",
        "  if a\n    if b\n      if true\n      end\n    end\n  end\n",
        "  x = 1\n",
        "  if c\n    if d\n      if true\n      end\n    end\n  end\n",
        "end\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "got: {bumps}");
}

#[test]
fn bumpy_road_single_bump_not_flagged() {
    let out = check(concat!("def f(a, b)\n", "  if a\n    if b\n      if true\n      end\n    end\n  end\n", "end\n",));
    assert!(!has_smell(&out, "Nested Conditional Chunks"));
}

// ===========================================================================
// Arguments (6)
// ===========================================================================

#[test]
fn args_zero() {
    let out = debug("def f\nend\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_one() {
    let out = debug("def f(x)\n  x\nend\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(1));
}

#[test]
fn args_five_at_threshold() {
    let out = check("def f(a, b, c, d, e)\n  a + b + c + d + e\nend\n");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn args_six_over_threshold() {
    let out = check("def f(a, b, c, d, e, g)\n  a + b + c + d + e + g\nend\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

#[test]
fn args_default_params_counted() {
    let out = debug("def f(a, b = 1, c = 2)\n  a + b + c\nend\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_splat_params_counted() {
    let out = debug("def f(a, *args, **kwargs, &blk)\n  a\nend\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(4));
}

// ===========================================================================
// LOC counting (4)
// ===========================================================================

#[test]
fn loc_single_line_method() {
    let out = debug("def f\nend\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(2));
}

#[test]
fn loc_multiline() {
    let out = debug("def f\n  x = 1\n  x + 1\nend\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(4));
}

#[test]
fn loc_comments_excluded_module() {
    let out = debug("# a comment\n# another\ndef f\nend\n");
    assert!(out.contains("LOC, 1 function"));
}

#[test]
fn loc_empty_lines_excluded_module() {
    let out = debug("\n\n\ndef f\nend\n\n\n");
    assert!(out.contains("LOC, 1 function"));
}

// ===========================================================================
// Embedded blocks (2)
// ===========================================================================

#[test]
fn embedded_large_heredoc() {
    let mut code = String::from("def f()\n  <<~HEREDOC\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("    line {i}\n"));
    }
    code.push_str("  HEREDOC\nend\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

#[test]
fn embedded_small_string_not_flagged() {
    let out = check("def f()\n  \"hello\"\nend\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Duplication (4)
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = check(concat!(
        "def alpha(x)\n  r = x\n  r = r * 2\n  r = r + 1\n  r = r - 3\n  r\nend\n",
        "def beta(x)\n  r = x\n  r = r * 2\n  r = r + 1\n  r = r - 3\n  r\nend\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn exact_duplication_below_min_loc() {
    let out = check("def a(x)\n  x\nend\ndef b(x)\n  x\nend\n");
    assert!(!has_smell(&out, "Code Duplication"));
}

#[test]
fn fuzzy_duplication_detected() {
    let out = check(concat!(
        "def process_alpha(data)\n",
        "  result = 0\n",
        "  data.each do |item|\n",
        "    if item > 100\n      result += 2\n    else\n      result += 1\n    end\n",
        "  end\n  result\nend\n",
        "def process_beta(items)\n",
        "  count = 0\n",
        "  items.each do |val|\n",
        "    if val > 100\n      count += 2\n    else\n      count += 1\n    end\n",
        "  end\n  count\nend\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn test_function_duplication_suppressed() {
    let out = check(concat!(
        "def test_alpha\n  r = 0\n  r += 1\n  r += 2\n  r += 3\n  r += 4\n  r += 5\nend\n",
        "def test_beta\n  r = 0\n  r += 1\n  r += 2\n  r += 3\n  r += 4\n  r += 5\nend\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"), "test duplication should be suppressed, got: {out}");
}

// ===========================================================================
// Assertions (3)
// ===========================================================================

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("def f()\n");
    for _ in 0..asserts_above() {
        code.push_str("  assert(true)\n");
    }
    code.push_str("end\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"), "got: {out}");
}

#[test]
fn assertion_block_below_threshold() {
    let mut code = String::from("def f()\n");
    for _ in 0..5 {
        code.push_str("  assert(true)\n");
    }
    code.push_str("end\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_interrupted_resets() {
    let mut code = String::from("def f(x)\n");
    for _ in 0..5 {
        code.push_str("  assert(true)\n");
    }
    code.push_str("  y = x + 1\n");
    for _ in 0..5 {
        code.push_str("  assert(true)\n");
    }
    code.push_str("end\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Compound conditions (2)
// ===========================================================================

#[test]
fn compound_condition_detected() {
    let out = check(concat!(
        "def f(a, b, c)\n",
        "  if a && b || c\n",
        "    if a || b && c\n",
        "      if b && c || a\n",
        "        true\n",
        "      end\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));
    assert!(has_smell(&out, "Complex Conditional"), "got: {out}");
}

#[test]
fn compound_condition_simple_not_detected() {
    let out = check("def f(a, b)\n  if a && b\n    true\n  end\nend\n");
    assert!(!has_smell(&out, "Complex Conditional"));
}

// ===========================================================================
// Primitive obsession (3)
// ===========================================================================

#[test]
fn primitive_obsession_never_triggers() {
    let out = check("def f(a, b, c, d)\n  a + b + c + d\nend\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn no_typed_params() {
    let out = debug("def f(a, b, c, d)\n  a\nend\n");
    assert!(out.contains("primitives=0/0"), "got: {out}");
}

#[test]
fn typed_param_count_zero() {
    let out = debug("def f(a, b)\n  a + b\nend\n");
    assert!(out.contains("primitives=0/0"));
}

// ===========================================================================
// LCOM4 (3)
// ===========================================================================

#[test]
fn lcom4_connected_no_smell() {
    let out =
        check(concat!("class S\n", "  def get_x\n    @x\n  end\n", "  def set_x(v)\n    @x = v\n  end\n", "end\n",));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_disconnected() {
    let out = check(concat!(
        "class S\n",
        "  def get_a\n    @a\n  end\n",
        "  def get_b\n    @b\n  end\n",
        "  def get_c\n    @c\n  end\n",
        "  def get_d\n    @d\n  end\n",
        "  def set_a(v)\n    @a = v\n  end\n",
        "  def set_b(v)\n    @b = v\n  end\n",
        "  def set_c(v)\n    @c = v\n  end\n",
        "  def set_d(v)\n    @d = v\n  end\n",
        "end\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_single_method_no_smell() {
    let out = check("class S\n  def get_x\n    @x\n  end\nend\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "class Coord\n",
        "  def process(e)\n    self.validate(e) && self.dispatch(e)\n  end\n",
        "  def validate(e)\n    e.positive?\n  end\n",
        "  def dispatch(e)\n    self.send_event(e)\n  end\n",
        "  def send_event(e)\n    true\n  end\n",
        "end\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "class Mixed\n",
        "  def a\n    @x\n  end\n",
        "  def b\n    @x = 1; self.c\n  end\n",
        "  def c\n    42\n  end\n",
        "end\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_class_still_fires() {
    let out = check(concat!(
        "class Svc\n",
        "  def get_user\n    @db\n  end\n",
        "  def cache_user\n    @cache\n  end\n",
        "  def send_welcome\n    @mailer\n  end\n",
        "  def publish\n    @events\n  end\n",
        "  def audit_log\n    @audit\n  end\n",
        "end\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "class Svc\n",
        "  def a\n    @db\n  end\n",
        "  def b\n    @cache\n  end\n",
        "  def c\n    @log\n  end\n",
        "end\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Method naming (3)
// ===========================================================================

#[test]
fn function_has_no_prefix() {
    let out = debug("def standalone\nend\n");
    assert!(out.contains("standalone"), "got: {out}");
    assert!(!out.contains(".standalone"));
}

#[test]
fn method_has_class_prefix() {
    let out = debug("class Svc\n  def handle\n  end\nend\n");
    assert!(out.contains("Svc.handle"), "got: {out}");
}

#[test]
fn initialize_is_constructor() {
    let out =
        debug(concat!("class Svc\n", "  def initialize(a, b, c, d, e, f)\n", "    @a = a\n", "  end\n", "end\n",));
    assert!(out.contains("Svc.initialize"), "got: {out}");
}

// ===========================================================================
// Ruby-specific (15)
// ===========================================================================

#[test]
fn unless_increments_cc() {
    let out = debug("def f(x)\n  unless x\n    return -1\n  end\n  x\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn until_increments_cc() {
    let out = debug("def f(x)\n  n = x\n  until n <= 0\n    n -= 1\n  end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn case_when_each_increments_cc() {
    let out = debug(concat!(
        "def f(x)\n",
        "  case x\n",
        "  when 1 then \"a\"\n",
        "  when 2 then \"b\"\n",
        "  when 3 then \"c\"\n",
        "  else \"?\"\n",
        "  end\n",
        "end\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn rescue_cc() {
    let out = debug("def f(x)\n  begin\n    Integer(x)\n  rescue\n    -1\n  end\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "rescue should add CC, got: {cc}");
}

#[test]
fn begin_rescue_ensure() {
    let out = debug(concat!(
        "def f(x)\n",
        "  begin\n",
        "    Integer(x)\n",
        "  rescue ArgumentError\n",
        "    -1\n",
        "  rescue TypeError\n",
        "    -2\n",
        "  ensure\n",
        "    puts \"done\"\n",
        "  end\n",
        "end\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "two rescues should add CC, got: {cc}");
}

#[test]
fn empty_rescue_detected() {
    let out = check(concat!("def f(x)\n", "  begin\n", "    Integer(x)\n", "  rescue\n", "  end\n", "end\n",));
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn postfix_if_increments_cc() {
    let out = debug("def f(x)\n  return 1 if x > 0\n  0\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn postfix_unless_increments_cc() {
    let out = debug("def f(x)\n  return -1 unless x > 0\n  x\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn do_block_complexity_counts() {
    let out = debug(concat!(
        "def f(items)\n",
        "  items.each do |i|\n",
        "    if i > 0\n",
        "      puts i\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "block if should contribute to CC, got: {cc}");
}

#[test]
fn curly_block_complexity_counts() {
    let out = debug("def f(items)\n  items.select { |i| i > 0 ? i : nil }\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "curly block ternary should contribute, got: {cc}");
}

#[test]
fn module_methods_analyzed() {
    let out = debug(concat!("module Helper\n", "  def compute(x)\n", "    x + 1\n", "  end\n", "end\n",));
    assert!(out.contains("Helper.compute"), "got: {out}");
}

#[test]
fn instance_variable_field_access() {
    let out = debug(concat!(
        "class Svc\n",
        "  def get_db\n    @db\n  end\n",
        "  def get_cache\n    @cache\n  end\n",
        "end\n",
    ));
    assert!(out.contains("fields=[\"db\"]") || out.contains("fields=[\"cache\"]"), "got: {out}");
}

#[test]
fn each_with_block_loop() {
    let out = debug(concat!(
        "def f(items)\n",
        "  items.each do |item|\n",
        "    if item > 0\n",
        "      if item > 10\n",
        "        puts item\n",
        "      end\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 2, "got: {nesting}");
}

#[test]
fn heredoc_embedded_block() {
    let mut code = String::from("def f()\n  <<~HEREDOC\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("    line {i}\n"));
    }
    code.push_str("  HEREDOC\nend\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

// ===========================================================================
// Performance (2)
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.rb");
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("def func{i}(x)\n"));
        for j in 0..18 {
            code.push_str(&format!("  v{j} = {j}\n"));
        }
        code.push_str("  x\nend\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(start.elapsed().as_millis() < 200, "took: {}ms", start.elapsed().as_millis());
}

#[test]
fn performance_class_hierarchy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("classes.rb");
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class S{i}\n"));
        for j in 0..5 {
            code.push_str(&format!("  def m{j}()\n    @x{i} + {j}\n  end\n"));
        }
        code.push_str("end\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(start.elapsed().as_millis() < 500, "took: {}ms", start.elapsed().as_millis());
}

// ===========================================================================
// Edge cases (5)
// ===========================================================================

#[test]
fn clean_module_not_flagged() {
    let out =
        check(concat!("class Point\n", "  def add()\n    @x + @y\n  end\n", "end\n", "def helper(x)\n  x + 1\nend\n",));
    assert!(out.is_empty(), "got: {out}");
}

#[test]
fn comments_only_no_output() {
    let out = check("# this is a comment\n# another comment\n");
    assert!(out.is_empty());
}

#[test]
fn empty_file_no_crash() {
    let out = check("");
    assert!(out.is_empty());
}

#[test]
fn empty_method_body() {
    let out = debug("def f\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn multiple_functions_independent_metrics() {
    let out = debug("def simple\nend\ndef complex(x)\n  if x\n  end\nend\n");
    assert_eq!(function_metric(&out, "simple", "cc"), Some(1));
    assert_eq!(function_metric(&out, "complex", "cc"), Some(2));
}
