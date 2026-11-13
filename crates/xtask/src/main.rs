//! Development tasks.
//!
//! Planned subcommands (cards A05, A06, F01-F05):
//!   matrix check|render|verify
//!   recipe lint
//!   plan golden
//!   secret canary
//!   disk selftest
//!   env --family <f> -- <command>
//!   vm boot --image <img>
//!   e2e --family <f> --profile <p>

fn main() {
    eprintln!("xtask: tasks land with the cards in docs/TASKS.md");
    std::process::exit(2);
}
