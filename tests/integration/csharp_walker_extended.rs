use pulse::parse::{parse_and_walk, Language};

use crate::common::t;

fn analyze(source: &str) -> pulse::walk::FileMetrics {
    parse_and_walk(source, Language::CSharp).expect("parse C#")
}

#[test]
fn auto_property_with_get_set_handled() {
    let source = "class M {\n    public int X { get; set; }\n}\n";
    let _ = analyze(source);
}

#[test]
fn property_with_explicit_accessors_handled() {
    let source = "class M {\n    private int _x;\n    public int X { get { return _x; } set { _x = value; } }\n}\n";
    let _ = analyze(source);
}

#[test]
fn indexer_declaration_handled() {
    let source = "class M {\n    public int this[int i] { get { return i; } }\n}\n";
    let _ = analyze(source);
}

#[test]
fn event_declaration_handled() {
    let source = "class M {\n    public event System.Action OnReady;\n}\n";
    let _ = analyze(source);
}

#[test]
fn async_method_with_await_handled() {
    let source = "using System.Threading.Tasks;\n\nclass M {\n    public async Task Run() {\n        await Task.Delay(100);\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn expression_bodied_method_handled() {
    let source = "class M {\n    public int Square(int x) => x * x;\n}\n";
    let _ = analyze(source);
}

#[test]
fn record_declaration_handled() {
    let source = "public record Point(int X, int Y);\n";
    let _ = analyze(source);
}

#[test]
fn pattern_matching_in_switch_expression_handled() {
    let source = "class M {\n    public string Describe(object o) => o switch {\n        int n when n > 0 => \"positive\",\n        int n when n < 0 => \"negative\",\n        int _ => \"zero\",\n        _ => \"other\",\n    };\n}\n";
    let _ = analyze(source);
}

#[test]
fn local_function_within_method_handled() {
    let source = "class M {\n    public int Run() {\n        int local() { return 42; }\n        return local();\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn lambda_expression_handled() {
    let source = "using System;\n\nclass M {\n    public void Run() {\n        Action<int> a = x => System.Console.WriteLine(x);\n        a(1);\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn partial_class_handled() {
    let source = "public partial class M {\n    public void A() {}\n}\npublic partial class M {\n    public void B() {}\n}\n";
    let _ = analyze(source);
}

#[test]
fn try_finally_only_no_catch_handled() {
    let source = "class M {\n    public void Run() {\n        try { System.Console.WriteLine(\"x\"); }\n        finally { System.Console.WriteLine(\"end\"); }\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn try_with_multiple_catches_handled() {
    let source = "using System;\n\nclass M {\n    public void Run() {\n        try { Console.WriteLine(\"x\"); }\n        catch (ArgumentException) { Console.WriteLine(\"a\"); }\n        catch (InvalidOperationException) { Console.WriteLine(\"b\"); }\n        catch (Exception) { Console.WriteLine(\"e\"); }\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn switch_with_default_case_handled() {
    let source = "class M {\n    public string F(int x) {\n        switch (x) {\n            case 1: return \"one\";\n            case 2: return \"two\";\n            default: return \"other\";\n        }\n    }\n}\n";
    let metrics = analyze(source);
    let _ = metrics.functions;
}

#[test]
fn generic_method_handled() {
    let source = "class M {\n    public T Identity<T>(T x) { return x; }\n}\n";
    let _ = analyze(source);
}

#[test]
fn constructor_with_initializer_handled() {
    let source = "class A {\n    public A(int x) {}\n}\nclass B : A {\n    public B(int x) : base(x) {}\n}\n";
    let _ = analyze(source);
}

#[test]
fn static_constructor_handled() {
    let source = "class M {\n    static M() {}\n}\n";
    let _ = analyze(source);
}

#[test]
fn nullable_reference_types_handled() {
    let source = "class M {\n    public string? GetValue() { return null; }\n}\n";
    let _ = analyze(source);
}

#[test]
fn using_statement_handled() {
    let source = "using System.IO;\n\nclass M {\n    public void Read() {\n        using (var f = new StreamReader(\"x\")) {\n            f.ReadLine();\n        }\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn empty_csharp_file_no_panic() {
    let _ = analyze("\n");
}

#[test]
fn malformed_csharp_no_panic() {
    let _ = analyze("class M { public void");
}

#[test]
fn complex_method_yields_high_cc() {
    let source = "class M {\n    public int F(int x, int y) {\n        if (x > 0) {\n            if (y > 0) {\n                while (x > 0) { x--; y--; }\n                for (int i = 0; i < y; i++) { x += i; }\n            } else if (y < 0) {\n                x = -x;\n            }\n        } else {\n            x = 0;\n        }\n        return x;\n    }\n}\n";
    let metrics = analyze(source);
    let max_cc = metrics.functions.iter().map(|f| f.cc).max().unwrap_or(0);
    assert!(max_cc >= 4, "complex method should have cc >= 4 (found {max_cc})");
    let _ = t().function;
}

#[test]
fn nested_class_handled() {
    let source = "class Outer {\n    public class Inner {\n        public void M() {}\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn interface_declaration_handled() {
    let source = "public interface IRunnable {\n    void Run();\n    int Count { get; }\n}\n";
    let _ = analyze(source);
}

#[test]
fn enum_declaration_handled() {
    let source = "public enum Color {\n    Red, Green, Blue\n}\n";
    let _ = analyze(source);
}

#[test]
fn struct_declaration_handled() {
    let source = "public struct Point {\n    public int X;\n    public int Y;\n}\n";
    let _ = analyze(source);
}

#[test]
fn null_coalescing_operator_handled() {
    let source = "class M {\n    public string F(string a) { return a ?? \"default\"; }\n}\n";
    let _ = analyze(source);
}

#[test]
fn null_conditional_operator_handled() {
    let source = "class M {\n    public int F(string a) { return a?.Length ?? 0; }\n}\n";
    let _ = analyze(source);
}

#[test]
fn typeof_expression_handled() {
    let source = "class M {\n    public System.Type F() { return typeof(int); }\n}\n";
    let _ = analyze(source);
}

#[test]
fn ref_and_out_parameters_handled() {
    let source = "class M {\n    public void F(ref int a, out int b) { b = a + 1; }\n}\n";
    let _ = analyze(source);
}

#[test]
fn for_each_loop_handled() {
    let source = "class M {\n    public int F(int[] arr) {\n        int sum = 0;\n        foreach (var x in arr) { sum += x; }\n        return sum;\n    }\n}\n";
    let _ = analyze(source);
}
