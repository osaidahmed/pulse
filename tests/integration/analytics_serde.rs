use pulse::analytics::{FindingRecord, OutcomeRecord};

fn sample_finding() -> FindingRecord {
    FindingRecord {
        ts: 1_700_000_000,
        file: "svc.rs".into(),
        path: "/repo/svc.rs".into(),
        lang: Some("rust".into()),
        smell: "god_method".into(),
        tier: "error".into(),
        function: Some("run".into()),
        line: Some(40),
        hash: Some(9_999),
        detail: "cc 21".into(),
        rarity: None,
    }
}

// ── positive paths ──

#[test]
fn finding_record_round_trips() {
    let r = sample_finding();
    let json = serde_json::to_string(&r).unwrap();
    let back: FindingRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn finding_record_serializes_the_fn_key_not_function() {
    let json = serde_json::to_string(&sample_finding()).unwrap();
    assert!(json.contains(r#""fn":"run""#), "expected renamed key, got {json}");
    assert!(!json.contains("function"));
}

#[test]
fn a_findings_jsonl_line_deserializes_into_a_typed_record() {
    // exactly the shape log_findings writes
    let line = r#"{"ts":1,"file":"x.rs","path":"/x.rs","lang":"rust","smell":"god_method","tier":"error","fn":"foo","line":10,"hash":123,"detail":"cc","rarity":null}"#;
    let r: FindingRecord = serde_json::from_str(line).unwrap();
    assert_eq!(r.function.as_deref(), Some("foo"));
    assert_eq!(r.line, Some(10));
    assert_eq!(r.hash, Some(123));
    assert_eq!(r.rarity, None);
}

#[test]
fn outcome_record_round_trips() {
    let r = OutcomeRecord {
        ts: 5,
        session: "abc".into(),
        file: "x.rs".into(),
        lang: None,
        smell: "large_method".into(),
        tier: "warning".into(),
        function: None,
        detail: "d".into(),
        rarity: None,
        outcome: "addressed".into(),
    };
    let json = serde_json::to_string(&r).unwrap();
    let back: OutcomeRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

// ── negative / robustness paths ──

#[test]
fn malformed_line_errors() {
    assert!(serde_json::from_str::<FindingRecord>("{broken").is_err());
}

#[test]
fn missing_required_field_errors() {
    // no `smell`
    assert!(
        serde_json::from_str::<FindingRecord>(r#"{"ts":1,"file":"x","path":"/x","tier":"e","detail":"d"}"#).is_err()
    );
}

#[test]
fn old_lines_missing_optional_fields_still_read_back_as_none() {
    // a record written before fn/lang/line/hash/rarity existed — Option fields default to None
    let line = r#"{"ts":1,"file":"x","path":"/x","smell":"god_method","tier":"error","detail":"d"}"#;
    let r: FindingRecord = serde_json::from_str(line).unwrap();
    assert_eq!(r.lang, None);
    assert_eq!(r.function, None);
    assert_eq!(r.line, None);
    assert_eq!(r.hash, None);
    assert_eq!(r.rarity, None);
}

#[test]
fn unknown_extra_field_is_tolerated() {
    let line = r#"{"ts":1,"file":"x","path":"/x","smell":"s","tier":"t","detail":"d","future_col":true}"#;
    let r: FindingRecord = serde_json::from_str(line).unwrap();
    assert_eq!(r.smell, "s");
}
