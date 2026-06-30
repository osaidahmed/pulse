use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

pub const REPO_BUDGET: Duration = Duration::from_mins(10);

pub fn drive_per_repo(
    corpus_root: &Path,
    snapshot_dir: &Path,
    ext: &str,
    mut spawn: impl FnMut(&Path, &Path) -> Child,
) -> Vec<PathBuf> {
    std::fs::create_dir_all(snapshot_dir).unwrap();
    let mut crashed = Vec::new();
    for repo in repo_dirs(corpus_root) {
        let out = snapshot_dir.join(format!("{}.{ext}", slug(&repo)));
        if out.exists() {
            continue;
        }
        if let Some(reason) = wait_bounded(spawn(&repo, &out)) {
            eprintln!("{reason} on {}", repo.display());
            crashed.push(repo.clone());
        }
    }
    crashed
}

pub fn wait_bounded(mut child: Child) -> Option<String> {
    let start = Instant::now();
    loop {
        match child.try_wait().unwrap() {
            Some(status) if status.success() => return None,
            Some(status) => return Some(format!("CRASH ({status})")),
            None if start.elapsed() > REPO_BUDGET => {
                let _ = child.kill();
                let _ = child.wait();
                return Some("TIMEOUT".to_string());
            }
            None => std::thread::sleep(Duration::from_secs(2)),
        }
    }
}

pub fn slug(repo: &Path) -> String {
    repo.components()
        .rev()
        .take(2)
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("_")
        .replace(['/', '.'], "_")
}

pub fn repo_dirs(root: &Path) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    for lang_dir in subdirs(root) {
        repos.extend(subdirs(&lang_dir));
    }
    repos.sort();
    repos
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()
}
