use std::cell::Cell;

pub const MAX_WALK_DEPTH: u32 = 2000;

thread_local! {
    static WALK_DEPTH: Cell<u32> = const { Cell::new(0) };
    static EDIT_SCOPE: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
    static CPG_ENABLED: Cell<bool> = const { Cell::new(false) };
}

pub struct DepthGuard;

impl DepthGuard {
    pub fn enter() -> Option<Self> {
        WALK_DEPTH.with(|d| {
            let cur = d.get();
            if cur >= MAX_WALK_DEPTH {
                return None;
            }
            d.set(cur + 1);
            Some(Self)
        })
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        WALK_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

struct ScopeReset;

impl Drop for ScopeReset {
    fn drop(&mut self) {
        EDIT_SCOPE.with(|s| s.set(None));
    }
}

pub fn with_edit_scope<T>(scope: Option<(usize, usize)>, f: impl FnOnce() -> T) -> T {
    EDIT_SCOPE.with(|s| s.set(scope));
    let _reset = ScopeReset;
    f()
}

pub(crate) fn extras_enabled(start_byte: usize, end_byte: usize) -> bool {
    let scope = EDIT_SCOPE.with(Cell::get);
    match scope {
        None => true,
        Some((lo, hi)) => start_byte <= hi && end_byte >= lo,
    }
}

struct CpgReset;

impl Drop for CpgReset {
    fn drop(&mut self) {
        CPG_ENABLED.with(|c| c.set(false));
    }
}

pub fn with_cpg_enabled<T>(enabled: bool, f: impl FnOnce() -> T) -> T {
    CPG_ENABLED.with(|c| c.set(enabled));
    let _reset = CpgReset;
    f()
}

pub(crate) fn cpg_enabled() -> bool {
    CPG_ENABLED.with(Cell::get)
}
