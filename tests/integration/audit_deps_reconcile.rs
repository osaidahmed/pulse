use pulse::audit::finding::{AuditFinding, AuditKind};
use pulse::audit::{self, AuditOpts, PassChoice};
use pulse::config::AuditSuppression;
use std::path::Path;

use crate::audit_common::t;

fn run_deps(root: &Path) -> Vec<AuditFinding> {
    let opts = AuditOpts {
        root: root.to_path_buf(),
        pass: Some(PassChoice::Deps),
        json: false,
        include_tests: false,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    audit::run(&opts, &t().audit)
}

fn bloated_names(findings: &[AuditFinding]) -> Vec<String> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::BloatedDependency(e) => Some(e.name.clone()),
            _ => None,
        })
        .collect()
}

fn phantom_names(findings: &[AuditFinding]) -> Vec<String> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::PhantomDependency(e) => Some(e.name.clone()),
            _ => None,
        })
        .collect()
}

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, body) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }
    dir
}

#[test]
fn ruby_gems_are_not_flagged_bloated() {
    let dir = project(&[
        ("Gemfile", "source 'https://rubygems.org'\ngem 'rails'\ngem 'nokogiri'\n"),
        ("app/main.rb", "puts 'hello'\n"),
    ]);
    assert!(
        bloated_names(&run_deps(dir.path())).is_empty(),
        "Bundler.require auto-loads gems, so a never-`require`d gem is not verifiably bloated"
    );
}

#[test]
fn cargo_unused_dependency_is_bloated() {
    let dir = project(&[
        (
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\nstale-helper = \"2\"\n",
        ),
        ("src/lib.rs", "use serde::Serialize;\n\npub fn f() {}\n"),
    ]);
    let findings = run_deps(dir.path());
    assert_eq!(bloated_names(&findings), vec!["stale-helper"], "{findings:?}");
    assert!(phantom_names(&findings).is_empty(), "{findings:?}");
}

#[test]
fn macro_only_usage_is_rescued_by_the_token_scan() {
    let dir = project(&[
        ("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\ntracing = \"0.1\"\n"),
        ("src/lib.rs", "pub fn f() {\n    tracing::info!(\"hi\");\n}\n"),
    ]);
    let findings = run_deps(dir.path());
    assert!(bloated_names(&findings).is_empty(), "macro-path usage counts: {findings:?}");
}

#[test]
fn dev_and_build_scopes_are_exempt_from_bloat() {
    let dir = project(&[
        (
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dev-dependencies]\ncriterion = \"0.5\"\n\n[build-dependencies]\ncc = \"1\"\n",
        ),
        ("src/lib.rs", "pub fn f() {}\n"),
    ]);
    let findings = run_deps(dir.path());
    assert!(bloated_names(&findings).is_empty(), "dev and build scopes are out of slice scope: {findings:?}");
}

#[test]
fn own_package_name_is_never_bloated() {
    let dir = project(&[
        ("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\n"),
        ("src/lib.rs", "pub fn f() {}\n"),
    ]);
    let findings = run_deps(dir.path());
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn npm_lockfile_only_import_is_phantom() {
    let dir = project(&[
        ("package.json", r#"{"name": "demo", "dependencies": {"express": "^4.0.0"}}"#),
        (
            "package-lock.json",
            r#"{"lockfileVersion": 3, "packages": {"node_modules/express": {"version": "4.18.2"}, "node_modules/qs": {"version": "6.11.0"}}}"#,
        ),
        ("svc.ts", "import express from \"express\";\nimport qs from \"qs\";\nimport ws from \"ws\";\n"),
    ]);
    let findings = run_deps(dir.path());
    assert_eq!(phantom_names(&findings), vec!["qs"], "only lockfile-resolved undeclared imports: {findings:?}");
    assert!(bloated_names(&findings).is_empty(), "express is imported: {findings:?}");
}

#[test]
fn python_alias_import_keeps_the_declared_package() {
    let dir = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\ndependencies = [\"scikit-learn\"]\n"),
        ("svc.py", "import sklearn\n"),
    ]);
    let findings = run_deps(dir.path());
    assert!(bloated_names(&findings).is_empty(), "sklearn import covers scikit-learn: {findings:?}");
}

