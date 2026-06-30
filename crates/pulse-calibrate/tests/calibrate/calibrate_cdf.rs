use pulse_calibrate::priors::corpus_priors;

#[test]
fn cdf_is_monotone_and_bounded_for_every_metric() {
    for lp in corpus_priors().main.values() {
        for mp in lp.metrics.values() {
            assert!(!mp.cdf.is_empty(), "every baked metric carries a cdf");
            let mut prev = 0.0;
            for v in 0..256u32 {
                let c = mp.cdf_at(v);
                assert!((0.0..=1.0).contains(&c), "cdf stays in [0,1]");
                assert!(c + 1e-9 >= prev, "cdf is monotone non-decreasing");
                prev = c;
            }
            assert!((mp.cdf_at(u32::MAX) - 1.0).abs() < 1e-9, "all mass sits at or below the max value");
            assert!((0.0..=1.0).contains(&mp.firing_rate(7)), "firing rate stays in [0,1]");
        }
    }
}

#[test]
fn cdf_agrees_with_the_quantile_probes() {
    let py = &corpus_priors().main["python"];
    let cc = &py.metrics["cc"];
    assert!(!cc.cdf.is_empty(), "python cc cdf is baked");
    let v90 = cc.quantile(0.90).ceil() as u32;
    assert!(cc.cdf_at(v90) >= 0.85, "cdf at the p90 value sits near 0.9: {}", cc.cdf_at(v90));
    assert!((cc.firing_rate(0) - 1.0).abs() < 1e-9, "every function has cc > 0");
}
