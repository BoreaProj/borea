//! borea command-line entry point.
//!
//! Status: plan stage. The command surface and exit codes are specified in
//! `docs/ARCHITECTURE.md` §4 and §14; the implementation lands with cards
//! A02, A07 and A08.

fn main() {
    let version = borea_core::version();
    eprintln!("borea {version} (plan stage)");
    eprintln!("no installer path is implemented yet; see docs/PLAN.md");
    std::process::exit(2);
}
