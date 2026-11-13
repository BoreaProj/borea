//! Family adapters and storage/boot planning.
//!
//! This is the only crate allowed to know distro-specific command syntax.
//! `docs/TASKS.md` workstream C fills in the adapters.

/// Families borea v0.1 understands.
pub const FAMILIES: [&str; 4] = ["arch", "debian", "fedora", "suse"];
