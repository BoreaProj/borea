//! Shared model for borea.
//!
//! This crate owns the config, recipe, matrix, plan and check types. It never
//! runs a command and never reads a block device. Modules land with the task
//! cards in `docs/TASKS.md`; the version constants exist so the workspace and
//! the CLI can agree on a schema from day zero.

/// Plan schema version. Bump when the public JSON changes.
pub const SCHEMA_VERSION: u16 = 1;

/// The borea release that produced this build.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
