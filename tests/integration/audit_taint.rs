use std::path::Path;

use pulse::audit::finding::{AuditFinding, AuditKind, ImportConfidence, InjectionEvidence};
use pulse::audit::{self, AuditOpts, IgnoreFilter, PassChoice};
use pulse::config::{AuditSuppression, IgnoreMatcher};

use crate::audit_common::*;

fn run_pass(dir: &Path, pass: Option<PassChoice>) -> Vec<AuditFinding> {
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let opts = AuditOpts {
        root: dir.to_path_buf(),
        pass,
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let filter = IgnoreFilter::new(&matcher, dir);
    audit::run_with_filter(&opts, &t().audit, &filter)
}

fn taint_findings(name: &str, body: &str) -> Vec<InjectionEvidence> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(name), body).unwrap();
    let findings = run_pass(dir.path(), Some(PassChoice::Taint));
    findings
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::InjectionShape(e) => Some(e),
            _ => None,
        })
        .collect()
}

#[test]
fn unsanitized_source_to_sink_is_flagged() {
    let body = "def handler(cursor):\n    q = input()\n    cursor.execute(q)\n";
    let found = taint_findings("app.py", body);
    assert_eq!(found.len(), 1, "expected one injection shape");
    let e = &found[0];
    assert_eq!(e.source_name, "input");
    assert_eq!(e.sink_name, "execute");
    assert_eq!(e.tainted_var, "q");
    assert!(!e.crossed_opacity);
    assert_eq!(e.confidence, ImportConfidence::Low);
}

#[test]
fn sanitized_flow_is_not_flagged() {
    let body = "def handler(cursor):\n    q = input()\n    safe = escape(q)\n    cursor.execute(safe)\n";
    assert!(taint_findings("app.py", body).is_empty());
}

#[test]
fn redefinition_clears_taint() {
    let body = "def handler(cursor):\n    q = input()\n    q = \"SELECT 1\"\n    cursor.execute(q)\n";
    assert!(taint_findings("app.py", body).is_empty());
}

#[test]
fn source_with_no_sink_is_not_flagged() {
    let body = "def handler():\n    q = input()\n    return q\n";
    assert!(taint_findings("app.py", body).is_empty());
}

#[test]
fn direct_source_into_sink_is_flagged() {
    let body = "import os\n\ndef handler():\n    os.system(input())\n";
    let found = taint_findings("app.py", body);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source_name, "input");
    assert_eq!(found[0].sink_name, "system");
}

#[test]
fn opacity_keeps_flow_as_unknown_not_clean() {
    let body = "def handler(cursor):\n    q = input()\n    payload = build(**q)\n    cursor.execute(payload)\n";
    let found = taint_findings("app.py", body);
    assert_eq!(found.len(), 1, "opacity must not silently drop the flow");
    assert!(found[0].crossed_opacity);
    assert_eq!(found[0].confidence, ImportConfidence::BestEffort);
}

#[test]
fn sanitizer_cannot_clean_an_opaque_flow() {
    let body = "def handler(cursor):\n    q = input()\n    payload = build(**q)\n    clean = escape(payload)\n    cursor.execute(clean)\n";
    let found = taint_findings("app.py", body);
    assert_eq!(found.len(), 1, "a sanitizer must not be trusted across opacity (A6)");
    assert!(found[0].crossed_opacity);
    assert_eq!(found[0].confidence, ImportConfidence::BestEffort);
}

#[test]
fn clean_function_has_no_findings() {
    let body = "def compute(a, b):\n    total = a + b\n    return total * 2\n";
    assert!(taint_findings("app.py", body).is_empty());
}

#[test]
fn rust_unsanitized_source_to_sink_is_flagged() {
    let body = "fn handle(conn: &Conn) {\n    let q = var(\"INPUT\");\n    conn.execute(q);\n}\n";
    let found = taint_findings("app.rs", body);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source_name, "var");
    assert_eq!(found[0].sink_name, "execute");
}

#[test]
fn java_unsanitized_source_to_sink_is_flagged() {
    let body = "class C {\n  void handler(Statement stmt, Request req) {\n    String q = req.getParameter(\"id\");\n    stmt.executeQuery(q);\n  }\n}\n";
    let found = taint_findings("App.java", body);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source_name, "getParameter");
    assert_eq!(found[0].sink_name, "executeQuery");
    assert_eq!(found[0].tainted_var, "q");
}

