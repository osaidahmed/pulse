use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pulse::parse;
use pulse::smells;
use pulse::thresholds::Thresholds;

fn generate_python(num_functions: usize) -> String {
    let mut code = String::new();
    for i in 0..num_functions {
        code.push_str(&format!("def fn_{i}(a, b, c, d, e, f):\n"));
        for j in 0..10 {
            code.push_str(&format!("    if a == {j}:\n"));
            code.push_str(&format!("        for x in range({j}):\n"));
            code.push_str(&format!("            if b == {j}:\n"));
            code.push_str("                pass\n");
        }
        code.push_str("    return a\n\n");
    }
    code
}

fn generate_typescript(num_functions: usize) -> String {
    let mut code = String::new();
    for i in 0..num_functions {
        code.push_str(&format!(
            "function fn_{i}(a: number, b: number, c: number): number {{\n"
        ));
        for j in 0..10 {
            code.push_str(&format!("    if (a === {j}) {{\n"));
            code.push_str(&format!("        for (let x = 0; x < {j}; x++) {{\n"));
            code.push_str(&format!("            if (b === {j}) {{}}\n"));
            code.push_str("        }\n");
            code.push_str("    }\n");
        }
        code.push_str("    return a;\n}\n\n");
    }
    code
}

fn generate_rust_code(num_functions: usize) -> String {
    let mut code = String::new();
    for i in 0..num_functions {
        code.push_str(&format!("fn fn_{i}(a: i32, b: i32, c: i32) -> i32 {{\n"));
        for j in 0..10 {
            code.push_str(&format!("    if a == {j} {{\n"));
            code.push_str(&format!("        for x in 0..{j} {{\n"));
            code.push_str(&format!("            if b == {j} {{}}\n"));
            code.push_str("        }\n");
            code.push_str("    }\n");
        }
        code.push_str("    a\n}\n\n");
    }
    code
}

// Full pipeline: parse + walk + detect
fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    for &num_fns in &[10, 30, 100, 300] {
        let code = generate_python(num_fns);
        let lines = code.lines().count();
        group.bench_with_input(
            BenchmarkId::new("python", format!("{num_fns}fn_{lines}loc")),
            &code,
            |b, code| {
                b.iter(|| {
                    let metrics =
                        parse::parse_and_walk(black_box(code), parse::Language::Python).unwrap();
                    let t = Thresholds::default();
                    smells::detect(&metrics, code, &t)
                });
            },
        );
    }

    for &num_fns in &[10, 30, 100, 300] {
        let code = generate_typescript(num_fns);
        let lines = code.lines().count();
        group.bench_with_input(
            BenchmarkId::new("typescript", format!("{num_fns}fn_{lines}loc")),
            &code,
            |b, code| {
                b.iter(|| {
                    let metrics =
                        parse::parse_and_walk(black_box(code), parse::Language::TypeScript)
                            .unwrap();
                    let t = Thresholds::default();
                    smells::detect(&metrics, code, &t)
                });
            },
        );
    }

    for &num_fns in &[10, 30, 100, 300] {
        let code = generate_rust_code(num_fns);
        let lines = code.lines().count();
        group.bench_with_input(
            BenchmarkId::new("rust", format!("{num_fns}fn_{lines}loc")),
            &code,
            |b, code| {
                b.iter(|| {
                    let metrics =
                        parse::parse_and_walk(black_box(code), parse::Language::Rust).unwrap();
                    let t = Thresholds::default();
                    smells::detect(&metrics, code, &t)
                });
            },
        );
    }

    group.finish();
}

// Isolate: parse only vs walk+detect
fn bench_parse_vs_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_vs_analysis");

    let code = generate_python(100);

    group.bench_function("parse_only_100fn", |b| {
        b.iter(|| {
            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(&tree_sitter_python::LANGUAGE.into())
                .unwrap();
            parser.parse(black_box(&code), None).unwrap()
        });
    });

    group.bench_function("walk_and_detect_100fn", |b| {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(&code, None).unwrap();
        b.iter(|| {
            let metrics = pulse::walk::python::walk(black_box(&tree), &code);
            let t = Thresholds::default();
            smells::detect(&metrics, &code, &t)
        });
    });

    group.finish();
}

// Scaling: how does time grow with LOC?
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_python");

    for &num_fns in &[1, 5, 10, 50, 100, 200, 500] {
        let code = generate_python(num_fns);
        let lines = code.lines().count();
        group.bench_with_input(
            BenchmarkId::new("loc", lines),
            &code,
            |b, code| {
                b.iter(|| {
                    let metrics =
                        parse::parse_and_walk(black_box(code), parse::Language::Python).unwrap();
                    let t = Thresholds::default();
                    smells::detect(&metrics, code, &t)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_full_pipeline, bench_parse_vs_analysis, bench_scaling);
criterion_main!(benches);
