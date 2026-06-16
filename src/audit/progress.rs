use std::io::{IsTerminal, Write};

/// Progress is shown only on an interactive terminal, so it never pollutes piped
/// output (`--json`), redirected stderr, or CI logs. `PULSE_FORCE_PROGRESS` overrides
/// the terminal check (for verification on non-tty harnesses); it still writes to stderr
/// only, so stdout output is never affected.
pub fn is_active() -> bool {
    std::env::var("PULSE_FORCE_PROGRESS").is_ok() || std::io::stderr().is_terminal()
}

/// Rewrite the single status line in place (carriage return + clear-to-end-of-line).
pub fn show(line: &str) {
    let mut err = std::io::stderr().lock();
    let _ = write!(err, "\r\x1b[K{line}");
    let _ = err.flush();
}

pub fn clear() {
    let mut err = std::io::stderr().lock();
    let _ = write!(err, "\r\x1b[K");
    let _ = err.flush();
}
