use pulse_syntax::parse::{parse_and_walk, Language};

fn analyze(source: &str, lang: Language) -> pulse_syntax::walk::FileMetrics {
    parse_and_walk(source, lang).unwrap_or_else(|| panic!("parse {lang:?}"))
}

#[test]
fn kotlin_sealed_class_handled() {
    let source = "sealed class Result {\n    object Loading : Result()\n    data class Success(val data: String) : Result()\n    data class Error(val msg: String) : Result()\n}\n";
    let _ = analyze(source, Language::Kotlin);
}

#[test]
fn kotlin_data_class_handled() {
    let source = "data class Point(val x: Int, val y: Int)\n";
    let _ = analyze(source, Language::Kotlin);
}

#[test]
fn kotlin_suspend_function_handled() {
    let source = "import kotlinx.coroutines.delay\n\nsuspend fun load(): Int {\n    delay(100)\n    return 42\n}\n";
    let _ = analyze(source, Language::Kotlin);
}

#[test]
fn kotlin_infix_function_handled() {
    let source = "infix fun Int.plus3(other: Int): Int = this + other + 3\n";
    let _ = analyze(source, Language::Kotlin);
}

#[test]
fn kotlin_when_expression_handled() {
    let source = "fun describe(x: Int): String = when {\n    x > 0 -> \"positive\"\n    x < 0 -> \"negative\"\n    else -> \"zero\"\n}\n";
    let _ = analyze(source, Language::Kotlin);
}

#[test]
fn kotlin_extension_function_handled() {
    let source = "fun String.shout() = uppercase() + \"!\"\n";
    let _ = analyze(source, Language::Kotlin);
}

#[test]
fn kotlin_lambda_with_receiver_handled() {
    let source = "fun build(action: StringBuilder.() -> Unit): String {\n    return StringBuilder().apply(action).toString()\n}\n";
    let _ = analyze(source, Language::Kotlin);
}

#[test]
fn java_record_declaration_handled() {
    let source = "public record Point(int x, int y) {}\n";
    let _ = analyze(source, Language::Java);
}

#[test]
fn java_sealed_class_handled() {
    let source = "public sealed interface Shape permits Circle, Square {}\nfinal class Circle implements Shape {}\nfinal class Square implements Shape {}\n";
    let _ = analyze(source, Language::Java);
}

#[test]
fn java_var_inference_handled() {
    let source = "class M {\n    void run() {\n        var list = new java.util.ArrayList<Integer>();\n        list.add(42);\n    }\n}\n";
    let _ = analyze(source, Language::Java);
}

#[test]
fn java_lambda_expressions_handled() {
    let source =
        "import java.util.function.Function;\n\nclass M {\n    Function<Integer,Integer> doubler = x -> x * 2;\n}\n";
    let _ = analyze(source, Language::Java);
}

#[test]
fn java_enhanced_for_loop_handled() {
    let source = "class M {\n    int sum(int[] arr) {\n        int s = 0;\n        for (int x : arr) s += x;\n        return s;\n    }\n}\n";
    let _ = analyze(source, Language::Java);
}

#[test]
fn java_try_with_resources_handled() {
    let source = "import java.io.*;\n\nclass M {\n    String read(String p) throws IOException {\n        try (BufferedReader r = new BufferedReader(new FileReader(p))) {\n            return r.readLine();\n        }\n    }\n}\n";
    let _ = analyze(source, Language::Java);
}

#[test]
fn java_switch_expression_handled() {
    let source = "class M {\n    String describe(int x) {\n        return switch (x) {\n            case 1 -> \"one\";\n            case 2, 3 -> \"two or three\";\n            default -> \"other\";\n        };\n    }\n}\n";
    let _ = analyze(source, Language::Java);
}

#[test]
fn java_generic_method_handled() {
    let source = "import java.util.List;\nclass M {\n    public <T> T first(List<T> items) {\n        return items.get(0);\n    }\n}\n";
    let _ = analyze(source, Language::Java);
}

#[test]
fn typescript_discriminated_union_handled() {
    let source = "type Result = { kind: 'ok'; value: number } | { kind: 'err'; msg: string };\n\nfunction handle(r: Result): number {\n    if (r.kind === 'ok') return r.value;\n    return -1;\n}\n";
    let _ = analyze(source, Language::TypeScript);
}

#[test]
fn typescript_generic_constraints_handled() {
    let source = "function first<T extends { length: number }>(item: T): T { return item; }\n";
    let _ = analyze(source, Language::TypeScript);
}

#[test]
fn typescript_conditional_type_handled() {
    let source = "type IsString<T> = T extends string ? true : false;\n";
    let _ = analyze(source, Language::TypeScript);
}

#[test]
fn typescript_mapped_type_handled() {
    let source = "type ReadonlyVersion<T> = { readonly [K in keyof T]: T[K] };\n";
    let _ = analyze(source, Language::TypeScript);
}

#[test]
fn typescript_arrow_function_handled() {
    let source = "const double = (x: number): number => x * 2;\n";
    let _ = analyze(source, Language::TypeScript);
}

#[test]
fn typescript_async_function_handled() {
    let source = "async function load(): Promise<number> {\n    return 42;\n}\n";
    let _ = analyze(source, Language::TypeScript);
}

#[test]
fn typescript_decorator_handled() {
    let source = "function log(target: any) { return target; }\n@log\nclass M { run() {} }\n";
    let _ = analyze(source, Language::TypeScript);
}

#[test]
fn cpp_template_function_handled() {
    let source = "template<typename T>\nT identity(T x) { return x; }\n";
    let _ = analyze(source, Language::Cpp);
}

#[test]
fn cpp_variadic_template_handled() {
    let source = "template<typename... Args>\nint count(Args... args) { return sizeof...(args); }\n";
    let _ = analyze(source, Language::Cpp);
}

#[test]
fn cpp_constexpr_function_handled() {
    let source = "constexpr int square(int x) { return x * x; }\n";
    let _ = analyze(source, Language::Cpp);
}

#[test]
fn cpp_lambda_handled() {
    let source = "void run() {\n    auto add = [](int a, int b) { return a + b; };\n    add(1, 2);\n}\n";
    let _ = analyze(source, Language::Cpp);
}

#[test]
fn cpp_try_catch_handled() {
    let source = "void run() {\n    try {\n        risky();\n    } catch (const std::exception& e) {\n        handle(e);\n    } catch (...) {\n        handle_unknown();\n    }\n}\n";
    let _ = analyze(source, Language::Cpp);
}

#[test]
fn cpp_namespace_handled() {
    let source = "namespace ns {\n    int add(int a, int b) { return a + b; }\n}\n";
    let _ = analyze(source, Language::Cpp);
}

#[test]
fn rust_async_function_handled() {
    let source = "async fn load() -> u32 {\n    42\n}\n";
    let _ = analyze(source, Language::Rust);
}

#[test]
fn rust_trait_definition_handled() {
    let source = "trait Animal {\n    fn name(&self) -> &str;\n    fn sound(&self) -> String;\n}\n";
    let _ = analyze(source, Language::Rust);
}

#[test]
fn rust_match_with_guards_handled() {
    let source = "fn describe(x: i32) -> &'static str {\n    match x {\n        n if n > 0 => \"positive\",\n        n if n < 0 => \"negative\",\n        _ => \"zero\",\n    }\n}\n";
    let _ = analyze(source, Language::Rust);
}

#[test]
fn rust_closure_with_captures_handled() {
    let source =
        "fn make_counter() -> impl FnMut() -> i32 {\n    let mut count = 0;\n    move || { count += 1; count }\n}\n";
    let _ = analyze(source, Language::Rust);
}
