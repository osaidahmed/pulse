use std::path::Path;
use std::process::Command;

struct Project {
    dir: tempfile::TempDir,
    baseline: tempfile::TempDir,
}

impl Project {
    fn new(manifest_name: &str, manifest_body: &str) -> Self {
        let p = Self { dir: tempfile::tempdir().unwrap(), baseline: tempfile::tempdir().unwrap() };
        std::fs::write(p.dir.path().join(manifest_name), manifest_body).unwrap();
        p
    }

    fn write(&self, name: &str, body: &str) -> std::path::PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    fn check(&self, path: &Path) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
            .args(["check", path.to_str().unwrap()])
            .env("PULSE_BASELINE_DIR", self.baseline.path())
            .output()
            .expect("run pulse check");
        String::from_utf8(output.stdout).unwrap()
    }
}

fn flags_over_injection(out: &str) -> bool {
    out.contains("Constructor Over-Injection")
}

fn service(deps: usize) -> String {
    let params: Vec<String> = (0..deps).map(|i| format!("private s{i}: Svc{i}")).collect();
    format!("export class Widget {{\n  constructor({}) {{}}\n}}\n", params.join(", "))
}

#[test]
fn constructor_over_injection_fires_without_a_di_framework() {
    let p = Project::new("package.json", "{\"dependencies\": {\"express\": \"^4.0.0\"}}\n");
    let file = p.write("widget.ts", &service(5));
    let out = p.check(&file);
    assert!(flags_over_injection(&out), "5 injected deps must flag without a DI framework: {out}");
}

#[test]
fn idiomatic_injection_is_allowed_when_a_di_framework_is_declared() {
    let p = Project::new("package.json", "{\"dependencies\": {\"@nestjs/common\": \"^10.0.0\"}}\n");
    let file = p.write("widget.ts", &service(5));
    let out = p.check(&file);
    assert!(!flags_over_injection(&out), "idiomatic DI injection must not flag under a DI framework: {out}");
}

#[test]
fn a_transitive_di_dependency_does_not_relax_the_threshold() {
    let p = Project::new("package.json", "{\"dependencies\": {\"express\": \"^4.0.0\"}}\n");
    p.write(
        "package-lock.json",
        "{\"lockfileVersion\": 3, \"packages\": {\"node_modules/@nestjs/common\": {\"version\": \"10.0.0\"}}}\n",
    );
    let file = p.write("widget.ts", &service(5));
    let out = p.check(&file);
    assert!(flags_over_injection(&out), "a DI framework present only transitively must not relax the threshold: {out}");
}

#[test]
fn excessive_injection_still_fires_under_a_di_framework() {
    let p = Project::new("package.json", "{\"dependencies\": {\"@nestjs/common\": \"^10.0.0\"}}\n");
    let file = p.write("widget.ts", &service(9));
    let out = p.check(&file);
    assert!(flags_over_injection(&out), "genuinely excessive injection must still flag under a DI framework: {out}");
}
