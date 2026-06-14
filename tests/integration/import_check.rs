use std::io::Write;
use std::path::Path;
use std::process::Command;

struct Project {
    files_dir: tempfile::TempDir,
    baseline_dir: tempfile::TempDir,
}

impl Project {
    fn new(manifest_name: &str, manifest_body: &str) -> Self {
        let p = Self { files_dir: tempfile::tempdir().unwrap(), baseline_dir: tempfile::tempdir().unwrap() };
        std::fs::write(p.files_dir.path().join(manifest_name), manifest_body).unwrap();
        p
    }

    fn write(&self, name: &str, body: &str) -> std::path::PathBuf {
        let path = self.files_dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, body).unwrap();
        path
    }

    fn hook(&self, path: &Path) -> String {
        let input =
            serde_json::json!({"tool_input": {"file_path": path.to_str().unwrap()}, "tool_response": {"originalFile": ""}})
                .to_string();
        let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
            .args(["--hook"])
            .env("PULSE_BASELINE_DIR", self.baseline_dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
                child.wait_with_output()
            })
            .expect("failed to run pulse --hook");
        String::from_utf8(output.stdout).unwrap()
    }
}

fn flags_hallucination(out: &str) -> bool {
    out.to_lowercase().contains("hallucinated import")
}

#[test]
fn python_undeclared_import_blocks() {
    let p = Project::new("pyproject.toml", "[project]\nname = \"demo\"\ndependencies = [\"requests>=2.0\"]\n");
    let file = p.write("svc.py", "import reqeusts\n\nprint(reqeusts.get)\n");
    let out = p.hook(&file);
    assert!(flags_hallucination(&out), "typo'd package must block: {out}");
    assert!(out.contains("reqeusts"), "names the bogus package: {out}");
}

#[test]
fn python_declared_stdlib_and_internal_imports_stay_silent() {
    let p = Project::new("pyproject.toml", "[project]\nname = \"demo\"\ndependencies = [\"requests>=2.0\"]\n");
    p.write("myapp/__init__.py", "");
    let file = p.write("svc.py", "import os\nimport requests\nimport myapp\nfrom myapp.sub import thing\n");
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "declared, stdlib, and internal imports are fine: {out}");
}

#[test]
fn python_alias_import_maps_to_declared_package() {
    let p = Project::new("pyproject.toml", "[project]\nname = \"demo\"\ndependencies = [\"scikit-learn\"]\n");
    let file = p.write("svc.py", "import sklearn\n");
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "sklearn maps to scikit-learn: {out}");
}

#[test]
fn python_without_a_manifest_is_not_checked() {
    let p = Project::new("README.md", "no manifests here");
    let file = p.write("svc.py", "import totallymadeup\n");
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "no manifest means no basis to judge: {out}");
}

#[test]
fn python_preexisting_undeclared_import_is_not_novel() {
    let p = Project::new("pyproject.toml", "[project]\nname = \"demo\"\ndependencies = []\n");
    let file = p.write("svc.py", "import mystery\nx = 1\n");
    let input = serde_json::json!({
        "tool_input": {"file_path": file.to_str().unwrap()},
        "tool_response": {"originalFile": "import mystery\n"}
    })
    .to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .env("PULSE_BASELINE_DIR", p.baseline_dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("hook run");
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(!flags_hallucination(&out), "imports already present before the edit are not novel: {out}");
}

#[test]
fn npm_scoped_deep_and_builtin_imports_resolve() {
    let p = Project::new(
        "package.json",
        r#"{"name": "demo", "dependencies": {"express": "^4.0.0", "@scope/kit": "^1.0.0"}}"#,
    );
    let file = p.write(
        "svc.ts",
        "import express from \"express\";\nimport sub from \"express/lib/router\";\nimport kit from \"@scope/kit/util\";\nimport fs from \"node:fs\";\nimport local from \"./local\";\n",
    );
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "declared roots, deep paths, builtins, locals: {out}");
}

