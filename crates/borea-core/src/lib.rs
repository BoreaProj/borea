//! Shared, side-effect-free installer contracts.
pub mod config;
pub mod defaults;
pub mod error;
pub mod matrix;
pub mod omarchy;
pub mod plan;
pub mod recipe;
pub mod runtime;
pub use config::*;
pub use error::{BoreaError, Result};
pub use plan::*;
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
