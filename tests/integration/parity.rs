use pulse::parse::{self, Language};
use pulse::walk::FunctionMetrics;
use std::ops::RangeInclusive;
use std::path::PathBuf;

const FIXTURES: &[(Language, &str)] = &[
    (Language::Python, "shapes.py"),
    (Language::TypeScript, "shapes.ts"),
    (Language::JavaScript, "shapes.js"),
    (Language::Rust, "shapes.rs"),
    (Language::C, "shapes.c"),
    (Language::Cpp, "shapes.cpp"),
    (Language::Java, "shapes.java"),
    (Language::CSharp, "shapes.cs"),
    (Language::Go, "shapes.go"),
    (Language::Swift, "shapes.swift"),
    (Language::Zig, "shapes.zig"),
    (Language::Ruby, "shapes.rb"),
    (Language::ObjectiveC, "shapes.m"),
    (Language::Tcl, "shapes.tcl"),
    (Language::Kotlin, "shapes.kt"),
    (Language::Haskell, "Shapes.hs"),
    (Language::Lua, "shapes.lua"),
    (Language::R, "shapes.r"),
    (Language::Php, "shapes.php"),
    (Language::Cobol, "shapes.cob"),
    (Language::D, "shapes.d"),
    (Language::Groovy, "shapes.groovy"),
];

const SHAPES: &[&str] = &["flat_calls", "pick_branch", "nested_guard", "loop_filter", "wide_params", "bool_blend"];

#[derive(Clone)]
struct Band {
    cc: RangeInclusive<u32>,
    nesting: RangeInclusive<u32>,
    loc: RangeInclusive<u32>,
    args: Option<u32>,
}

fn canonical_band(shape: &str) -> Band {
    match shape {
        "flat_calls" => Band { cc: 1..=1, nesting: 0..=0, loc: 4..=9, args: Some(0) },
        "pick_branch" => Band { cc: 3..=3, nesting: 1..=2, loc: 6..=15, args: Some(1) },
        "nested_guard" => Band { cc: 4..=4, nesting: 3..=3, loc: 6..=14, args: Some(3) },
        "loop_filter" => Band { cc: 3..=3, nesting: 2..=2, loc: 6..=14, args: Some(1) },
        "wide_params" => Band { cc: 1..=1, nesting: 0..=0, loc: 2..=5, args: Some(6) },
        "bool_blend" => Band { cc: 5..=5, nesting: 1..=1, loc: 4..=10, args: Some(4) },
        other => panic!("unknown shape {other}"),
    }
}

fn band_for(lang: Language, shape: &str) -> Option<Band> {
    let mut band = canonical_band(shape);
    match (lang, shape) {
        (Language::Cobol, "wide_params") => return None,
        (Language::Cobol, _) => {
            band.args = None;
            if shape == "pick_branch" {
                band.nesting = 1..=2;
            }
            if shape == "loop_filter" {
                band.loc = 6..=16;
            }
        }
        (Language::Haskell, "flat_calls") => band.args = None,
        (Language::Haskell, "wide_params") => band.loc = 1..=5,
        (Language::Haskell, "loop_filter") => {
            band = Band { cc: 1..=4, nesting: 0..=3, loc: 4..=12, args: Some(1) };
        }
        _ => {}
    }
    Some(band)
}

fn parity_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("parity")
}

fn normalize(name: &str) -> String {
    let bare = name.rsplit('.').next().unwrap_or(name);
    bare.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_lowercase()
}

fn shape_metrics<'a>(functions: &'a [FunctionMetrics], shape: &str) -> Option<&'a FunctionMetrics> {
    let want = normalize(shape);
    functions.iter().find(|f| normalize(&f.name).starts_with(&want))
}

fn describe(f: &FunctionMetrics) -> String {
    format!(
        "cc={} nesting={} loc={} args={} compound={}",
        f.cc, f.max_nesting, f.loc, f.arg_count, f.compound_condition_count
    )
}

fn check_shape(lang: Language, shape: &str, functions: &[FunctionMetrics], violations: &mut Vec<String>) {
    let Some(band) = band_for(lang, shape) else { return };
    let Some(f) = shape_metrics(functions, shape) else {
        let found: Vec<String> = functions.iter().map(|f| f.name.clone()).collect();
        violations.push(format!("{lang:?}/{shape}: function not found (walker saw: {found:?})"));
        return;
    };
    if !band.cc.contains(&f.cc) {
        violations.push(format!("{lang:?}/{shape}: cc {} outside {:?}  [{}]", f.cc, band.cc, describe(f)));
    }
    if !band.nesting.contains(&f.max_nesting) {
        violations.push(format!(
            "{lang:?}/{shape}: nesting {} outside {:?}  [{}]",
            f.max_nesting,
            band.nesting,
            describe(f)
        ));
    }
    if !band.loc.contains(&f.loc) {
        violations.push(format!("{lang:?}/{shape}: loc {} outside {:?}  [{}]", f.loc, band.loc, describe(f)));
    }
    if let Some(args) = band.args {
        if f.arg_count != args {
            violations.push(format!("{lang:?}/{shape}: args {} != {}  [{}]", f.arg_count, args, describe(f)));
        }
    }
}

#[test]
fn every_language_has_a_parity_fixture() {
    for lang in Language::ALL {
        assert!(
            FIXTURES.iter().any(|(l, _)| *l == lang),
            "{lang:?} missing from the parity fixture table — add tests/fixtures/parity coverage"
        );
    }
    assert_eq!(FIXTURES.len(), Language::COUNT);
}

#[test]
fn canonical_shapes_agree_across_languages() {
    let mut violations = Vec::new();
    for (lang, file) in FIXTURES {
        let path = parity_root().join(file);
        let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let Some(metrics) = parse::parse_and_walk_guarded(&source, *lang) else {
            violations.push(format!("{lang:?}: fixture failed to parse"));
            continue;
        };
        for shape in SHAPES {
            check_shape(*lang, shape, &metrics.functions, &mut violations);
        }
    }
    assert!(violations.is_empty(), "cross-language parity violations:\n{}", violations.join("\n"));
}