#[test]
fn npm_undeclared_package_blocks() {
    let p = Project::new("package.json", r#"{"name": "demo", "dependencies": {"express": "^4.0.0"}}"#);
    let file = p.write("svc.ts", "import thing from \"left-pad-ultra\";\n");
    let out = p.hook(&file);
    assert!(flags_hallucination(&out), "undeclared npm package must block: {out}");
}

#[test]
fn rust_declared_and_builtin_roots_resolve() {
    let p = Project::new(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nxxhash-rust = \"0.8\"\n",
    );
    let file = p.write(
        "src/lib.rs",
        "use std::collections::HashMap;\nuse xxhash_rust::xxh3;\nuse crate::other;\n\npub fn f() {}\n",
    );
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "std, declared (underscore form), and crate paths: {out}");
}

#[test]
fn rust_sibling_module_uniform_path_resolves() {
    let p = Project::new("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\n");
    p.write("src/walkers/booleans.rs", "pub fn x() {}\n");
    let file = p.write("src/walkers/mod.rs", "mod booleans;\nuse booleans::x;\n\npub fn f() { x() }\n");
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "sibling modules via uniform paths are internal: {out}");
}

#[test]
fn rust_undeclared_crate_blocks() {
    let p = Project::new("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\n");
    let file = p.write("src/lib.rs", "use figment_extras::Figment;\n\npub fn f() {}\n");
    let out = p.hook(&file);
    assert!(flags_hallucination(&out), "undeclared crate must block: {out}");
}

#[test]
fn go_module_prefixes_and_stdlib_resolve() {
    let p = Project::new("go.mod", "module example.com/demo\n\ngo 1.22\n\nrequire github.com/spf13/cobra v1.8.0\n");
    let file = p.write(
        "main.go",
        "package main\n\nimport (\n\t\"fmt\"\n\t\"github.com/spf13/cobra/doc\"\n\t\"example.com/demo/internal/db\"\n)\n\nfunc main() { fmt.Println(doc.Used, db.Used) }\n",
    );
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "stdlib, declared prefix, own module: {out}");
}

#[test]
fn go_undeclared_module_blocks() {
    let p = Project::new("go.mod", "module example.com/demo\n\ngo 1.22\n");
    let file =
        p.write("main.go", "package main\n\nimport \"github.com/nobody/ghost\"\n\nfunc main() { ghost.Run() }\n");
    let out = p.hook(&file);
    assert!(flags_hallucination(&out), "undeclared module must block: {out}");
}

#[test]
fn ruby_gems_stdlib_and_relative_requires_resolve() {
    let p = Project::new("Gemfile", "source \"https://rubygems.org\"\n\ngem \"rails\"\n");
    p.write("helper.rb", "def help\nend\n");
    let file = p.write(
        "svc.rb",
        "require \"rails\"\nrequire \"json\"\nrequire \"net/http\"\nrequire_relative \"helper\"\n\ndef f\nend\n",
    );
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "gem, stdlib, nested stdlib, relative: {out}");
}

#[test]
fn ruby_undeclared_gem_blocks() {
    let p = Project::new("Gemfile", "source \"https://rubygems.org\"\n\ngem \"rails\"\n");
    let file = p.write("svc.rb", "require \"hyperwarp\"\n\ndef f\nend\n");
    let out = p.hook(&file);
    assert!(flags_hallucination(&out), "undeclared gem must block: {out}");
}

#[test]
fn lockfile_only_transitive_packages_resolve() {
    let p = Project::new("package.json", r#"{"name": "demo", "dependencies": {"express": "^4.0.0"}}"#);
    std::fs::write(
        p.files_dir.path().join("package-lock.json"),
        r#"{"lockfileVersion": 3, "packages": {"node_modules/qs": {"version": "6.11.0"}}}"#,
    )
    .unwrap();
    let file = p.write("svc.ts", "import qs from \"qs\";\n");
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "lockfile transitive entries count as declared: {out}");
}

#[test]
fn csharp_blocking_hallucination_check_is_held() {
    let p = Project::new(
        "App.csproj",
        "<Project>\n  <ItemGroup>\n    <PackageReference Include=\"Newtonsoft.Json\" Version=\"13.0.1\" />\n  </ItemGroup>\n</Project>\n",
    );
    let file = p.write("Program.cs", "using Totally.Fake.Package;\nclass P {}\n");
    let out = p.hook(&file);
    assert!(!flags_hallucination(&out), "blocking hallucinated-import is held for C# pending corpus validation: {out}");
}
