#![allow(clippy::float_cmp)]

use pulse::calibrate::priors::{MetricPrior, PriorsBuilder, QUANTILE_PROBES};
use pulse::calibrate::stats::{count_quantile, weighted_quantile, Bin, WeightedHist};
use pulse::parse::Language;
use pulse::walk::{FileMetrics, FunctionMetrics, ModuleMetrics};

fn empty_module() -> ModuleMetrics {
    ModuleMetrics {
        total_loc: 0,
        total_functions: 0,
        sum_cc: 0,
        global_conditional_count: 0,
        global_max_nesting: 0,
        declaration_count: 0,
        struct_fields: Vec::new(),
    }
}

fn one_function() -> FunctionMetrics {
    FunctionMetrics { name: "alpha".to_string(), loc: 10, cc: 3, ..Default::default() }
}

#[test]
fn weighted_hist_observe_accumulates_count_and_weight() {
    let mut h = WeightedHist::default();
    h.observe(5, 2.0);
    h.observe(5, 3.0);
    h.observe(9, 4.0);
    assert_eq!(h.n, 3);
    assert_eq!(h.weight, 9.0);
    let five = h.bins.get(&5).expect("bin at 5");
    assert_eq!(five.count, 2);
    assert_eq!(five.weight, 5.0);
    assert_eq!(h.bins.get(&9).expect("bin at 9").count, 1);
}

#[test]
fn weighted_hist_merge_adds_disjoint_and_overlapping_bins() {
    let mut a = WeightedHist::default();
    a.observe(1, 1.0);
    a.observe(2, 2.0);
    let mut b = WeightedHist::default();
    b.observe(2, 4.0);
    b.observe(7, 8.0);
    a.merge(&b);
    assert_eq!(a.n, 4);
    assert_eq!(a.weight, 15.0);
    let two = a.bins.get(&2).expect("merged bin at 2");
    assert_eq!(two.count, 2, "counts from both histograms summed");
    assert_eq!(two.weight, 6.0);
    assert_eq!(a.bins.get(&7).expect("bin at 7").count, 1, "disjoint bin carried over");
}

#[test]
fn quantile_returns_zero_for_empty_distribution() {
    let h = WeightedHist::default();
    assert_eq!(count_quantile(&h, 0.5), 0.0, "no count mass");
    assert_eq!(weighted_quantile(&h, 0.5), 0.0, "no weight mass");
}

#[test]
fn count_quantile_zero_total_short_circuits_even_with_bins() {
    let mut h = WeightedHist::default();
    h.bins.insert(3, Bin { count: 0, weight: 0.0 });
    assert_eq!(h.n, 0, "n drives count_quantile total");
    assert_eq!(count_quantile(&h, 0.9), 0.0, "total<=0 returns 0 before scanning bins");
}

#[test]
fn weighted_quantile_zero_total_short_circuits_with_count_present() {
    let mut h = WeightedHist::default();
    h.bins.insert(4, Bin { count: 5, weight: 0.0 });
    h.n = 5;
    assert_eq!(h.weight, 0.0, "weight mass drives weighted_quantile total");
    assert_eq!(weighted_quantile(&h, 0.9), 0.0, "zero weight total returns 0");
}

#[test]
fn quantile_falls_back_to_last_key_when_mass_never_reaches_target() {
    let mut h = WeightedHist::default();
    h.bins.insert(3, Bin { count: 1, weight: 1.0 });
    h.bins.insert(11, Bin { count: 1, weight: 1.0 });
    h.n = 10;
    let q = count_quantile(&h, 0.95);
    assert_eq!(q, 11.0, "cumulative count (2) never reaches target (9.5) so last key wins");
}

#[test]
fn quantile_returns_first_bin_when_target_met_immediately() {
    let mut h = WeightedHist::default();
    h.observe(2, 1.0);
    h.observe(2, 1.0);
    h.observe(2, 1.0);
    h.observe(8, 1.0);
    assert_eq!(count_quantile(&h, 0.5), 2.0, "median sits in the dense low bin");
    assert_eq!(count_quantile(&h, 1.0), 8.0, "p=1 walks to the top bin");
}

#[test]
fn count_quantile_clamps_probe_below_zero_and_above_one() {
    let mut h = WeightedHist::default();
    h.observe(4, 1.0);
    h.observe(40, 1.0);
    assert_eq!(count_quantile(&h, -5.0), 4.0, "negative probe clamps to 0 -> first bin");
    assert_eq!(count_quantile(&h, 5.0), 40.0, "probe above 1 clamps to 1 -> last bin");
}

