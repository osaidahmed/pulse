#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JitThresholds {
    pub use_lt: bool,
    pub use_age: bool,
}

impl JitThresholds {
    pub const DEFAULTS: Self = Self { use_lt: true, use_age: true };
}
