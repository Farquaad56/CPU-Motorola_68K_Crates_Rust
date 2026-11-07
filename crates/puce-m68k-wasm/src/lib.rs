//! WebAssembly bindings for puce-m68k (skeleton; wasm-bindgen is added in E13).
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn e01_05_version() {
        assert!(!super::VERSION.is_empty());
    }
}
