use std::path::{Path, PathBuf};

use xxhash_rust::xxh3::xxh3_64;

use crate::thresholds::Thresholds;

mod uninstall;

const HOOKS: &[(&str, Option<&str>, &str)] = &[
    ("PostToolUse", Some("Edit|Write|MultiEdit"), "pulse --hook"),
    ("Stop", Some(".*"), "pulse --stop"),
    ("SessionStart", None, "pulse --cleanup"),
];

const MD_START: &str = "<!-- pulse:setup";
const MD_END: &str = "<!-- /pulse:setup -->";
const LEGACY_SIGNATURE: &str = "Pulse fires as a PostToolUse hook";
const VERSION_KEY: &str = "_pulse_version";

pub fn run_setup(uninstall: bool) {
    let dir = claude_dir();
    if uninstall {
        uninstall::run(&dir);
        return;
    }
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
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));

    let mut changed = false;
    let hooks = root
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));
    if !hooks.is_object() {
        *hooks = serde_json::json!({});
    }

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

    if set_version(&mut root) {
        changed = true;
    }

    if changed {
        let json = serde_json::to_string_pretty(&root).unwrap_or_default();
        let _ = std::fs::write(&path, json.as_bytes());
    } else {
        eprintln!("  . hooks already configured in {}", path.display());
    }
    changed
}

fn set_version(root: &mut serde_json::Value) -> bool {
    let ver = serde_json::json!(env!("CARGO_PKG_VERSION"));
    let Some(obj) = root.as_object_mut() else {
        return false;
    };
    if obj.get(VERSION_KEY) == Some(&ver) {
        return false;
    }
    obj.insert(VERSION_KEY.to_string(), ver);
    true
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
    let block = render_md_block();

    let new_content = match locate_pulse_md(&existing) {
        Some((s, e)) if existing[s..e] == *block => {
            eprintln!("  . Pulse instructions already current in {}", path.display());
            return false;
        }
        Some((s, e)) => {
            eprintln!("  ~ updated Pulse instructions in {}", path.display());
            format!("{}{}{}", &existing[..s], block, &existing[e..])
        }
        None => {
            eprintln!("  + added Pulse instructions to {}", path.display());
            append_md_block(&existing, &block)
        }
    };
    let _ = std::fs::write(&path, new_content);
    true
}

fn append_md_block(existing: &str, block: &str) -> String {
    if existing.is_empty() {
        block.to_string()
    } else {
        format!("{existing}\n{block}")
    }
}

fn render_md_block() -> String {
    let body = pulse_instructions();
    let hash = xxh3_64(body.as_bytes());
    format!(
        "{MD_START} v={} hash={hash:016x} -->\n{body}{MD_END}\n",
        env!("CARGO_PKG_VERSION")
    )
}

fn locate_pulse_md(content: &str) -> Option<(usize, usize)> {
    if let Some(s) = content.find(MD_START) {
        let Some(rel) = content[s..].find(MD_END) else {
            return Some((s, content.len()));
        };
        let mut e = s + rel + MD_END.len();
        if content[e..].starts_with('\n') {
            e += 1;
        }
        return Some((s, e));
    }
    locate_legacy_md(content)
}

fn locate_legacy_md(content: &str) -> Option<(usize, usize)> {
    let sig = content.find(LEGACY_SIGNATURE)?;
    let head = content[..sig].rfind("# Pulse")?;
    if head != 0 && !content[..head].ends_with('\n') {
        return None;
    }
    let e = content[sig..]
        .find("\n# ")
        .map_or(content.len(), |r| sig + r + 1);
    Some((head, e))
}

fn pulse_instructions() -> String {
    format!(
        "# Pulse\n\n\
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
