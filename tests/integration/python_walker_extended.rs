use pulse::parse::{parse_and_walk, Language};

fn analyze(source: &str) -> pulse::walk::FileMetrics {
    parse_and_walk(source, Language::Python).expect("parse Python")
}

#[test]
fn match_case_statement_handled() {
    let source = "def f(x):\n    match x:\n        case 1:\n            return 'one'\n        case 2:\n            return 'two'\n        case _:\n            return 'other'\n";
    let _ = analyze(source);
}

#[test]
fn walrus_operator_in_condition_handled() {
    let source = "def f(items):\n    if (n := len(items)) > 5:\n        return n\n    return 0\n";
    let _ = analyze(source);
}

#[test]
fn async_def_with_await_handled() {
    let source = "import asyncio\n\nasync def f():\n    await asyncio.sleep(1)\n    return 42\n";
    let _ = analyze(source);
}

#[test]
fn list_comprehension_handled() {
    let source = "def f(items):\n    return [x * 2 for x in items if x > 0]\n";
    let _ = analyze(source);
}

#[test]
fn nested_comprehension_handled() {
    let source = "def f(matrix):\n    return [[cell * 2 for cell in row] for row in matrix]\n";
    let _ = analyze(source);
}

#[test]
fn dict_comprehension_handled() {
    let source = "def f(items):\n    return {k: v * 2 for k, v in items.items()}\n";
    let _ = analyze(source);
}

#[test]
fn set_comprehension_handled() {
    let source = "def f(items):\n    return {x for x in items if x > 0}\n";
    let _ = analyze(source);
}

#[test]
fn generator_expression_handled() {
    let source = "def f(items):\n    return sum(x * 2 for x in items)\n";
    let _ = analyze(source);
}

#[test]
fn multi_context_manager_handled() {
    let source = "def f():\n    with open('a') as fa, open('b') as fb:\n        return fa.read() + fb.read()\n";
    let _ = analyze(source);
}

#[test]
fn complex_decorator_chain_handled() {
    let source =
        "def deco1(fn):\n    return fn\n\ndef deco2(fn):\n    return fn\n\n@deco1\n@deco2\ndef f():\n    return 1\n";
    let _ = analyze(source);
}

#[test]
fn class_with_multiple_inheritance_handled() {
    let source = "class Mixin1: pass\nclass Mixin2: pass\nclass Combined(Mixin1, Mixin2): pass\n";
    let _ = analyze(source);
}

#[test]
fn class_with_no_base_handled() {
    let source = "class Lonely:\n    def m(self): pass\n";
    let _ = analyze(source);
}

#[test]
fn yield_in_function_handled() {
    let source = "def f():\n    for i in range(10):\n        yield i\n";
    let _ = analyze(source);
}

#[test]
fn yield_from_handled() {
    let source = "def f():\n    yield from range(10)\n";
    let _ = analyze(source);
}

#[test]
fn try_except_finally_handled() {
    let source = "def f():\n    try:\n        risky()\n    except ValueError as e:\n        handle(e)\n    except (KeyError, IndexError):\n        handle_lookup()\n    except Exception:\n        log()\n    finally:\n        cleanup()\n";
    let _ = analyze(source);
}

#[test]
fn complex_method_yields_high_cc() {
    let source = "def f(x, y):\n    if x > 0:\n        if y > 0:\n            for i in range(y):\n                if i % 2 == 0:\n                    print(i)\n        elif y < 0:\n            x = -x\n    else:\n        x = 0\n    return x\n";
    let metrics = analyze(source);
    let max_cc = metrics.functions.iter().map(|f| f.cc).max().unwrap_or(0);
    assert!(max_cc >= 4);
}

#[test]
fn type_alias_python_312_handled() {
    let source = "type Vector = list[float]\n";
    let _ = analyze(source);
}

#[test]
fn class_method_decorator_handled() {
    let source = "class Foo:\n    @classmethod\n    def create(cls):\n        return cls()\n";
    let _ = analyze(source);
}

#[test]
fn static_method_decorator_handled() {
    let source = "class Foo:\n    @staticmethod\n    def utility():\n        return 42\n";
    let _ = analyze(source);
}

#[test]
fn property_decorator_handled() {
    let source = "class Foo:\n    @property\n    def value(self):\n        return self._x\n    @value.setter\n    def value(self, v):\n        self._x = v\n";
    let _ = analyze(source);
}

#[test]
fn lambda_expression_handled() {
    let source = "double = lambda x: x * 2\nresult = double(5)\n";
    let _ = analyze(source);
}

#[test]
fn nested_function_handled() {
    let source = "def outer(x):\n    def inner(y):\n        return x + y\n    return inner\n";
    let _ = analyze(source);
}

#[test]
fn empty_python_file_no_panic() {
    let _ = analyze("\n");
}

#[test]
fn malformed_python_no_panic() {
    let _ = analyze("def f(\n    return\n");
}

#[test]
fn unicode_identifiers_handled() {
    let source = "def функция(параметр):\n    return параметр + 1\n";
    let _ = analyze(source);
}

#[test]
fn type_annotations_handled() {
    let source = "from typing import List, Optional\n\ndef f(x: int, items: List[str] = None) -> Optional[str]:\n    return items[0] if items else None\n";
    let _ = analyze(source);
}

#[test]
fn assignment_expression_in_loop_handled() {
    let source = "import sys\n\ndef f():\n    while (chunk := sys.stdin.read(8192)):\n        process(chunk)\n";
    let _ = analyze(source);
}

#[test]
fn star_args_kwargs_handled() {
    let source = "def f(*args, **kwargs):\n    return len(args) + len(kwargs)\n";
    let _ = analyze(source);
}

#[test]
fn raise_with_from_handled() {
    let source =
        "def f():\n    try:\n        risky()\n    except Exception as e:\n        raise ValueError('bad') from e\n";
    let _ = analyze(source);
}

#[test]
fn multiple_function_definitions_each_extracted() {
    let source = "def a(): pass\ndef b(): pass\ndef c(): pass\n";
    let metrics = analyze(source);
    let names: Vec<String> = metrics.functions.iter().map(|f| f.name.clone()).collect();
    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"b".to_string()));
    assert!(names.contains(&"c".to_string()));
}