#[test]
fn java_sanitized_flow_is_not_flagged() {
    let body = "class C {\n  void handler(Statement stmt, Request req) {\n    String q = req.getParameter(\"id\");\n    String safe = encode(q);\n    stmt.executeQuery(safe);\n  }\n}\n";
    assert!(taint_findings("App.java", body).is_empty(), "encode() sanitizes the flow");
}

#[test]
fn go_unsanitized_source_to_sink_is_flagged() {
    let body = "package main\nfunc handler(db DB, r Req) {\n\tq := r.FormValue(\"id\")\n\tdb.Query(q)\n}\n";
    let found = taint_findings("app.go", body);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source_name, "FormValue");
    assert_eq!(found[0].sink_name, "Query");
    assert_eq!(found[0].tainted_var, "q");
}

#[test]
fn go_sanitized_flow_is_not_flagged() {
    let body = "package main\nfunc handler(db DB, r Req) {\n\tq := r.FormValue(\"id\")\n\tsafe := Escape(q)\n\tdb.Query(safe)\n}\n";
    assert!(taint_findings("app.go", body).is_empty(), "Escape() sanitizes the flow");
}

#[test]
fn typescript_request_property_source_to_sink_is_flagged() {
    let body = "function handler(db, req) {\n  const q = req.query;\n  db.query(q);\n}\n";
    let found = taint_findings("app.ts", body);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source_name, "query");
    assert_eq!(found[0].sink_name, "query");
    assert_eq!(found[0].tainted_var, "q");
    assert_eq!(found[0].confidence, ImportConfidence::Low);
}

#[test]
fn typescript_sanitized_flow_is_not_flagged() {
    let body = "function handler(db, req) {\n  const q = req.body;\n  const safe = escape(q);\n  db.query(safe);\n}\n";
    assert!(taint_findings("app.ts", body).is_empty(), "escape() sanitizes the flow");
}

#[test]
fn typescript_method_callee_is_not_a_phantom_source() {
    let body = "function handler(db, other, x) {\n  const r = db.query(x);\n  other.exec(r);\n}\n";
    assert!(taint_findings("app.ts", body).is_empty(), "a `.query` in callee position is a method, not a source");
}

#[test]
fn javascript_request_property_source_to_sink_is_flagged() {
    let body = "function handler(db, req) {\n  var q = req.body;\n  db.query(q);\n}\n";
    let found = taint_findings("app.js", body);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source_name, "body");
    assert_eq!(found[0].sink_name, "query");
}

#[test]
fn csharp_request_property_source_to_sink_is_flagged() {
    let body = "class C {\n  void Handler(SqlCommand cmd) {\n    var q = Request.Form[\"id\"];\n    cmd.ExecuteReader(q);\n  }\n}\n";
    let found = taint_findings("App.cs", body);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source_name, "Form");
    assert_eq!(found[0].sink_name, "ExecuteReader");
    assert_eq!(found[0].tainted_var, "q");
}

#[test]
fn csharp_sanitized_flow_is_not_flagged() {
    let body = "class C {\n  void Handler(SqlCommand cmd) {\n    var q = Request.QueryString[\"id\"];\n    var safe = Encode(q);\n    cmd.ExecuteReader(safe);\n  }\n}\n";
    assert!(taint_findings("App.cs", body).is_empty(), "Encode() sanitizes the flow");
}

#[test]
fn php_superglobal_source_to_sink_is_flagged() {
    let body = "<?php\nfunction handler($db) {\n  $q = $_GET['id'];\n  $db->query($q);\n}\n";
    let found = taint_findings("app.php", body);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source_name, "$_GET");
    assert_eq!(found[0].sink_name, "query");
    assert_eq!(found[0].tainted_var, "$q");
}

#[test]
fn php_sanitized_flow_is_not_flagged() {
    let body = "<?php\nfunction handler($db, $conn) {\n  $q = $_POST['id'];\n  $safe = mysqli_real_escape_string($conn, $q);\n  $db->query($safe);\n}\n";
    assert!(taint_findings("app.php", body).is_empty(), "mysqli_real_escape_string sanitizes the flow");
}

#[test]
fn taint_is_opt_in_not_in_default_passes() {
    let dir = tempfile::tempdir().unwrap();
    let body = "def handler(cursor):\n    q = input()\n    cursor.execute(q)\n";
    std::fs::write(dir.path().join("app.py"), body).unwrap();
    let default_findings = run_pass(dir.path(), None);
    let any_injection = default_findings.iter().any(|f| matches!(f.kind, AuditKind::InjectionShape(_)));
    assert!(!any_injection, "taint must not run in the default (All) pass");
}