#[test]
fn metric_prior_quantile_empty_returns_zero() {
    let mp = MetricPrior { n: 0, quantiles: Vec::new(), cdf: Vec::new() };
    assert_eq!(mp.quantile(0.5), 0.0, "no probes yields the zero fallback");
}

#[test]
fn metric_prior_quantile_handles_duplicate_probe_spans() {
    let mp = MetricPrior { n: 50, quantiles: vec![(0.5, 4.0), (0.5, 9.0), (0.9, 20.0)], cdf: Vec::new() };
    assert_eq!(mp.quantile(0.5), 4.0, "p<=first_p clamps to the first probe value");
    let between = mp.quantile(0.7);
    assert!(between > 9.0 && between < 20.0, "interpolation uses the second of the duplicate pair: {between}");
}

#[test]
fn metric_prior_quantile_single_probe_is_flat() {
    let mp = MetricPrior { n: 7, quantiles: vec![(0.5, 12.0)], cdf: Vec::new() };
    assert_eq!(mp.quantile(0.3), 12.0, "below the lone probe clamps to it");
    assert_eq!(mp.quantile(0.5), 12.0, "at the lone probe");
    assert_eq!(mp.quantile(0.99), 12.0, "above the lone probe falls back to last value");
}

#[test]
fn cdf_at_zero_n_reports_certainty() {
    let mp = MetricPrior { n: 0, quantiles: Vec::new(), cdf: vec![(1, 3)] };
    assert_eq!(mp.cdf_at(5), 1.0, "empty sample treats everything as at-or-below");
    assert_eq!(mp.firing_rate(5), 0.0, "and nothing fires");
    assert_eq!(mp.percentile_of(5), 100.0);
}

#[test]
fn firing_rate_complements_the_cdf() {
    let mp = MetricPrior { n: 4, quantiles: Vec::new(), cdf: vec![(2, 1), (5, 1), (9, 2)] };
    assert_eq!(mp.cdf_at(5), 0.5);
    assert_eq!(mp.firing_rate(5), 0.5, "half the mass sits above the threshold");
    assert_eq!(mp.cdf_at(1), 0.0, "below the smallest bin nothing accumulates");
    assert_eq!(mp.firing_rate(1), 1.0);
}

#[test]
fn add_file_metrics_routes_main_versus_test_strata() {
    let mut builder = PriorsBuilder::default();
    let main = FileMetrics {
        functions: vec![one_function()],
        module: ModuleMetrics { total_loc: 10, total_functions: 1, sum_cc: 3, ..empty_module() },
    };
    let test = FileMetrics {
        functions: vec![FunctionMetrics { name: "test_alpha".to_string(), loc: 4, cc: 1, ..Default::default() }],
        module: ModuleMetrics { total_loc: 4, total_functions: 1, sum_cc: 1, ..empty_module() },
    };
    builder.add_file_metrics(Language::Python, false, &main);
    builder.add_file_metrics(Language::Python, true, &test);
    let table = builder.build(false);
    let main_cc = table.main.get("python").expect("main python").metrics.get("cc").expect("cc");
    let test_cc = table.tests.get("python").expect("test python").metrics.get("cc").expect("cc");
    assert_eq!(main_cc.n, 1);
    assert_eq!(test_cc.n, 1);
    assert_eq!(main_cc.quantiles.len(), QUANTILE_PROBES.len());
}

#[test]
fn add_file_metrics_records_struct_field_distribution() {
    let mut builder = PriorsBuilder::default();
    let metrics = FileMetrics {
        functions: Vec::new(),
        module: ModuleMetrics {
            total_loc: 30,
            struct_fields: vec![("Big".to_string(), 14), ("Small".to_string(), 2)],
            ..empty_module()
        },
    };
    builder.add_file_metrics(Language::Rust, false, &metrics);
    let table = builder.build(false);
    let sf = table.main.get("rust").expect("rust").metrics.get("struct_fields").expect("struct_fields");
    assert_eq!(sf.n, 2, "two structs observed");
    let cdf_total: u64 = sf.cdf.iter().map(|(_, c)| c).sum();
    assert_eq!(cdf_total, 2, "cdf carries both observations");
}

