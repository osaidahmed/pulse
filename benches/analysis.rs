use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pulse::parse;
use pulse::smells;
use pulse::thresholds::Thresholds;

#[derive(Clone, Copy)]
enum BenchLang {
    Python,
    TypeScript,
    Rust,
}

fn generate_code(lang: BenchLang, num_functions: usize) -> String {
    let (header, body, footer) = match lang {
        BenchLang::Python => (
            "def fn_{I}(a, b, c, d, e, f):\n",
            "    if a == {J}:\n        for x in range({J}):\n            if b == {J}:\n                pass\n",
            "    return a\n\n",
        ),
        BenchLang::TypeScript => (
            "function fn_{I}(a: number, b: number, c: number): number {\n",
            "    if (a === {J}) {\n        for (let x = 0; x < {J}; x++) {\n            if (b === {J}) {}\n        }\n    }\n",
            "    return a;\n}\n\n",
        ),
        BenchLang::Rust => (
            "fn fn_{I}(a: i32, b: i32, c: i32) -> i32 {\n",
            "    if a == {J} {\n        for x in 0..{J} {\n            if b == {J} {}\n        }\n    }\n",
            "    a\n}\n\n",
        ),
    };
    let mut code = String::new();
    for i in 0..num_functions {
        code.push_str(&header.replace("{I}", &i.to_string()));
        for j in 0..10 {
            code.push_str(&body.replace("{J}", &j.to_string()));
        }
        code.push_str(footer);
    }
    code
}

// Full pipeline: parse + walk + detect
fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    for &num_fns in &[10, 30, 100, 300] {
        let code = generate_code(BenchLang::Python, num_fns);
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
        let code = generate_code(BenchLang::TypeScript, num_fns);
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
        let code = generate_code(BenchLang::Rust, num_fns);
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

    let code = generate_code(BenchLang::Python, 100);

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
        let code = generate_code(BenchLang::Python, num_fns);
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
