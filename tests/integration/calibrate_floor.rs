use std::collections::BTreeSet;

use pulse::calibrate::estimator::{estimate_languages, EstimatorConfig};
use pulse::calibrate::priors::corpus_priors;
use pulse::parse::Language;

const CONTINUOUS: [&str; 4] = ["cc", "cogc", "fn_loc", "embedded_block_loc"];

#[test]
fn editorial_defaults_sit_above_every_language_continuous_quantile() {
    let cfg = EstimatorConfig::default();
    let corpus = &corpus_priors().main;
    for lang in Language::ALL {
        let key = lang.to_config_key();
        let mut one = BTreeSet::new();
        one.insert(key.to_string());
        let stratum = estimate_languages(&one, corpus, &cfg);
        let Some(metrics) = stratum.get(key) else { continue };
        for name in CONTINUOUS {
            let Some(pair) = metrics.get(name) else { continue };
            assert!(
                !pair.warning_loosened(),
                "{key} {name}: corpus warning quantile {} rose above editorial {} — per-language flooring is no longer inert here",
                pair.corpus_warning,
                pair.editorial_warning
            );
            assert!(
                !pair.alert_loosened(),
                "{key} {name}: corpus alert quantile {} rose above editorial {} — per-language flooring is no longer inert here",
                pair.corpus_alert,
                pair.editorial_alert
            );
        }
    }
}
