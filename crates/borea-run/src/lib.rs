//! Execution, journaling, resume and verification.
//!
//! The executor is deliberately small: one process, one plan, one append-only
//! journal. It never decides what is supported; the resolver does that.