#[test]
fn add_file_metrics_floors_weight_when_loc_is_zero() {
    let mut builder = PriorsBuilder::default();
    let metrics = FileMetrics {
        functions: vec![FunctionMetrics { name: "z".to_string(), loc: 0, cc: 1, ..Default::default() }],
        module: empty_module(),
    };
    builder.add_file_metrics(Language::Go, false, &metrics);
    let table = builder.build(false);
    let cc = table.main.get("go").expect("go").metrics.get("cc").expect("cc");
    assert_eq!(cc.n, 1, "a zero-loc function is still observed with a floored weight");
}

#[test]
fn build_populates_cdf_alongside_quantiles() {
    let mut builder = PriorsBuilder::default();
    for cc in [1u32, 1, 5, 9] {
        let metrics = FileMetrics {
            functions: vec![FunctionMetrics { name: "f".to_string(), loc: 5, cc, ..Default::default() }],
            module: ModuleMetrics { total_loc: 5, total_functions: 1, sum_cc: cc, ..empty_module() },
        };
        builder.add_file_metrics(Language::Java, false, &metrics);
    }
    let table = builder.build(false);
    let cc = table.main.get("java").expect("java").metrics.get("cc").expect("cc");
    assert_eq!(cc.n, 4);
    assert_eq!(cc.cdf, vec![(1, 2), (5, 1), (9, 1)], "cdf is the sorted per-value count");
    assert_eq!(cc.cdf_at(9), 1.0);
}

#[test]
fn build_records_cpg_flag() {
    let builder = PriorsBuilder::default();
    assert!(!builder.build(false).cpg_enabled);
    assert!(builder.build(true).cpg_enabled);
}

#[test]
fn merge_combines_disjoint_languages() {
    let mut a = PriorsBuilder::default();
    a.add_file_metrics(
        Language::Python,
        false,
        &FileMetrics {
            functions: vec![one_function()],
            module: ModuleMetrics { total_loc: 10, total_functions: 1, sum_cc: 3, ..empty_module() },
        },
    );
    let mut b = PriorsBuilder::default();
    b.add_file_metrics(
        Language::Rust,
        true,
        &FileMetrics {
            functions: vec![FunctionMetrics { name: "r".to_string(), loc: 6, cc: 2, ..Default::default() }],
            module: ModuleMetrics { total_loc: 6, total_functions: 1, sum_cc: 2, ..empty_module() },
        },
    );
    a.merge(&b);
    let table = a.build(false);
    assert!(table.main.contains_key("python"), "main stratum kept");
    assert!(table.tests.contains_key("rust"), "test stratum merged in");
}

#[test]
fn merge_sums_counts_for_the_same_metric() {
    let make = || {
        let mut bld = PriorsBuilder::default();
        bld.add_file_metrics(
            Language::Python,
            false,
            &FileMetrics {
                functions: vec![one_function()],
                module: ModuleMetrics { total_loc: 10, total_functions: 1, sum_cc: 3, ..empty_module() },
            },
        );
        bld
    };
    let mut a = make();
    let b = make();
    a.merge(&b);
    let cc = a.build(false).main.get("python").expect("python").metrics.get("cc").expect("cc").n;
    assert_eq!(cc, 2, "the same metric observed once per builder sums to two");
}

#[test]
fn json_round_trip_preserves_observations() {
    let mut original = PriorsBuilder::default();
    original.add_file_metrics(
        Language::Python,
        false,
        &FileMetrics {
            functions: vec![one_function()],
            module: ModuleMetrics { total_loc: 10, total_functions: 1, sum_cc: 3, ..empty_module() },
        },
    );
    let json = original.to_json();
    assert!(!json.is_empty());
    let mut rebuilt = PriorsBuilder::default();
    assert!(rebuilt.merge_json(&json), "valid json merges");
    let built = rebuilt.build(false);
    let cc = built.main.get("python").expect("python").metrics.get("cc").expect("cc");
    assert_eq!(cc.n, 1, "the merged observation survived the round trip");
}

#[test]
fn merge_json_rejects_malformed_input() {
    let mut builder = PriorsBuilder::default();
    assert!(!builder.merge_json("{ this is not json"), "garbage is rejected");
    assert!(!builder.merge_json("[]"), "wrong shape is rejected");
    assert!(builder.build(false).main.is_empty(), "nothing was absorbed");
}
