pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "Deviruchi";

/// Build date - returns None if not set at compile time
pub const BUILD_DATE: Option<&str> = option_env!("BUILD_DATE");
