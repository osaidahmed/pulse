use pulse::audit::corpus_df;
use pulse::audit::scoring::corpus_idiom_suppressed;
use pulse::parse::Language;

#[test]
fn empty_sketch_estimates_zero() {
    let df = corpus_df::empty();
    assert!(corpus_df::is_empty(&df));
    assert_eq!(corpus_df::estimate(&df, 7, 0xABCD), 0);
    assert!(corpus_df::file_frequency(&df, Language::Python, 0xABCD) < f64::EPSILON);
}

#[test]
fn counts_added_fingerprints() {
    let mut df = corpus_df::with_dims(4, 1024);
    for _ in 0..5 {
        corpus_df::add(&mut df, 3, 0x1111);
    }
    assert!(corpus_df::estimate(&df, 3, 0x1111) >= 5);
    assert_eq!(corpus_df::estimate(&df, 3, 0x2222), 0);
}

#[test]
fn frequency_normalizes_by_language_total() {
    let mut df = corpus_df::with_dims(4, 1024);
    let tag = corpus_df::lang_tag(Language::Rust);
    corpus_df::set_lang_total(&mut df, tag, 100);
    for _ in 0..20 {
        corpus_df::add(&mut df, tag, 0xDEAD);
    }
    assert!((corpus_df::file_frequency(&df, Language::Rust, 0xDEAD) - 0.2).abs() < 1e-9);
    assert!(corpus_df::file_frequency(&df, Language::Python, 0xDEAD) < f64::EPSILON);
}

#[test]
fn bytes_round_trip_preserves_estimates_and_totals() {
    let mut df = corpus_df::with_dims(4, 256);
    corpus_df::set_lang_total(&mut df, 1, 50);
    corpus_df::set_lang_total(&mut df, 2, 70);
    corpus_df::add(&mut df, 1, 0xCAFE);
    let restored = corpus_df::from_bytes(&corpus_df::to_bytes(&df)).expect("valid blob");
    assert_eq!(corpus_df::estimate(&restored, 1, 0xCAFE), corpus_df::estimate(&df, 1, 0xCAFE));
    let freq_diff = corpus_df::file_frequency(&restored, Language::Rust, 0xCAFE)
        - corpus_df::file_frequency(&df, Language::Rust, 0xCAFE);
    assert!(freq_diff.abs() < 1e-9);
}

#[test]
fn to_bytes_is_deterministic() {
    let mut df = corpus_df::with_dims(4, 256);
    corpus_df::set_lang_total(&mut df, 9, 3);
    corpus_df::set_lang_total(&mut df, 1, 8);
    corpus_df::set_lang_total(&mut df, 5, 5);
    assert_eq!(corpus_df::to_bytes(&df), corpus_df::to_bytes(&df));
}

#[test]
fn placeholder_round_trips_to_empty() {
    let restored = corpus_df::from_bytes(&corpus_df::to_bytes(&corpus_df::empty())).expect("placeholder parses");
    assert!(corpus_df::is_empty(&restored));
}

#[test]
fn rejects_garbage_blobs() {
    assert!(corpus_df::from_bytes(b"nope").is_none());
    assert!(corpus_df::from_bytes(&[]).is_none());
}

#[test]
fn corpus_idiom_suppression_is_off_by_default_and_config_opts_in() {
    let base = pulse::thresholds::Thresholds::default();
    assert_eq!(base.audit.pattern_mining.corpus_idiom_frequency, None);

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".pulse.toml"), "[thresholds.pattern_mining]\ncorpus_idiom_frequency = 0.1\n")
        .unwrap();
    let cfg = pulse::config::load_config(dir.path()).expect("config loads");
    let resolved = pulse::config::resolve_thresholds(Some(&cfg), Language::Rust);
    assert_eq!(resolved.audit.pattern_mining.corpus_idiom_frequency, Some(0.1));
}

#[test]
fn baked_sketch_is_populated() {
    let df = corpus_df::corpus_df();
    assert!(!corpus_df::is_empty(df), "the shipped sketch should be baked from the calibration corpus");
    assert!(!corpus_idiom_suppressed(df, Language::Rust, 0xABCD_EF12_3456_789A, None));
}

#[test]
fn none_threshold_never_suppresses_even_a_saturated_pattern() {
    let mut df = corpus_df::with_dims(4, 1024);
    let tag = corpus_df::lang_tag(Language::Rust);
    corpus_df::set_lang_total(&mut df, tag, 100);
    for _ in 0..95 {
        corpus_df::add(&mut df, tag, 0xFEED);
    }
    assert!(!corpus_idiom_suppressed(&df, Language::Rust, 0xFEED, None));
}

#[test]
fn corpus_common_pattern_is_suppressed_above_threshold() {
    let mut df = corpus_df::with_dims(4, 1024);
    let tag = corpus_df::lang_tag(Language::Rust);
    corpus_df::set_lang_total(&mut df, tag, 100);
    for _ in 0..60 {
        corpus_df::add(&mut df, tag, 0xFEED);
    }
    assert!(corpus_idiom_suppressed(&df, Language::Rust, 0xFEED, Some(0.5)));
}

#[test]
fn corpus_rare_pattern_survives_the_threshold() {
    let mut df = corpus_df::with_dims(4, 1024);
    let tag = corpus_df::lang_tag(Language::Rust);
    corpus_df::set_lang_total(&mut df, tag, 100);
    for _ in 0..2 {
        corpus_df::add(&mut df, tag, 0xFEED);
    }
    assert!(!corpus_idiom_suppressed(&df, Language::Rust, 0xFEED, Some(0.5)));
}
