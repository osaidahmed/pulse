use std::path::Path;

use super::{locate_pulse_md, HOOKS, VERSION_KEY};

pub fn run(dir: &Path) {
    let hooks_changed = uninstall_hooks(dir);
    let md_changed = uninstall_claude_md(dir);
    if hooks_changed || md_changed {
        eprintln!("\nPulse removed. Restart your Claude Code session to clear the hooks.");
    } else {
        eprintln!("Pulse was not installed; nothing to remove.");
    }
}

fn uninstall_hooks(dir: &Path) -> bool {
    let path = dir.join("settings.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    let mut changed = strip_pulse_hooks(&mut root);
    if root
        .as_object_mut()
        .is_some_and(|o| o.remove(VERSION_KEY).is_some())
    {
        changed = true;
    }
    if changed {
        let json = serde_json::to_string_pretty(&root).unwrap_or_default();
        let _ = std::fs::write(&path, json.as_bytes());
        eprintln!("  - removed pulse hooks from {}", path.display());
    }
    changed
}

fn strip_pulse_hooks(root: &mut serde_json::Value) -> bool {
    let Some(hooks) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
        return false;
    };
    let mut changed = false;
    for &(event, _, _) in HOOKS {
        let removed = hooks
            .get_mut(event)
            .and_then(|g| g.as_array_mut())
            .is_some_and(remove_pulse_from_event);
        let empty = hooks
            .get(event)
            .and_then(|g| g.as_array())
            .is_some_and(Vec::is_empty);
        if removed {
            changed = true;
        }
        if removed && empty {
            hooks.remove(event);
        }
    }
    changed
}

fn remove_pulse_from_event(arr: &mut Vec<serde_json::Value>) -> bool {
    let mut changed = false;
    for group in arr.iter_mut() {
        if let Some(hooks) = group.get_mut("hooks").and_then(|h| h.as_array_mut()) {
            let before = hooks.len();
            hooks.retain(|h| !is_pulse_command(h));
            changed |= hooks.len() != before;
        }
    }
    let before = arr.len();
    arr.retain(|g| {
        g.get("hooks")
            .and_then(|h| h.as_array())
            .is_none_or(|a| !a.is_empty())
    });
    changed || arr.len() != before
}

fn is_pulse_command(hook: &serde_json::Value) -> bool {
    hook.get("command")
        .and_then(|c| c.as_str())
        .is_some_and(|c| HOOKS.iter().any(|&(_, _, cmd)| cmd == c))
}

fn uninstall_claude_md(dir: &Path) -> bool {
    let path = dir.join("CLAUDE.md");
    let Ok(existing) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Some((s, e)) = locate_pulse_md(&existing) else {
        return false;
    };
    let start = if s > 0 && existing[..s].ends_with('\n') {
        s - 1
    } else {
        s
    };
    let cleaned = format!("{}{}", &existing[..start], &existing[e..]);
    let _ = std::fs::write(&path, cleaned);
    eprintln!("  - removed pulse instructions from {}", path.display());
    true
}
