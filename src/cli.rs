use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "pulse", version, disable_help_subcommand = true)]
pub struct Cli {
    #[arg(long = "all", short = 'a', global = true)]
    pub all: bool,

    #[arg(long = "include-tests", short = 't', global = true)]
    pub include_tests: bool,

    #[command(subcommand)]
    pub command: Option<SubCmd>,
}

#[derive(Subcommand, Debug)]
pub enum SubCmd {
    Setup,
    Check {
        file: Option<String>,
    },
    Debug {
        file: String,
    },
    Budget {
        file: Option<String>,
        #[arg(long)]
        new: bool,
    },
    Audit(AuditArgs),
}

#[derive(clap::Args, Debug)]
pub struct AuditArgs {
    #[arg(long)]
    pub json: bool,

    #[arg(long)]
    pub layer: Option<u8>,

    #[arg(long)]
    pub root: Option<String>,
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
    Audit(AuditArgs),
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
            eprintln!("usage: pulse setup | --hook | --stop | --cleanup | check <file> | debug <file> | budget <file> | -a/--all [--include-tests] | audit | --version");
            std::process::exit(1);
        }
        let _ = e.print();
        std::process::exit(usage_exit_code(&e));
    })
}

fn dispatch_from_clap(cli: Cli) -> Dispatch {
    match cli.command {
        Some(SubCmd::Setup) => Dispatch::Setup,
        Some(SubCmd::Check { file }) => fileful_dispatch(file, cli.all, Dispatch::Check, || Dispatch::CheckAll { include_tests: cli.include_tests }),
        Some(SubCmd::Debug { file }) => Dispatch::Debug(file),
        Some(SubCmd::Budget { file, new }) => fileful_dispatch(file, new, |f| Dispatch::Budget(Some(f)), || Dispatch::Budget(None)),
        Some(SubCmd::Audit(args)) => Dispatch::Audit(args),
        None if cli.all => Dispatch::CheckAll { include_tests: cli.include_tests },
        None => Dispatch::UsageError,
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
