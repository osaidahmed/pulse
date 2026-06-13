use std::fmt::Write;

use super::estimator::{Calibrated, Stratum, ThresholdPair};

enum Field {
    Pair(&'static str, &'static str),
    WarningLevel(&'static str),
    AlertLevel(&'static str),
}

const FIELD_MAP: &[(&str, Field)] = &[
    ("cc", Field::Pair("cc_warning", "cc_alert")),
    ("cogc", Field::Pair("cogc_warning", "cogc_alert")),
    ("fn_loc", Field::Pair("fn_loc_warning", "fn_loc_alert")),
    ("file_loc", Field::Pair("file_loc_warning", "file_loc_alert")),
    ("nesting", Field::WarningLevel("nesting_depth")),
    ("global_nesting", Field::WarningLevel("global_nesting_depth")),
    ("bump", Field::AlertLevel("bump_count")),
    ("args", Field::AlertLevel("arg_max")),
    ("compound_conditions", Field::AlertLevel("compound_conditions")),
    ("embedded_block_loc", Field::AlertLevel("embedded_block_loc")),
    ("consecutive_asserts", Field::AlertLevel("consecutive_asserts_max")),
    ("short_vars", Field::AlertLevel("short_var_max_count")),
    ("string_match_arms", Field::AlertLevel("max_string_match_arms")),
    ("struct_fields", Field::AlertLevel("max_struct_fields")),
    ("file_functions", Field::AlertLevel("file_function_count")),
    ("file_total_cc", Field::AlertLevel("file_total_cc")),
    ("declarations", Field::AlertLevel("max_declarations")),
    ("global_conditionals", Field::AlertLevel("global_conditionals_max")),
];

pub struct Rendered {
    pub main: String,
    pub tests: String,
}

pub fn render(calibrated: &Calibrated) -> Rendered {
    Rendered { main: render_stratum(&calibrated.main), tests: render_stratum(&calibrated.tests) }
}

fn render_stratum(stratum: &Stratum) -> String {
    let mut out = String::new();
    for (lang, metrics) in stratum {
        if !out.is_empty() {
            out.push('\n');
        }
        let _ = writeln!(out, "[languages.{lang}]");
        for (metric, field) in FIELD_MAP {
            if let Some(pair) = metrics.get(*metric) {
                write_field(&mut out, metric, field, pair);
            }
        }
    }
    out
}

fn write_field(out: &mut String, metric: &str, field: &Field, pair: &ThresholdPair) {
    match field {
        Field::Pair(warning_field, alert_field) => {
            let warning = round(pair.warning);
            let alert = round(pair.alert).max(warning + 1);
            let _ = writeln!(
                out,
                "# {metric} warning: {}",
                level(pair.editorial_warning, pair.corpus_warning, pair.warning_loosened())
            );
            let _ = writeln!(out, "{warning_field} = {warning}");
            let _ = writeln!(
                out,
                "# {metric} alert: {}",
                level(pair.editorial_alert, pair.corpus_alert, pair.alert_loosened())
            );
            let _ = writeln!(out, "{alert_field} = {alert}");
        }
        Field::WarningLevel(name) => {
            let _ = writeln!(
                out,
                "# {metric}: {}",
                level(pair.editorial_warning, pair.corpus_warning, pair.warning_loosened())
            );
            let _ = writeln!(out, "{name} = {}", round(pair.warning));
        }
        Field::AlertLevel(name) => {
            let _ =
                writeln!(out, "# {metric}: {}", level(pair.editorial_alert, pair.corpus_alert, pair.alert_loosened()));
            let _ = writeln!(out, "{name} = {}", round(pair.alert));
        }
    }
}

fn level(editorial: u32, corpus: f64, loosened: bool) -> String {
    if loosened {
        format!("editorial {editorial}, corpus {} — loosened to corpus", corpus.round() as u32)
    } else {
        format!("editorial {editorial} kept (corpus {})", corpus.round() as u32)
    }
}

fn round(value: f64) -> u32 {
    value.max(0.0).round() as u32
}
