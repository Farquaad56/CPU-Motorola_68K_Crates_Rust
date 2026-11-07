//! Test machines shared by the CLI and the web front end (D-006).
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod flat;
pub mod session;
pub mod snapshot;
pub mod ssc;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn e01_02_version() {
        assert!(!super::VERSION.is_empty());
    }
}
