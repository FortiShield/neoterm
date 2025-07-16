// This module would contain macros for embedding assets directly into the binary.
// For example, using `include_bytes!` or `include_str!`.
// This is a placeholder for future implementation.

/// A macro to embed a file as bytes at compile time.
///
/// Usage: `asset_bytes!("path/to/asset.png")`
#[macro_export]
macro_rules! asset_bytes {
    ($path:expr) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path))
    };
}

/// A macro to embed a file as a string at compile time.
///
/// Usage: `asset_str!("path/to/text.txt")`
#[macro_export]
macro_rules! asset_str {
    ($path:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path))
    };
}

pub fn init() {
    log::info!("Asset macro module initialized.");
    // No runtime initialization needed for macros, but this function
    // can be used for any setup if the module evolves to include
    // runtime asset management (e.g., loading from external sources).
}
