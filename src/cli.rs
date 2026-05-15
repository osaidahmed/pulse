use clap::{Parser, Subcommand};

use crate::audit::PassChoice;

/// Pulse — fast code smell detector and refactoring auditor.
#[derive(Parser, Debug)]
#[command(name = "pulse", version, disable_help_subcommand = true)]
pub struct Cli {
    /// Run analysis on every supported file in the project.
    #[arg(long = "all", short = 'a', global = true)]
    pub all: bool,

    /// Include test files in analysis (default: skip).
    #[arg(long = "include-tests", short = 't', global = true)]
    pub include_tests: bool,

    #[command(subcommand)]
    pub command: Option<SubCmd>,
}

#[derive(Subcommand, Debug)]
pub enum SubCmd {
    /// Install pulse hooks into ~/.claude.
    Setup,
    /// Analyze a single file (or use -a for whole project).
    Check {
        file: Option<String>,
    },
    /// Dump raw per-function metrics for a file.
    Debug {
        file: String,
    },
    /// Report threshold budgets for a file or for new files.
    Budget {
        file: Option<String>,
        /// Show ceilings that apply to a NEW file rather than an existing one.
        #[arg(long)]
        new: bool,
    },
    /// Cross-file refactoring analysis (manual invocation, never automated).
    Audit(AuditArgs),
    /// Mine git history for evolutionary smells (manual invocation, never automated).
    History(HistoryArgs),
}

#[derive(clap::Args, Debug)]
pub struct AuditArgs {
    /// Emit findings as JSON (envelope with summary + findings array).
    #[arg(long)]
    pub json: bool,

    /// Run a single analysis pass (default: all passes).
    #[arg(long, value_enum)]
    pub pass: Option<PassChoice>,

    /// Project root directory (defaults to current directory).
    #[arg(long)]
    pub root: Option<String>,

    /// Surface framework-convention and uncategorized findings (hidden by default).
    #[arg(long = "show-noise")]
    pub show_noise: bool,
}

#[derive(clap::Args, Debug)]
pub struct HistoryArgs {
    /// Emit findings as JSON (envelope with summary + findings array).
    #[arg(long)]
    pub json: bool,

    /// Project root directory (defaults to current directory).
    #[arg(long)]
    pub root: Option<String>,

    /// Only consider commits since this git-log expression (e.g. "6 months ago", "2024-01-01").
    #[arg(long)]
    pub since: Option<String>,

    /// Cap the number of commits scanned (escape hatch for very large repos).
    #[arg(long = "max-commits")]
    pub max_commits: Option<u32>,

    /// Override the cap on architectural-drift (co-change) findings reported.
    #[arg(long = "co-change-top")]
    pub co_change_top: Option<u32>,

    /// Override the cap on hotspot findings reported.
    #[arg(long = "hotspot-top")]
    pub hotspot_top: Option<u32>,

    /// Override the cap on knowledge-fragmentation findings reported.
    #[arg(long = "contributors-top")]
    pub contributors_top: Option<u32>,
}

pub enum Dispatch {
    Hook,
    Stop,
    Cleanup,
    Setup,
    Check(String),
    CheckAll { include_tests: bool },
    Debug(String),
    Budget(Option<String>),
    Audit { args: AuditArgs, include_tests: bool },
    History { args: HistoryArgs, include_tests: bool },
    UsageError,
}

pub fn parse() -> Dispatch {
    let raw: Vec<String> = std::env::args().collect();
    match raw.get(1).map(String::as_str) {
        Some("--hook") => Dispatch::Hook,
        Some("--stop") => Dispatch::Stop,
        Some("--cleanup") => Dispatch::Cleanup,
        _ => dispatch_from_clap(parse_clap()),
    }
}

fn parse_clap() -> Cli {
    Cli::try_parse().unwrap_or_else(|e| {
        if matches!(e.kind(), clap::error::ErrorKind::InvalidSubcommand) {
            eprintln!("usage: pulse setup | --hook | --stop | --cleanup | check <file> | debug <file> | budget <file> | -a/--all [--include-tests] | audit | history | --version");
            std::process::exit(1);
        }
        let _ = e.print();
        std::process::exit(usage_exit_code(&e));
    })
}

fn dispatch_from_clap(cli: Cli) -> Dispatch {
    let all = cli.all;
    let include_tests = cli.include_tests;
    match cli.command {
        Some(sub) => dispatch_subcmd(sub, all, include_tests),
        None if all => Dispatch::CheckAll { include_tests },
        None => Dispatch::UsageError,
    }
}

fn dispatch_subcmd(sub: SubCmd, all: bool, include_tests: bool) -> Dispatch {
    match sub {
        SubCmd::Setup => Dispatch::Setup,
        SubCmd::Check { file } => fileful_dispatch(file, all, Dispatch::Check, || Dispatch::CheckAll { include_tests }),
        SubCmd::Debug { file } => Dispatch::Debug(file),
        SubCmd::Budget { file, new } => fileful_dispatch(file, new, |f| Dispatch::Budget(Some(f)), || Dispatch::Budget(None)),
        SubCmd::Audit(args) => Dispatch::Audit { args, include_tests },
        SubCmd::History(args) => Dispatch::History { args, include_tests },
    }
}

fn fileful_dispatch(
    file: Option<String>,
    flag_active: bool,
    with_file: impl FnOnce(String) -> Dispatch,
    with_flag: impl FnOnce() -> Dispatch,
) -> Dispatch {
    if flag_active {
        return with_flag();
    }
    file.map_or(Dispatch::UsageError, with_file)
}

fn usage_exit_code(err: &clap::Error) -> i32 {
    use clap::error::ErrorKind;
    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
        _ => 2,
    }
}