#[test]
fn go_subpath_import_covers_the_declared_module() {
    let dir = project(&[
        (
            "go.mod",
            "module example.com/demo\n\ngo 1.22\n\nrequire (\n\tgithub.com/spf13/cobra v1.8.0\n\tgolang.org/x/extra v0.1.0\n)\n",
        ),
        ("main.go", "package main\n\nimport \"github.com/spf13/cobra/doc\"\n\nfunc main() { doc.Run() }\n"),
    ]);
    let findings = run_deps(dir.path());
    assert_eq!(bloated_names(&findings), vec!["golang.org/x/extra"], "{findings:?}");
}

#[test]
fn fully_consistent_project_is_silent() {
    let dir = project(&[
        ("package.json", r#"{"name": "demo", "dependencies": {"express": "^4.0.0"}}"#),
        ("svc.ts", "import express from \"express\";\n"),
    ]);
    let findings = run_deps(dir.path());
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn projects_without_manifests_produce_nothing() {
    let dir = project(&[("svc.py", "import anything\n")]);
    let findings = run_deps(dir.path());
    assert!(findings.is_empty(), "{findings:?}");
}

const CSPROJ: &str = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <ItemGroup>\n    <PackageReference Include=\"Newtonsoft.Json\" Version=\"13.0.1\" />\n    <PackageReference Include=\"Stale.Package\" Version=\"2.0.0\" />\n  </ItemGroup>\n</Project>\n";

#[test]
fn csharp_unused_dependency_is_bloated() {
    let dir = project(&[
        ("App.csproj", CSPROJ),
        ("Program.cs", "using Newtonsoft.Json;\nclass P { static void Main() {} }\n"),
    ]);
    let findings = run_deps(dir.path());
    assert_eq!(bloated_names(&findings), vec!["Stale.Package"], "{findings:?}");
}

#[test]
fn csharp_subnamespace_import_covers_the_package() {
    let csproj = "<Project>\n  <ItemGroup>\n    <PackageReference Include=\"Microsoft.Extensions.Logging\" Version=\"8.0.0\" />\n  </ItemGroup>\n</Project>\n";
    let dir = project(&[
        ("App.csproj", csproj),
        ("Program.cs", "using Microsoft.Extensions.Logging.Abstractions;\nclass P {}\n"),
    ]);
    let findings = run_deps(dir.path());
    assert!(bloated_names(&findings).is_empty(), "a sub-namespace import satisfies the package: {findings:?}");
}

#[test]
fn csharp_project_in_a_subdirectory_is_discovered() {
    let dir = project(&[("src/App.csproj", CSPROJ), ("src/Program.cs", "using Newtonsoft.Json;\nclass P {}\n")]);
    let findings = run_deps(dir.path());
    assert_eq!(bloated_names(&findings), vec!["Stale.Package"], "a .csproj in a subdirectory is found: {findings:?}");
}

#[test]
fn csharp_commented_out_package_is_not_bloated() {
    let csproj = "<Project>\n  <ItemGroup>\n    <!-- <PackageReference Include=\"Removed.Package\" Version=\"1.0.0\" /> -->\n    <PackageReference Include=\"Newtonsoft.Json\" Version=\"13.0.1\" />\n  </ItemGroup>\n</Project>\n";
    let dir = project(&[("App.csproj", csproj), ("Program.cs", "using Newtonsoft.Json;\nclass P {}\n")]);
    let findings = run_deps(dir.path());
    assert!(bloated_names(&findings).is_empty(), "a commented-out PackageReference is not a real dep: {findings:?}");
}

#[test]
fn csharp_abstraction_package_is_covered_by_the_shorter_namespace() {
    let csproj = "<Project>\n  <ItemGroup>\n    <PackageReference Include=\"Microsoft.Extensions.DependencyInjection.Abstractions\" Version=\"8.0.0\" />\n  </ItemGroup>\n</Project>\n";
    let dir = project(&[
        ("App.csproj", csproj),
        ("Program.cs", "using Microsoft.Extensions.DependencyInjection;\nclass P {}\n"),
    ]);
    let findings = run_deps(dir.path());
    assert!(bloated_names(&findings).is_empty(), "an import shorter than the package id still covers it: {findings:?}");
}

#[test]
fn csharp_divergent_namespace_family_is_aliased() {
    let csproj = "<Project>\n  <ItemGroup>\n    <PackageReference Include=\"AWSSDK.S3\" Version=\"3.7.0\" />\n  </ItemGroup>\n</Project>\n";
    let dir = project(&[("App.csproj", csproj), ("Program.cs", "using Amazon.S3;\nclass P {}\n")]);
    let findings = run_deps(dir.path());
    assert!(bloated_names(&findings).is_empty(), "AWSSDK.* maps to the Amazon.* namespace: {findings:?}");
}

#[test]
fn swift_unused_dependency_is_bloated() {
    let manifest = "// swift-tools-version:5.9\nimport PackageDescription\nlet package = Package(\n    name: \"Demo\",\n    dependencies: [\n        .package(url: \"https://github.com/Alamofire/Alamofire.git\", from: \"5.8.0\"),\n        .package(url: \"https://github.com/acme/UnusedKit.git\", from: \"1.0.0\"),\n    ]\n)\n";
    let dir = project(&[("Package.swift", manifest), ("Sources/Demo/main.swift", "import Alamofire\n\nfunc f() {}\n")]);
    let findings = run_deps(dir.path());
    let bloated = bloated_names(&findings);
    assert!(bloated.contains(&"UnusedKit".to_string()), "an unused swift dependency is bloated: {bloated:?}");
    assert!(!bloated.contains(&"Alamofire".to_string()), "an imported swift dependency is not bloated: {bloated:?}");
}

fn strictness_files(findings: &[AuditFinding]) -> Vec<String> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::StrictnessDebt(e) => Some(e.file.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

fn ts_with_anys(n: usize) -> String {
    let mut s = String::from("export function f() {\n");
    for i in 0..n {
        s.push_str(&format!("  const v{i}: any = 0;\n"));
    }
    s.push_str("}\n");
    s
}

#[test]
fn typescript_any_density_under_nonstrict_tsconfig_is_flagged() {
    let min = t().audit.strictness.any_min as usize;
    let dir = project(&[
        ("tsconfig.json", "{\n  \"compilerOptions\": { \"strict\": false }\n}\n"),
        ("src/app.ts", &ts_with_anys(min + 4)),
    ]);
    let files = strictness_files(&run_deps(dir.path()));
    assert!(
        files.iter().any(|f| f.ends_with("app.ts")),
        "high any-density under non-strict tsconfig is flagged: {files:?}"
    );
}

#[test]
fn typescript_strict_tsconfig_suppresses_strictness_debt() {
    let min = t().audit.strictness.any_min as usize;
    let dir = project(&[
        ("tsconfig.json", "{\n  \"compilerOptions\": { \"strict\": true }\n}\n"),
        ("src/app.ts", &ts_with_anys(min + 4)),
    ]);
    assert!(strictness_files(&run_deps(dir.path())).is_empty(), "strict projects are not flagged for any-debt");
}

#[test]
fn typescript_low_any_density_is_not_strictness_debt() {
    let dir = project(&[
        ("tsconfig.json", "{\n  \"compilerOptions\": { \"strict\": false }\n}\n"),
        ("src/app.ts", &ts_with_anys(1)),
    ]);
    assert!(strictness_files(&run_deps(dir.path())).is_empty(), "a single any is below the debt threshold");
}

#[test]
fn typescript_without_tsconfig_has_no_strictness_basis() {
    let min = t().audit.strictness.any_min as usize;
    let dir = project(&[("src/app.ts", &ts_with_anys(min + 4))]);
    assert!(strictness_files(&run_deps(dir.path())).is_empty(), "no tsconfig means no basis to judge strictness");
}
