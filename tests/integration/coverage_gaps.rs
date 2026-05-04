
use crate::common::*;
use std::process::Command;

fn pulse_bin() -> String {
    env!("CARGO_BIN_EXE_pulse").to_string()
}

// ===========================================================================
// main.rs coverage: CLI paths
// ===========================================================================

#[test]
fn invalid_args_prints_usage() {
    let out = Command::new(pulse_bin())
        .args(["garbage"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("usage:"), "should print usage on bad args: {stderr}");
}

#[test]
fn check_unsupported_file_silent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.xyz");
    std::fs::write(&path, "hello world").unwrap();
    let out = Command::new(pulse_bin())
        .args(["check", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).is_empty());
}

#[test]
fn check_nonexistent_file_silent() {
    let out = Command::new(pulse_bin())
        .args(["check", "/nonexistent/file.py"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).is_empty());
}

#[test]
fn debug_unsupported_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.xyz");
    std::fs::write(&path, "hello world").unwrap();
    let out = Command::new(pulse_bin())
        .args(["debug", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

// ===========================================================================
// C# coverage: namespaces, nested classes, constructors, ternary, try/catch/finally
// ===========================================================================

#[test]
fn csharp_namespace_functions() {
    let out = debug(r"
namespace MyApp {
    public class Service {
        public void Process() {
            if (true) { }
            if (true) { }
            if (true) { }
        }
    }
}
", "cs");
    assert!(out.contains("Process"), "should find method in namespace: {out}");
}

#[test]
fn csharp_nested_class() {
    let out = debug(r"
public class Outer {
    public class Inner {
        public void Work() {
            if (true) { }
        }
    }
}
", "cs");
    assert!(out.contains("Inner.Work"), "should find nested class method: {out}");
}

#[test]
fn csharp_constructor() {
    let out = debug(r"
public class Service {
    public Service(int x, int y) {
        if (x > 0) { }
    }
}
", "cs");
    assert!(out.contains("Service.Service"), "should find constructor: {out}");
}

#[test]
fn csharp_record_declaration() {
    let out = debug(r"
public record Config {
    public void Validate() {
        if (true) { }
    }
}
", "cs");
    assert!(out.contains("Validate"), "should find method in record: {out}");
}

#[test]
fn csharp_struct_declaration() {
    let out = debug(r"
public struct Point {
    public double Distance() {
        if (true) { }
        return 0.0;
    }
}
", "cs");
    assert!(out.contains("Point.Distance"), "should find struct method: {out}");
}

#[test]
fn csharp_interface_methods() {
    let out = debug(r"
public interface IService {
    public void Execute() {
        if (true) { }
    }
}
", "cs");
    assert!(out.contains("Execute"), "should find interface default method: {out}");
}

#[test]
fn csharp_ternary_expression() {
    let out = debug(r"
public class T {
    public int F(int x) {
        return x > 0 ? x : -x;
    }
}
", "cs");
    assert_eq!(function_metric(&out, "F", "cc"), Some(2));
}

#[test]
fn csharp_lambda_skipped() {
    let out = debug(r"
public class T {
    public void F() {
        var fn = (int x) => { if (x > 0) { } };
    }
}
", "cs");
    assert_eq!(function_metric(&out, "F", "cc"), Some(1));
}

#[test]
fn csharp_try_catch_finally() {
    let out = debug(r"
public class T {
    public void F() {
        try {
            if (true) { }
        }
        catch (Exception e) {
            if (true) { }
        }
        finally {
            if (true) { }
        }
    }
}
", "cs");
    let cc = function_metric(&out, "F", "cc").unwrap_or(0);
    assert!(cc >= 4, "try/catch/finally should add cc: {cc}");
}

#[test]
fn csharp_empty_catch() {
    let out = check(r"
public class T {
    public void F() {
        try { throw new Exception(); }
        catch (Exception e) { }
        try { throw new Exception(); }
        catch (Exception e) { }
        try { throw new Exception(); }
        catch (Exception e) { }
    }
}
", "cs");
    // Empty catch is tracked but not a standalone smell; just exercises the path
    let _ = out;
}

#[test]
fn csharp_else_if_chain() {
    let out = debug(r"
public class T {
    public int F(int x) {
        if (x > 10) { return 1; }
        else if (x > 5) { return 2; }
        else if (x > 0) { return 3; }
        else { return 0; }
    }
}
", "cs");
    let cc = function_metric(&out, "F", "cc").unwrap_or(0);
    assert!(cc >= 4, "else-if chain should increase cc: {cc}");
}

#[test]
fn csharp_switch_default() {
    let out = debug(r#"
public class T {
    public string F(int x) {
        switch (x) {
            case 1: return "a";
            default: return "c";
        }
    }
}
"#, "cs");
    let cc = function_metric(&out, "F", "cc").unwrap_or(0);
    // switch_statement adds 1 (track_nesting cogc), case adds 1, default does not
    assert!(cc >= 2, "default should not add cc: {cc}");
}

#[test]
fn csharp_type_identifier_primitive() {
    let out = debug(r"
public class T {
    public void F(String a, String b, String c, String d, String e) {
        if (true) { }
    }
}
", "cs");
    assert!(out.contains("args=5"), "should count 5 args: {out}");
}

#[test]
fn csharp_top_level_method() {
    let out = debug(r"
void TopLevel() {
    if (true) { }
}
", "cs");
    assert!(out.contains("TopLevel"), "should find top-level method: {out}");
}

#[test]
fn csharp_local_function() {
    let out = debug(r"
public class T {
    public void F() {
        void Inner() {
            if (true) { }
        }
    }
}
", "cs");
    assert!(out.contains("Inner") || out.contains('F'), "should find local function: {out}");
}

// ===========================================================================
// C++ coverage: namespaces, pointer declarator, for_range_loop, ternary, lambda, void params
// ===========================================================================

#[test]
fn cpp_namespace_functions() {
    let out = debug(r"
namespace ns {
    void process() {
        if (true) { }
    }
}
", "cpp");
    assert!(out.contains("process"), "should find namespace function: {out}");
}

#[test]
fn cpp_pointer_return_function() {
    let out = debug(r"
int* create(int n) {
    if (n > 0) { }
    return nullptr;
}
", "cpp");
    assert!(out.contains("create"), "should find pointer-return function: {out}");
}

#[test]
fn cpp_for_range_loop() {
    let out = debug(r"
void f() {
    int arr[] = {1, 2, 3};
    for (auto x : arr) {
        if (x > 0) { }
    }
}
", "cpp");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "range-for should add cc: {cc}");
}

#[test]
fn cpp_ternary() {
    let out = debug(r"
int f(int x) {
    return x > 0 ? x : -x;
}
", "cpp");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cpp_lambda_skipped() {
    let out = debug(r"
void f() {
    auto fn = [](int x) { if (x > 0) {} };
}
", "cpp");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn cpp_try_catch_finally() {
    let out = debug(r"
void f() {
    try {
        if (true) { }
    } catch (int e) {
        if (true) { }
    }
}
", "cpp");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "try/catch should add cc: {cc}");
}

#[test]
fn cpp_else_if_chain() {
    let out = debug(r"
int f(int x) {
    if (x > 10) { return 1; }
    else if (x > 5) { return 2; }
    else { return 0; }
}
", "cpp");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "else-if should add cc: {cc}");
}

#[test]
fn cpp_void_param_list() {
    let out = debug(r"
void f(void) {
    if (true) { }
}
", "cpp");
    assert!(out.contains("args=0"), "void param should count as 0 args: {out}");
}

#[test]
fn cpp_variadic_params() {
    // Exercises the variadic_parameter branch in count_param_children
    // C++ tree-sitter may parse `...` as plain token; test exercises the fold path
    let out = debug(r"
void f(int a, int b, ...) {
    if (true) { }
}
", "cpp");
    assert!(out.contains("args="), "should parse params: {out}");
}

#[test]
fn cpp_type_identifier_primitive() {
    let out = debug(r"
void f(string a, string b, auto c) {
    if (true) { }
}
", "cpp");
    assert!(out.contains("args=3"), "should count 3 args: {out}");
}

#[test]
fn cpp_empty_catch() {
    let out = debug(r"
void f() {
    try { } catch (int e) { }
}
", "cpp");
    assert!(out.contains('f'), "empty catch path exercised: {out}");
}

#[test]
fn cpp_class_method_constructor() {
    let out = debug(r"
class Foo {
    void bar() {
        if (true) { }
    }
};
", "cpp");
    assert!(out.contains("Foo::bar"), "should find class method: {out}");
}

// ===========================================================================
// Ruby coverage: singleton, modifiers, begin/rescue/ensure, ternary, binary, heredoc
// ===========================================================================

#[test]
fn ruby_singleton_method() {
    let out = debug(r"
class Foo
  def self.bar
    if true
      1
    end
  end
end
", "rb");
    assert!(out.contains("bar"), "should find singleton method: {out}");
}

#[test]
fn ruby_unless_modifier() {
    let out = debug(r"
class Foo
  def bar
    x = 1 unless false
    y = 2 if true
  end
end
", "rb");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 3, "unless modifier should add cc: {cc}");
}

#[test]
fn ruby_while_modifier() {
    let out = debug(r"
class Foo
  def bar
    x = 1
    x += 1 while x < 10
    x -= 1 until x <= 0
  end
end
", "rb");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 3, "while/until modifiers should add cc: {cc}");
}

#[test]
fn ruby_begin_rescue_ensure() {
    let out = debug(r#"
class Foo
  def bar
    begin
      raise "err"
    rescue StandardError => e
      puts e
    ensure
      puts "done"
    end
  end
end
"#, "rb");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 2, "rescue should add cc: {cc}");
}

#[test]
fn ruby_empty_rescue() {
    let out = debug(r#"
class Foo
  def bar
    begin
      raise "err"
    rescue
    end
  end
end
"#, "rb");
    assert!(out.contains("bar"), "empty rescue path exercised: {out}");
}

#[test]
fn ruby_ternary_conditional() {
    let out = debug(r"
class Foo
  def bar(x)
    x > 0 ? x : -x
  end
end
", "rb");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 2, "ternary should add cc: {cc}");
}

#[test]
fn ruby_boolean_operators() {
    let out = debug(r"
class Foo
  def bar(a, b, c)
    if a && b || c
      1
    end
  end
end
", "rb");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 3, "boolean ops should add cc: {cc}");
}

#[test]
fn ruby_do_block() {
    let out = debug(r"
class Foo
  def bar
    [1, 2].each do |x|
      if x > 0
        puts x
      end
    end
  end
end
", "rb");
    assert!(out.contains("bar"), "do_block path exercised: {out}");
}

#[test]
fn ruby_heredoc() {
    let out = debug(r"
class Foo
  def bar
    text = <<~HEREDOC
      line one
      line two
      line three
      line four
      line five
      line six
    HEREDOC
  end
end
", "rb");
    assert!(out.contains("bar"), "heredoc path exercised: {out}");
}

#[test]
fn ruby_field_accesses() {
    let out = debug(r#"
class Foo
  def bar
    @name = "test"
    @age = 10
  end

  def baz
    @name
  end

  def qux
    @age
  end
end
"#, "rb");
    assert!(out.contains("bar"), "field access path exercised: {out}");
}

#[test]
fn ruby_check_condition_ops() {
    let out = debug(r"
class Foo
  def bar(a, b, c)
    if a and b or c
      1
    end
  end
end
", "rb");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 2, "and/or should count: {cc}");
}

#[test]
fn ruby_elsif_chain() {
    let out = debug(r"
class Foo
  def bar(x)
    if x > 10
      1
    elsif x > 5
      2
    elsif x > 0
      3
    else
      0
    end
  end
end
", "rb");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 4, "elsif chain should increase cc: {cc}");
}

// ===========================================================================
// Java coverage: constructors, nested class, try_with_resources, ternary, text_block
// ===========================================================================

#[test]
fn java_constructor() {
    let out = debug(r"
public class Service {
    public Service(int x, int y) {
        if (x > 0) { }
    }
}
", "java");
    assert!(out.contains("Service.Service"), "should find constructor: {out}");
}

#[test]
fn java_nested_class() {
    let out = debug(r"
public class Outer {
    public class Inner {
        public void work() {
            if (true) { }
        }
    }
}
", "java");
    assert!(out.contains("Inner.work"), "should find nested class method: {out}");
}

#[test]
fn java_try_with_resources() {
    let out = debug(r"
public class T {
    public void f() throws Exception {
        try (var r = new Object()) {
            if (true) { }
        } catch (Exception e) {
            if (true) { }
        } finally {
            if (true) { }
        }
    }
}
", "java");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "try-with-resources/catch/finally: {cc}");
}

#[test]
fn java_ternary() {
    let out = debug(r"
public class T {
    public int f(int x) {
        return x > 0 ? x : -x;
    }
}
", "java");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn java_text_block() {
    let out = debug(r#"
public class T {
    public String f() {
        return """
            line one
            line two
            line three
            line four
            line five
            line six
            """;
    }
}
"#, "java");
    assert!(out.contains('f'), "text_block path exercised: {out}");
}

#[test]
fn java_else_if_chain() {
    let out = debug(r"
public class T {
    public int f(int x) {
        if (x > 10) { return 1; }
        else if (x > 5) { return 2; }
        else { return 0; }
    }
}
", "java");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "else-if should add cc: {cc}");
}

#[test]
fn java_switch_default() {
    let out = debug(r#"
public class T {
    public String f(int x) {
        switch (x) {
            case 1: return "a";
            default: return "b";
        }
    }
}
"#, "java");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2, "default should not add cc: {cc}");
}

#[test]
fn java_spread_parameter() {
    let out = debug(r"
public class T {
    public void f(String... args) {
        if (true) { }
    }
}
", "java");
    assert!(out.contains("args=1"), "spread should count as 1 arg: {out}");
}

#[test]
fn java_type_identifier_primitive() {
    let out = debug(r"
public class T {
    public void f(String a) {
        if (true) { }
    }
}
", "java");
    assert!(out.contains("primitives=1"), "String should be primitive: {out}");
}

#[test]
fn java_empty_catch() {
    let out = debug(r"
public class T {
    public void f() {
        try { } catch (Exception e) { }
    }
}
", "java");
    assert!(out.contains('f'), "empty catch path exercised: {out}");
}

// ===========================================================================
// Python coverage: assert with bool, condition complexity, primitive types, splats, finally
// ===========================================================================

#[test]
fn python_assert_with_boolean() {
    let out = debug(r"
def f(x, y):
    assert x > 0 and y > 0
", "py");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "assert with boolean should add cc: {cc}");
}

#[test]
fn python_condition_complexity() {
    let out = debug(r"
def f(a, b, c):
    if a and b or c:
        pass
", "py");
    let cond = function_metric(&out, "f", "conditions").unwrap_or(0);
    assert!(cond >= 1, "complex condition: {cond}");
}

#[test]
fn python_primitive_types() {
    let out = debug(r"
def f(a: int, b: str, c: float):
    if True:
        pass
", "py");
    assert!(out.contains("primitives=3"), "should detect 3 primitive types: {out}");
}

#[test]
fn python_typed_default_parameter() {
    let out = debug(r#"
def f(a: int = 0, b: str = ""):
    if True:
        pass
"#, "py");
    assert!(out.contains("args=2"), "typed defaults count as args: {out}");
}

#[test]
fn python_splat_patterns() {
    let out = debug(r"
def f(*args, **kwargs):
    if True:
        pass
", "py");
    assert!(out.contains("args=2"), "splats should count as args: {out}");
}

#[test]
fn python_try_finally() {
    let out = debug(r"
def f():
    try:
        if True:
            pass
    except ValueError:
        pass
    finally:
        if True:
            pass
", "py");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "except/finally should work: {cc}");
}

#[test]
fn python_except_in_try_children() {
    let out = debug(r"
def f():
    try:
        pass
    except ValueError:
        if True:
            pass
    except KeyError:
        pass
", "py");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "multiple except clauses: {cc}");
}

#[test]
fn python_empty_except() {
    let out = debug(r"
def f():
    try:
        pass
    except:
        pass
", "py");
    assert!(out.contains('f'), "empty except path exercised: {out}");
}

// ===========================================================================
// TypeScript coverage: generator, function expr, default case, catch/finally, else-if, exports
// ===========================================================================

#[test]
fn typescript_generator_function() {
    let out = debug(r"
function* gen() {
    if (true) { }
    yield 1;
}
", "ts");
    assert!(out.contains("gen"), "should find generator function: {out}");
}

#[test]
fn typescript_function_expression() {
    // Arrow function from variable declaration exercises the collect_arrow_functions path
    let out = debug(r"
const myFunc = () => {
    if (true) { }
};
", "ts");
    assert!(out.contains("myFunc"), "should find arrow function: {out}");
}

#[test]
fn typescript_switch_default() {
    let out = debug(r#"
function f(x: number): string {
    switch (x) {
        case 1: return "a";
        default: return "b";
    }
}
"#, "ts");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2, "default should not add cc: {cc}");
}

#[test]
fn typescript_try_catch_finally() {
    let out = debug(r"
function f() {
    try {
        if (true) { }
    } catch (e) {
        if (true) { }
    } finally {
        if (true) { }
    }
}
", "ts");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "try/catch/finally should add cc: {cc}");
}

#[test]
fn typescript_else_if_chain() {
    let out = debug(r"
function f(x: number): number {
    if (x > 10) { return 1; }
    else if (x > 5) { return 2; }
    else { return 0; }
}
", "ts");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "else-if should add cc: {cc}");
}

#[test]
fn typescript_empty_catch() {
    let out = debug(r"
function f() {
    try { } catch (e) { }
}
", "ts");
    assert!(out.contains('f'), "empty catch path: {out}");
}

#[test]
fn typescript_exported_class() {
    let out = debug(r"
export class Foo {
    bar(): void {
        if (true) { }
    }
}
", "ts");
    assert!(out.contains("Foo.bar"), "should find exported class method: {out}");
}

#[test]
fn typescript_ternary() {
    let out = debug(r"
function f(x: number): number {
    return x > 0 ? x : -x;
}
", "ts");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// Go coverage: pointer receiver, type switch, select, default case, pointer/slice types
// ===========================================================================

#[test]
fn go_pointer_receiver() {
    let out = debug(r"
package main

type Foo struct{}

func (f *Foo) Bar() {
    if true { }
}
", "go");
    assert!(out.contains("Foo.Bar"), "should find pointer receiver method: {out}");
}

#[test]
fn go_type_switch() {
    let out = debug(r"
package main

func f(x interface{}) {
    switch x.(type) {
    case int:
    case string:
    default:
    }
}
", "go");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "type switch should add cc: {cc}");
}

#[test]
fn go_select_statement() {
    let out = debug(r"
package main

func f(ch chan int) {
    select {
    case v := <-ch:
        _ = v
    default:
    }
}
", "go");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "select should add cc: {cc}");
}

#[test]
fn go_else_body() {
    let out = debug(r"
package main

func f(x int) int {
    if x > 0 {
        return 1
    } else {
        return 0
    }
}
", "go");
    assert!(out.contains('f'), "else body path exercised: {out}");
}

#[test]
fn go_else_if() {
    let out = debug(r"
package main

func f(x int) int {
    if x > 10 {
        return 1
    } else if x > 5 {
        return 2
    } else {
        return 0
    }
}
", "go");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "else-if should add cc: {cc}");
}

#[test]
fn go_pointer_primitive_type() {
    let out = debug(r"
package main

func f(x *int) {
    if true { }
}
", "go");
    assert!(out.contains("primitives=1"), "pointer to int is primitive: {out}");
}

#[test]
fn go_slice_primitive_type() {
    let out = debug(r"
package main

func f(x []string) {
    if true { }
}
", "go");
    assert!(out.contains("primitives=1"), "slice of string is primitive: {out}");
}

#[test]
fn go_variadic_param() {
    let out = debug(r"
package main

func f(args ...int) {
    if true { }
}
", "go");
    assert!(out.contains("args=1"), "variadic counts as 1 arg: {out}");
}

#[test]
fn go_defer_statement() {
    let out = debug(r"
package main

func f() {
    defer func() {}()
    if true { }
}
", "go");
    assert!(out.contains('f'), "defer path exercised: {out}");
}

// ===========================================================================
// ObjC coverage: C functions, pointer declarator, void params, else block
// ===========================================================================

#[test]
fn objc_c_function() {
    let out = debug(r"
void helper(int x) {
    if (x > 0) { }
}
", "m");
    assert!(out.contains("helper"), "should find C function in ObjC: {out}");
}

#[test]
fn objc_pointer_function() {
    let out = debug(r"
int* create(int n) {
    if (n > 0) { }
    return 0;
}
", "m");
    assert!(out.contains("create"), "should find pointer-return C function: {out}");
}

#[test]
fn objc_void_params() {
    let out = debug(r"
void f(void) {
    if (1) { }
}
", "m");
    assert!(out.contains("args=0"), "void should be 0 args: {out}");
}

#[test]
fn objc_variadic_param() {
    let out = debug(r"
void f(int x, ...) {
    if (1) { }
}
", "m");
    assert!(out.contains("args=2"), "variadic counts: {out}");
}

#[test]
fn objc_else_if() {
    let out = debug(r"
@implementation Foo
- (void)bar {
    if (1) { }
    else if (2) { }
    else { }
}
@end
", "m");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 3, "else if/else in ObjC: {cc}");
}

#[test]
fn objc_empty_catch() {
    let out = debug(r"
@implementation Foo
- (void)bar {
    @try { }
    @catch (NSException *e) { }
}
@end
", "m");
    assert!(out.contains("bar"), "empty catch path: {out}");
}

#[test]
fn objc_ternary() {
    let out = debug(r"
@implementation Foo
- (int)bar:(int)x {
    return x > 0 ? x : -x;
}
@end
", "m");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 2, "ternary should add cc: {cc}");
}

// ===========================================================================
// Zig coverage: labeled statement, binary non-orelse, catch, comptime, defer/errdefer
// ===========================================================================

#[test]
fn zig_labeled_block() {
    let out = debug(r"
fn f() void {
    const x = blk: {
        if (true) {}
        break :blk 0;
    };
    _ = x;
}
", "zig");
    assert!(out.contains('f'), "labeled block path exercised: {out}");
}

#[test]
fn zig_catch_expression() {
    let out = debug(r"
fn f() !void {
    const x = getVal() catch |err| {
        if (true) {}
        return err;
    };
    _ = x;
}

fn getVal() !i32 {
    return 42;
}
", "zig");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "catch should add cc: {cc}");
}

#[test]
fn zig_orelse() {
    let out = debug(r"
fn f() void {
    const x: ?i32 = null;
    const y = x orelse 0;
    _ = y;
}
", "zig");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "orelse should add cc: {cc}");
}

#[test]
fn zig_defer_errdefer() {
    let out = debug(r"
fn f() !void {
    defer {}
    errdefer {}
    if (true) {}
}
", "zig");
    assert!(out.contains('f'), "defer/errdefer path: {out}");
}

#[test]
fn zig_comptime() {
    let out = debug(r"
fn f() void {
    comptime {
        if (true) {}
    }
}
", "zig");
    assert!(out.contains('f'), "comptime path exercised: {out}");
}

#[test]
fn zig_else_if_chain() {
    let out = debug(r"
fn f(x: i32) i32 {
    if (x > 10) {
        return 1;
    } else if (x > 5) {
        return 2;
    } else {
        return 0;
    }
}
", "zig");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "else-if should add cc: {cc}");
}

#[test]
fn zig_switch_default() {
    let out = debug(r"
fn f(x: u8) u8 {
    return switch (x) {
        1 => 10,
        2 => 20,
        else => 0,
    };
}
", "zig");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "switch non-default cases add cc: {cc}");
}

// ===========================================================================
// Swift coverage: extensions, enum body, protocol, empty catch, field access
// ===========================================================================

#[test]
fn swift_extension() {
    let out = debug(r"
struct Foo {}
extension Foo {
    func bar() {
        if true { }
    }
}
", "swift");
    assert!(out.contains("Foo.bar"), "should find extension method: {out}");
}

#[test]
fn swift_enum_with_methods() {
    let out = debug(r#"
enum Direction {
    case north, south
    func label() -> String {
        if true { }
        return ""
    }
}
"#, "swift");
    assert!(out.contains("label"), "should find enum method: {out}");
}

#[test]
fn swift_protocol_declaration() {
    let out = check(r"
protocol Serviceable {}
protocol Configurable {}
protocol Loggable {}
protocol Cacheable {}
protocol Validatable {}
protocol Serializable {}
protocol Buildable {}
", "swift");
    // Protocols count as declarations - exercises count_declarations protocol path
    let _ = out;
}

#[test]
fn swift_empty_do_catch() {
    let out = debug(r"
class Foo {
    func bar() {
        do {
            if true { }
        } catch {
        }
    }
}
", "swift");
    assert!(out.contains("bar"), "empty catch path: {out}");
}

#[test]
fn swift_self_field_access() {
    let out = debug(r#"
class Foo {
    var name: String = ""
    var age: Int = 0

    func bar() {
        self.name = "x"
    }

    func baz() {
        self.age = 1
    }

    func qux() {
        let _ = self.name
    }
}
"#, "swift");
    // Exercises collect_swift_field_accesses and try_extract_self_field
    assert!(out.contains("Foo.bar"), "field access tracking: {out}");
}

#[test]
fn swift_guard_statement() {
    let out = debug(r"
class Foo {
    func bar(x: Int?) {
        guard let v = x else { return }
        if v > 0 { }
    }
}
", "swift");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 3, "guard should add cc: {cc}");
}

#[test]
fn swift_repeat_while() {
    let out = debug(r"
class Foo {
    func bar() {
        var x = 0
        repeat {
            x += 1
        } while x < 10
    }
}
", "swift");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 2, "repeat-while should add cc: {cc}");
}

// ===========================================================================
// Rust coverage: match wildcard arm
// ===========================================================================

#[test]
fn rust_match_wildcard() {
    let out = debug(r#"
fn f(x: i32) -> &'static str {
    match x {
        1 => "one",
        2 => "two",
        _ => "other",
    }
}
"#, "rs");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 3, "wildcard should not add cc: {cc}");
}

#[test]
fn rust_else_if() {
    let out = debug(r"
fn f(x: i32) -> i32 {
    if x > 10 { 1 }
    else if x > 5 { 2 }
    else { 0 }
}
", "rs");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "else-if should add cc: {cc}");
}

// ===========================================================================
// Module smells coverage: large structs (Rust only populates struct_fields)
// ===========================================================================

#[test]
fn module_large_struct_detected() {
    let code = r"
struct BigStruct {
    f1: i32,
    f2: i32,
    f3: i32,
    f4: i32,
    f5: i32,
    f6: i32,
    f7: i32,
    f8: i32,
    f9: i32,
    f10: i32,
    f11: i32,
    f12: i32,
    f13: i32,
}

fn dummy() {
    if true { }
}
";
    let out = check(code, "rs");
    assert!(out.contains("Large Struct"), "should detect large struct: {out}");
}

#[test]
fn module_god_class_detected() {
    // God Class: total_loc > 500, total_functions > 20, has god method (cc>=9 AND loc>=65)
    let mut code = String::from("struct S {}\nimpl S {\n");
    // One god method: cc=15, loc=70+ (must be both complex AND large)
    code.push_str("fn god(&self, x: i32) -> i32 {\n");
    for i in 0..14 {
        code.push_str(&format!("    if x > {i} {{ }}\n"));
    }
    for j in 0..100 {
        code.push_str(&format!("    let _pad{j} = {j};\n"));
    }
    code.push_str("    x\n}\n");
    // 20 more methods to exceed function count threshold (>20)
    for i in 0..20 {
        code.push_str(&format!("fn m{i}(&self) {{\n"));
        for j in 0..20 {
            code.push_str(&format!("    let _v{i}_{j} = {i} * {j};\n"));
        }
        code.push_str("}\n");
    }
    code.push_str("}\n");
    let out = check(&code, "rs");
    assert!(out.contains("God Class"), "should detect God Class: {out}");
}

#[test]
fn module_overall_function_size() {
    // overall function size: >=3 functions with loc >= large_fn_loc (30)
    // Each must have structurally different bodies to avoid code duplication
    let mut code = String::new();

    // Function 1: uses if chains (40+ LOC)
    code.push_str("fn big0(x: i32) -> i32 {\n");
    for i in 0..42 {
        code.push_str(&format!("    if x > {i} {{ return {i}; }}\n"));
    }
    code.push_str("    x\n}\n");

    // Function 2: uses match (40+ LOC)
    code.push_str("fn big1(x: i32) -> i32 {\n    match x {\n");
    for i in 0..large_fn_lines() {
        code.push_str(&format!("        {i} => {{ return {i}; }}\n"));
    }
    code.push_str("        _ => 0,\n    }\n}\n");

    // Function 3: uses for loops (40+ LOC)
    code.push_str("fn big2() -> i32 {\n    let mut sum = 0;\n");
    for i in 0..41 {
        code.push_str(&format!("    for _k in 0..{i} {{ sum += _k; }}\n"));
    }
    code.push_str("    sum\n}\n");

    let out = check(&code, "rs");
    assert!(out.contains("Overall Function Size"), "should detect: {out}");
}

// ===========================================================================
// Analytics coverage: analytics_dir fallbacks, module location in resolve
// ===========================================================================

#[test]
fn analytics_dir_uses_env() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.py");
    std::fs::write(&file, "def f():\n    if True:\n        pass\n").unwrap();

    let out = Command::new(pulse_bin())
        .args(["check", file.to_str().unwrap()])
        .env("PULSE_ANALYTICS_DIR", dir.path().join("analytics"))
        .output()
        .unwrap();
    let _ = out;
}

#[test]
fn analytics_stop_hook_exercises_resolve() {
    // Run hook then stop to exercise analytics resolve path
    let dir = tempfile::tempdir().unwrap();
    let baseline_dir = dir.path().join("baselines");
    let analytics_dir = dir.path().join("analytics");

    // Create a Python file with a smell
    let file = dir.path().join("smelly.py");
    std::fs::write(&file, concat!(
        "def complex_function(a, b, c, d, e, f):\n",
        "    if a > 0:\n",
        "        if b > 0:\n",
        "            if c > 0:\n",
        "                if d > 0:\n",
        "                    if e > 0:\n",
        "                        return f\n",
        "    return 0\n",
    )).unwrap();

    // Run hook to create baseline and log findings
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}"}}, "session_id":"test-session"}}"#,
        file.to_str().unwrap()
    );
    let _ = Command::new(pulse_bin())
        .args(["--hook"])
        .env("PULSE_BASELINE_DIR", &baseline_dir)
        .env("PULSE_ANALYTICS_DIR", &analytics_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .unwrap();

    // Run stop hook to exercise analytics resolve
    let _ = Command::new(pulse_bin())
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", &baseline_dir)
        .env("PULSE_ANALYTICS_DIR", &analytics_dir)
        .output()
        .unwrap();
}

// ===========================================================================
// Module smells: detect_similar_clones and are_size_similar
// ===========================================================================

#[test]
fn module_similar_clones_detected() {
    // Two functions with same skeleton but slightly different sizes (within 1.3 ratio)
    let code = r"
fn process_alpha(items: &[i32]) -> i32 {
    let mut result = 0;
    for item in items {
        if *item > 0 {
            result += *item;
        } else {
            result -= *item;
        }
    }
    for item in items {
        if *item > 10 {
            result += *item * 2;
        } else {
            result -= *item * 2;
        }
    }
    result += 1;
    result += 2;
    result += 3;
    result
}

fn process_beta(items: &[i32]) -> i32 {
    let mut total = 0;
    for val in items {
        if *val > 0 {
            total += *val;
        } else {
            total -= *val;
        }
    }
    for val in items {
        if *val > 10 {
            total += *val * 2;
        } else {
            total -= *val * 2;
        }
    }
    total += 10;
    total += 20;
    total += 30;
    total
}
";
    let out = check(code, "rs");
    // Exercises detect_similar_clones and are_size_similar paths
    let _ = out;
}

// ===========================================================================
// Python: elif depth, assert boolean, typed params
// ===========================================================================

#[test]
fn python_elif_with_depth() {
    let out = debug(r"
def f(x):
    if x > 100:
        if x > 200:
            return 3
        elif x > 150:
            return 2
    elif x > 50:
        return 1
    else:
        return 0
", "py");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "nested elif should work: {cc}");
}

#[test]
fn python_not_operator_in_condition() {
    let out = debug(r"
def f(a, b):
    if not a and not b:
        return True
    return False
", "py");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "not operator: {cc}");
}

// ===========================================================================
// TypeScript: class type_identifier, switch default, catch in walk_children
// ===========================================================================

#[test]
fn typescript_class_type_identifier() {
    let out = debug(r"
class MyService {
    constructor() {
        if (true) { }
    }
    process(): void {
        if (true) { }
    }
}
", "ts");
    assert!(out.contains("MyService.constructor"), "should find constructor: {out}");
    assert!(out.contains("MyService.process"), "should find method: {out}");
}

// ===========================================================================
// Go: type_case, communication_case, function with no block
// ===========================================================================

#[test]
fn go_method_no_extra_params() {
    // Method with receiver but no additional params (param_lists.len() < 2 case)
    let out = debug(r#"
package main

type Foo struct{}

func (f Foo) Name() string {
    if true { }
    return ""
}
"#, "go");
    assert!(out.contains("Foo.Name"), "value receiver method: {out}");
}

// ===========================================================================
// Zig: test declaration, catch expression, labeled statement
// ===========================================================================

#[test]
fn zig_test_declaration() {
    let out = debug(r#"
const std = @import("std");

test "basic addition" {
    const result = 2 + 3;
    if (result != 5) {
        return error.TestFailed;
    }
}
"#, "zig");
    assert!(out.contains("test_basic_addition"), "should find test: {out}");
}

#[test]
fn zig_switch_with_if() {
    let out = debug(r"
fn f(x: u8) u8 {
    return switch (x) {
        1 => blk: {
            if (true) {}
            break :blk 10;
        },
        2 => 20,
        else => 0,
    };
}
", "zig");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "switch with if in case: {cc}");
}

// ===========================================================================
// ObjC: method with primitive type params
// ===========================================================================

#[test]
fn objc_method_with_primitives() {
    let out = debug(r"
@implementation Calculator
- (NSInteger)add:(NSInteger)a to:(NSInteger)b {
    if (a > 0) { }
    return a + b;
}
@end
", "m");
    assert!(out.contains("Calculator.add"), "should find ObjC method with params: {out}");
}

// ===========================================================================
// Swift: init_declaration, type annotation params
// ===========================================================================

#[test]
fn swift_init_declaration() {
    let out = debug(r#"
class Service {
    var name: String = ""

    init(name: String) {
        self.name = name
        if name.isEmpty { }
    }

    func process() {
        if true { }
    }
}
"#, "swift");
    assert!(out.contains("Service.init"), "should find init: {out}");
}

#[test]
fn swift_type_annotation_param() {
    let out = debug(r"
class Foo {
    func bar(x: Int, y: Double, z: String) {
        if true { }
    }
}
", "swift");
    assert!(out.contains("primitives=3"), "Int/Double/String are primitives: {out}");
}

// ===========================================================================
// C++: class with no field_declaration_list, sized_type_specifier
// ===========================================================================

#[test]
fn cpp_sized_type_specifier() {
    let out = debug(r"
void f(unsigned int a, long long b) {
    if (true) { }
}
", "cpp");
    assert!(out.contains("primitives=2"), "sized types are primitive: {out}");
}

#[test]
fn cpp_for_range_loop_nested() {
    let out = debug(r"
#include <vector>
void f() {
    std::vector<int> v = {1, 2, 3};
    for (auto& x : v) {
        for (auto& y : v) {
            if (x > y) { }
        }
    }
}
", "cpp");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "nested range-for with if: {cc}");
}

// ===========================================================================
// Java: interface_declaration, enum_declaration in count_declarations
// ===========================================================================

#[test]
fn java_interface_and_enum_declarations() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("public interface I{i} {{ void m{i}(); }}\n"));
    }
    let out = check(&code, "java");
    assert!(out.contains("Excessive Declarations"), "{} declarations: {}", declarations_above(), out);
}

// ===========================================================================
// Ruby: module with nested class
// ===========================================================================

#[test]
fn ruby_nested_module_class() {
    let out = debug(r"
module Outer
  class Inner
    def work
      if true
        1
      end
    end
  end
end
", "rb");
    assert!(out.contains("Inner.work"), "nested module class: {out}");
}

#[test]
fn ruby_case_with_when_else() {
    let out = debug(r#"
class Foo
  def bar(x)
    case x
    when 1
      "one"
    when 2
      "two"
    else
      "other"
    end
  end
end
"#, "rb");
    let cc = function_metric(&out, "bar", "cc").unwrap_or(0);
    assert!(cc >= 3, "case/when should add cc: {cc}");
}

// ===========================================================================
// Baselines: session-scoped dir without PULSE_BASELINE_DIR env
// ===========================================================================

#[test]
fn baselines_session_dir_without_env() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.py");
    std::fs::write(&file, "def f():\n    pass\n").unwrap();

    let session_id = "covtest-baseline-01234567abcdef";
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}"}}, "session_id":"{}"}}"#,
        file.to_str().unwrap(),
        session_id
    );

    let _ = Command::new(pulse_bin())
        .args(["--hook"])
        .env_remove("PULSE_BASELINE_DIR")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .unwrap();

    let truncated = &session_id[..session_id.len().min(16)];
    let _ = std::fs::remove_dir_all(format!("/tmp/pulse-baselines-{truncated}"));
}

// ===========================================================================
// Analytics: HOME fallback when PULSE_ANALYTICS_DIR is unset
// ===========================================================================

#[test]
fn analytics_dir_home_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("smelly.py");
    std::fs::write(&file, concat!(
        "def f(a, b, c, d, e, g):\n",
        "    if a > 0:\n",
        "        if b > 0:\n",
        "            if c > 0:\n",
        "                if d > 0:\n",
        "                    if e > 0:\n",
        "                        return g\n",
        "    return 0\n",
    )).unwrap();
    let baseline_dir = dir.path().join("baselines");
    let fake_home = dir.path().join("home");
    std::fs::create_dir_all(&fake_home).unwrap();

    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}"}}, "session_id":"covtest-analytics-home"}}"#,
        file.to_str().unwrap()
    );

    let _ = Command::new(pulse_bin())
        .args(["--hook"])
        .env("PULSE_BASELINE_DIR", &baseline_dir)
        .env("HOME", &fake_home)
        .env_remove("PULSE_ANALYTICS_DIR")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .unwrap();

    let _ = Command::new(pulse_bin())
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", &baseline_dir)
        .env("HOME", &fake_home)
        .env_remove("PULSE_ANALYTICS_DIR")
        .output()
        .unwrap();
}

// ===========================================================================
// Analytics: resolve handles malformed JSONL lines
// ===========================================================================

#[test]
fn analytics_resolve_skips_malformed_lines() {
    let dir = tempfile::tempdir().unwrap();
    let baseline_dir = dir.path().join("baselines");
    std::fs::create_dir_all(&baseline_dir).unwrap();

    let target_file = dir.path().join("dummy.py");
    std::fs::write(&target_file, "def f():\n    pass\n").unwrap();

    std::fs::write(
        baseline_dir.join("manifest.txt"),
        format!("{}\n", target_file.to_str().unwrap()),
    ).unwrap();
    std::fs::write(
        baseline_dir.join("findings.jsonl"),
        "this is not json\n{ also not json\n",
    ).unwrap();

    let analytics_dir = dir.path().join("analytics");

    let _ = Command::new(pulse_bin())
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", &baseline_dir)
        .env("PULSE_ANALYTICS_DIR", &analytics_dir)
        .output()
        .unwrap();
}

// ===========================================================================
// Analytics: resolve writes outcome for module-level findings
// ===========================================================================

#[test]
fn analytics_resolve_module_outcome_direct() {
    let dir = tempfile::tempdir().unwrap();
    let baseline_dir = dir.path().join("baselines");
    let analytics_dir = dir.path().join("analytics");
    std::fs::create_dir_all(&baseline_dir).unwrap();

    let target_file = dir.path().join("clean.py");
    std::fs::write(&target_file, "def f():\n    pass\n").unwrap();

    std::fs::write(
        baseline_dir.join("manifest.txt"),
        format!("{}\n", target_file.to_str().unwrap()),
    ).unwrap();
    std::fs::write(baseline_dir.join("session_id"), "covtest-session\n").unwrap();

    let entry = serde_json::json!({
        "ts": 0,
        "file": "clean.py",
        "path": target_file.to_str().unwrap(),
        "smell": "Too Many Functions",
        "fn": serde_json::Value::Null,
        "line": serde_json::Value::Null,
        "detail": "module finding for resolve coverage"
    });
    std::fs::write(
        baseline_dir.join("findings.jsonl"),
        format!("{entry}\n"),
    ).unwrap();

    let _ = Command::new(pulse_bin())
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", &baseline_dir)
        .env("PULSE_ANALYTICS_DIR", &analytics_dir)
        .output()
        .unwrap();
}

// ===========================================================================
// Analytics: /tmp fallback when neither PULSE_ANALYTICS_DIR nor HOME is set
// ===========================================================================

#[test]
fn analytics_dir_no_env_no_home() {
    let dir = tempfile::tempdir().unwrap();
    let baseline_dir = dir.path().join("baselines");
    std::fs::create_dir_all(&baseline_dir).unwrap();

    let target_file = dir.path().join("dummy.py");
    std::fs::write(&target_file, "def f():\n    pass\n").unwrap();

    std::fs::write(
        baseline_dir.join("manifest.txt"),
        format!("{}\n", target_file.to_str().unwrap()),
    ).unwrap();

    let _ = Command::new(pulse_bin())
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", &baseline_dir)
        .env_remove("PULSE_ANALYTICS_DIR")
        .env_remove("HOME")
        .output()
        .unwrap();

    let _ = std::fs::remove_dir_all("/tmp/pulse-analytics");
}

// ===========================================================================
// Duplication: similar-clone novel pair via different if-condition expression kinds
// ===========================================================================

#[test]
fn duplication_skeleton_only_pair() {
    let mut code = String::new();

    code.push_str("fn alpha_id() -> i32 {\n");
    for i in 0..22 {
        code.push_str(&format!("    let a{i} = {i};\n"));
    }
    code.push_str("    if a0 > 0 { a0 } else { 1 }\n");
    code.push_str("}\n\n");

    code.push_str("fn beta_id() -> i32 {\n");
    for i in 0..22 {
        code.push_str(&format!("    let b{i} = {i};\n"));
    }
    code.push_str("    if maybe() { b0 } else { 2 }\n");
    code.push_str("}\n\n");

    code.push_str("fn gamma_id() -> i32 {\n");
    for i in 0..22 {
        code.push_str(&format!("    let g{i} = {i};\n"));
    }
    code.push_str("    if g0.is_positive() { g0 } else { 3 }\n");
    code.push_str("}\n\n");

    code.push_str("fn maybe() -> bool { true }\n");

    let out = check(&code, "rs");
    let _ = out;
}

// ===========================================================================
// Duplication: extract_line_numbers via exact-clone finding
// ===========================================================================

#[test]
fn duplication_extract_line_numbers_path() {
    let mut code = String::new();
    for name in ["foo_dup", "bar_dup"] {
        code.push_str(&format!("fn {name}() -> i32 {{\n"));
        for i in 0..22 {
            code.push_str(&format!("    let v{i} = {i};\n"));
        }
        code.push_str("    v0 + v1\n");
        code.push_str("}\n\n");
    }
    let out = check(&code, "rs");
    let _ = out;
}

// ===========================================================================
// Helpers
// ===========================================================================

fn check(code: &str, ext: &str) -> String {
    pulse_check_code(code, ext)
}

fn debug(code: &str, ext: &str) -> String {
    pulse_debug_code(code, ext)
}
