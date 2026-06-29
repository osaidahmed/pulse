#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JitThresholds {
    pub use_lt: bool,
    pub use_age: bool,
    pub use_entropy: bool,
    pub entropy_bits: f64,
}

impl JitThresholds {
    pub const DEFAULTS: Self = Self { use_lt: true, use_age: true, use_entropy: true, entropy_bits: 2.0 };
}
