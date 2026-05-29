use std::path::{Path, PathBuf};

use crate::thresholds::Thresholds;

const HOOKS: &[(&str, Option<&str>, &str)] = &[
    ("PostToolUse", Some("Edit|Write|MultiEdit"), "pulse --hook"),
    ("Stop", Some(".*"), "pulse --stop"),
    ("SessionStart", None, "pulse --cleanup"),
];

pub fn run_setup() {
    let dir = claude_dir();
    let _ = std::fs::create_dir_all(&dir);

    let hooks_changed = configure_hooks(&dir);
    let md_changed = configure_claude_md(&dir);

    if hooks_changed || md_changed {
        eprintln!("\nSetup complete. Start a Claude Code session and Pulse will monitor code health automatically.");
    }
}

fn claude_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Path::new(&home).join(".claude")
}

fn configure_hooks(dir: &Path) -> bool {
    let path = dir.join("settings.json");
    let mut root: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let hooks = root
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));

    let mut changed = false;
    for &(event, matcher, command) in HOOKS {
        let groups = hooks
            .as_object_mut()
            .unwrap()
            .entry(event)
            .or_insert_with(|| serde_json::json!([]));
        if ensure_hook_entry(groups, matcher, command) {
            eprintln!("  + configured {event} hook ({command})");
            changed = true;
        }
    }

    if changed {
        let json = serde_json::to_string_pretty(&root).unwrap_or_default();
        let _ = std::fs::write(&path, json.as_bytes());
    } else {
        eprintln!("  . hooks already configured in {}", path.display());
    }
    changed
}

fn ensure_hook_entry(groups: &mut serde_json::Value, matcher: Option<&str>, command: &str) -> bool {
    let arr = groups.as_array_mut().unwrap();
    if let Some(group) = arr.iter_mut().find(|g| has_pulse_command(g, command)) {
        return migrate_matcher(group, matcher);
    }
    if let Some(group) = arr.iter_mut().find(|g| group_matcher_matches(g, matcher)) {
        if let Some(hooks) = g_hooks_mut(group) {
            hooks.push(build_hook_object(command));
            return true;
        }
    }
    arr.push(build_group_object(matcher, command));
    true
}

fn migrate_matcher(group: &mut serde_json::Value, matcher: Option<&str>) -> bool {
    if group_matcher_matches(group, matcher) {
        return false;
    }
    let Some(obj) = group.as_object_mut() else {
        return false;
    };
    match matcher {
        Some(m) => {
            obj.insert("matcher".into(), serde_json::json!(m));
        }
        None => {
            obj.remove("matcher");
        }
    }
    true
}

fn group_matcher_matches(group: &serde_json::Value, matcher: Option<&str>) -> bool {
    match (group.get("matcher").and_then(|m| m.as_str()), matcher) {
        (Some(existing), Some(wanted)) => existing == wanted,
        (None, None) => true,
        _ => false,
    }
}

fn has_pulse_command(group: &serde_json::Value, command: &str) -> bool {
    group
        .get("hooks")
        .and_then(|h| h.as_array())
        .is_some_and(|hooks| {
            hooks.iter().any(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|c| c == command)
            })
        })
}

fn g_hooks_mut(group: &mut serde_json::Value) -> Option<&mut Vec<serde_json::Value>> {
    group.get_mut("hooks")?.as_array_mut()
}

fn build_hook_object(command: &str) -> serde_json::Value {
    serde_json::json!({"type": "command", "command": command})
}

fn build_group_object(matcher: Option<&str>, command: &str) -> serde_json::Value {
    let hook = build_hook_object(command);
    match matcher {
        Some(m) => serde_json::json!({"matcher": m, "hooks": [hook]}),
        None => serde_json::json!({"hooks": [hook]}),
    }
}

fn configure_claude_md(dir: &Path) -> bool {
    let path = dir.join("CLAUDE.md");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();

    if existing.contains("\n# Pulse") || existing.starts_with("# Pulse") {
        eprintln!("  . Pulse instructions already in {}", path.display());
        return false;
    }

    let content = format!("{}{}", existing, pulse_instructions());
    let _ = std::fs::write(&path, content);
    eprintln!("  + added Pulse instructions to {}", path.display());
    true
}

fn pulse_instructions() -> String {
    format!(
        "\n# Pulse\n\n\
        Pulse fires as a PostToolUse hook on every file edit. When it reports \
        `error[pulse]` findings, fix them before moving to your next step. \
        Pulse findings are blocking \u{2014} treat them as linter errors that must be \
        resolved before proceeding.\n\n\
        Do not filter by authorship, severity, or pre-existing status. \
        If pulse surfaces it, address it.\n\n\
        Before creating a new source file, MUST run `pulse budget --new` and plan \
        within the reported ceilings. Before expanding an existing file significantly, \
        run `pulse budget <file>` to check remaining headroom (functions, LOC, cc).\n\n\
        Pulse threshold quick reference:\n{}\n",
        threshold_reference()
    )
}

fn threshold_reference() -> String {
    let t = Thresholds::default();
    format!(
        "cc>={} warning, >={} alert | cogc>={} warning, >={} alert | \
         fn_loc>={} warning, >={} alert | nesting>={} | args>{} | \
         compound_conditions>{} (strict >) | file_loc>={} | \
         functions>{} | total_cc>{} | struct_fields>{} | \
         string_match_arms>{}",
        t.function.cc_warning,
        t.function.cc_alert,
        t.function.cogc_warning,
        t.function.cogc_alert,
        t.function.fn_loc_warning,
        t.function.fn_loc_alert,
        t.function.nesting_depth,
        t.function.arg_max,
        t.function.compound_conditions,
        t.module.file_loc_warning,
        t.module.file_function_count,
        t.module.file_total_cc,
        t.module.max_struct_fields,
        t.analysis.max_string_match_arms
    )
}
