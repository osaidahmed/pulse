use std::io::{IsTerminal, Write};

pub fn is_active() -> bool {
    std::env::var("PULSE_FORCE_PROGRESS").is_ok() || std::io::stderr().is_terminal()
}

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
