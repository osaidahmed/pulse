use clap::{Parser, Subcommand};

use pulse::audit::PassChoice;

pub const USAGE: &str = "usage: pulse setup | --hook | --stop | --cleanup | check <file> | debug <file> | budget <file> | -a/--all [--include-tests] | audit | history | calibrate | --version";

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
    /// Install pulse hooks into ~/.claude (or remove them with --uninstall).
    Setup {
        /// Remove pulse hooks and the Pulse CLAUDE.md block.
        #[arg(long)]
        uninstall: bool,
    },
    /// Analyze a single file (or use -a for whole project).
    Check {
        file: Option<String>,
        /// Emit findings as a JSON array instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Dump raw per-function metrics for a file.
    Debug { file: String },
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
    /// Derive project-specific thresholds and write them to .pulse.toml.
    Calibrate(CalibrateArgs),
}

#[derive(clap::Args, Debug)]
pub struct CalibrateArgs {
    /// Project root directory (defaults to current directory).
    #[arg(long)]
    pub root: Option<String>,

    /// Write the derived thresholds to .pulse.toml instead of printing them.
    #[arg(long)]
    pub write: bool,

    /// Corpus percentile for the per-language warning floor (default 0.75).
    #[arg(long = "warn-percentile")]
    pub warn_percentile: Option<f64>,

    /// Corpus percentile for the per-language alert floor (default 0.95).
    #[arg(long = "alert-percentile")]
    pub alert_percentile: Option<f64>,
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

    /// Query deps.dev for dependency freshness and known vulnerabilities (opt-in network, cached).
    #[arg(long)]
    pub online: bool,
}

#[derive(clap::Args, Debug)]
#[allow(clippy::struct_excessive_bools)]
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

    /// Surface evolutionary (HIST) smells such as file blobs (opt-in).
    #[arg(long)]
    pub hist: bool,

    /// Flag architectural cycles freshly introduced (catalyst) or worsening (decay) across the history window (opt-in).
    #[arg(long = "arch-trend")]
    pub arch_trend: bool,

    /// Disable the SZZ-lite defect-prone-file detector (on by default; skips its extra bug-fix blame git work).
    #[arg(long = "no-szz")]
    pub no_szz: bool,

    /// Compute and persist JIT edit-risk calibration (LT/AGE percentiles + change-entropy threshold) for the repo.
    #[arg(long = "jit-calibrate")]
    pub jit_calibrate: bool,
}

pub enum Dispatch {
    Hook,
    Stop,
    Cleanup,
    Setup { uninstall: bool },
    Check { file: String, json: bool },
    CheckAll { include_tests: bool, json: bool },
    Debug(String),
    Budget(Option<String>),
    Audit { args: AuditArgs, include_tests: bool },
    History { args: HistoryArgs, include_tests: bool },
    Calibrate(CalibrateArgs),
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
            eprintln!("{USAGE}");
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
        None if all => Dispatch::CheckAll { include_tests, json: false },
        None => Dispatch::UsageError,
    }
}

fn dispatch_subcmd(sub: SubCmd, all: bool, include_tests: bool) -> Dispatch {
    match sub {
        SubCmd::Setup { uninstall } => Dispatch::Setup { uninstall },
        SubCmd::Check { file, json } => fileful_dispatch(
            file,
            all,
            |f| Dispatch::Check { file: f, json },
            || Dispatch::CheckAll { include_tests, json },
        ),
        SubCmd::Debug { file } => Dispatch::Debug(file),
        SubCmd::Budget { file, new } => {
            fileful_dispatch(file, new, |f| Dispatch::Budget(Some(f)), || Dispatch::Budget(None))
        }
        SubCmd::Audit(args) => Dispatch::Audit { args, include_tests },
        SubCmd::History(args) => Dispatch::History { args, include_tests },
        SubCmd::Calibrate(args) => Dispatch::Calibrate(args),
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
