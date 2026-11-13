# First-pass verification — 2026-11-13

The source was verified on Linux with system Rust/Cargo 1.99.1.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo test --workspace --all-features --locked` | 32 passed; none failed or ignored |
| `cargo xtask matrix check` | Passed structural and native-package mapping checks |
| `cargo xtask recipe lint` | Passed embedded recipe/profile validation |
| `cargo xtask matrix render --check` | Passed; generated table matches data |
| `cargo xtask plan golden` | 25 draft plans and 4 refusal fixtures match; 29 cases total |
| `cargo xtask secret canary` | Passed redaction and plan/dry-run boundary checks |
| `cargo xtask disk selftest --stage create` | Passed 20 GiB sparse image creation/removal |
| `cargo xtask e2e --profile omarchy-trim` | Passed preview argv mapping only |
| `cargo xtask matrix verify --strict` | Expected failure: 0/29 VM cells, exit 2 |
| Btrfs/ext4 disk mutation lanes | Expected unsupported errors, exit 3 |
| Arch minimal/GRUB installation e2e | Expected unsupported error, exit 3 |

The supported checks pass. The three gated integration areas remain failures
for release purposes; their expected exit status is a check of honest refusal,
not proof of the missing functionality.

Local command logs and a machine-readable summary are under
`target/validation/first-pass/` (ignored build artifacts). No VM artifact path
exists. No host disk was formatted/mounted, no package removal ran, and no
external SSH verification was performed.

See [IMPLEMENTATION.md](../IMPLEMENTATION.md) and [TASKS.md](../TASKS.md) for the
remaining work. Upstream packaging and old policy compatibility were not
validated by these tests.
