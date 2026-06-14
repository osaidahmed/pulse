use pulse::analyze::{analyze_source, ScanOptions};
use pulse::config::PulseConfig;
use pulse::parse::Language;
use pulse::smells::{Finding, Smell};

fn cpg_config(extra: &str) -> PulseConfig {
    toml::from_str(&format!("[thresholds.cpg]\nenabled = true\nuse_before_def = true\n{extra}")).unwrap()
}

fn findings_of(src: &str, lang: Language, ext: &str, extra: &str) -> Vec<Finding> {
    let cfg = cpg_config(extra);
    let path = format!("t.{ext}");
    analyze_source(&path, src, lang, Some(&cfg), ScanOptions::check()).expect("analyze").findings
}

fn count(findings: &[Finding], smell: Smell) -> usize {
    findings.iter().filter(|f| f.smell == smell).count()
}

fn details(findings: &[Finding], smell: Smell) -> String {
    findings.iter().filter(|f| f.smell == smell).map(|f| f.detail.clone()).collect::<Vec<_>>().join("\n")
}

#[test]
fn unreachable_line_is_reported_exactly() {
    let src = "def f():\n    x = 1\n    return x\n    y = 2\n";
    let out = findings_of(src, Language::Python, "py", "");
    let d = details(&out, Smell::UnreachableCode);
    assert!(d.contains("starting at line 4"), "exact unreachable line expected: {out:?}");
}

#[test]
fn fully_reachable_branches_stay_silent() {
    let src = "def f(c):\n    if c:\n        return 1\n    else:\n        return 2\n";
    let out = findings_of(src, Language::Python, "py", "");
    assert_eq!(count(&out, Smell::UnreachableCode), 0, "both-return if/else has no unreachable code: {out:?}");
}

#[test]
fn cross_block_definitions_reach_their_uses() {
    let src = "def f(c):\n    x = 1\n    if c:\n        x = 2\n    return x\n";
    let out = findings_of(src, Language::Python, "py", "");
    assert_eq!(count(&out, Smell::UseBeforeDef), 0, "defs reaching across blocks must suppress: {out:?}");
}

#[test]
fn parameter_use_before_late_reassignment_is_not_flagged() {
    let src = "def f(x):\n    y = x\n    x = 2\n    return y\n";
    let out = findings_of(src, Language::Python, "py", "");
    assert_eq!(count(&out, Smell::UseBeforeDef), 0, "parameters are definitions at entry: {out:?}");
}

#[test]
fn every_unreached_use_is_reported_separately() {
    let src = "def f(c):\n    if c:\n        w = z\n    b = z\n    z = 1\n    return b\n";
    let out = findings_of(src, Language::Python, "py", "");
    assert_eq!(count(&out, Smell::UseBeforeDef), 2, "both early reads of z precede its definition: {out:?}");
    let d = details(&out, Smell::UseBeforeDef);
    assert!(d.contains("line 3") && d.contains("line 4"), "both use lines reported: {d}");
}

#[test]
fn dead_store_lines_are_reported_exactly() {
    let src = "fun f(): String {\n    var keep = 1\n    keep = 2\n    var gone = 3\n    gone = 4\n    return \"the gone flag: $keep\"\n}\n";
    let out = findings_of(src, Language::Kotlin, "kt", "");
    let d = details(&out, Smell::DeadStore);
    assert!(d.contains("`gone` assigned at line 4"), "the overwritten store must be flagged with its line: {out:?}");
    assert!(!d.contains("`keep` assigned at line 3"), "the template-read store must stay alive: {out:?}");
}

#[test]
fn plain_words_in_strings_do_not_keep_stores_alive() {
    let src = "fun f(): String {\n    var keep = 1\n    keep = 2\n    var gone = 3\n    gone = 4\n    return \"the gone flag: $keep\"\n}\n";
    let out = findings_of(src, Language::Kotlin, "kt", "");
    assert!(count(&out, Smell::DeadStore) >= 2, "non-template mention of `gone` must not suppress: {out:?}");
}

#[test]
fn dead_store_detector_honors_its_own_toggle() {
    let src = "fun f(): String {\n    var keep = 1\n    keep = 2\n    var gone = 3\n    gone = 4\n    return \"the gone flag: $keep\"\n}\n";
    let out = findings_of(src, Language::Kotlin, "kt", "dead_store = false\n");
    assert_eq!(count(&out, Smell::DeadStore), 0, "disabled detector must stay silent: {out:?}");
}

#[test]
fn when_arm_bodies_are_analyzed() {
    let src = "fun f(x: Int) {\n    when (x) {\n        1 -> {\n            var t = 9\n            t = 8\n        }\n        else -> { }\n    }\n}\n";
    let out = findings_of(src, Language::Kotlin, "kt", "");
    let d = details(&out, Smell::DeadStore);
    assert!(d.contains("`t` assigned at line 4"), "an overwrite inside a when arm must be visible: {out:?}");
}

#[test]
fn java_declaration_without_initializer_is_not_a_phantom_use() {
    let src = "class A {\n    void f() {\n        int x;\n        x = 5;\n        System.out.println(x);\n    }\n}\n";
    let out = findings_of(src, Language::Java, "java", "");
    assert_eq!(count(&out, Smell::UseBeforeDef), 0, "bare declaration is not a read: {out:?}");
    assert_eq!(count(&out, Smell::DeadStore), 0, "assigned-then-read is not dead: {out:?}");
}

#[test]
fn java_compound_assignment_reads_the_prior_store() {
    let src =
        "class B {\n    void f() {\n        int x = 1;\n        x += 1;\n        System.out.println(x);\n    }\n}\n";
    let out = findings_of(src, Language::Java, "java", "");
    assert_eq!(count(&out, Smell::DeadStore), 0, "compound assignment reads its target: {out:?}");
}
