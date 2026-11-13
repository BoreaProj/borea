# First implementation preview

This is the first executable version of the restarted Borea project. It is not
v0.1 and cannot install a system yet. The original plan remains the acceptance
contract. The preview deliberately exposes incomplete paths as errors rather
than reporting an installation or VM pass.

## Independent product contract

Borea is an entirely independent, standalone installer. Neither a distro live
or rescue environment nor a Borea-specific live environment is a prerequisite.
The selected distro is the target; Borea must supply or provision its own
installation tooling. D-031 supersedes the former distro-live requirement.

G01 now delivers verified runtime bundles for Arch, Debian, Fedora and openSUSE.
`doctor` probes those independent bundles rather than host PATH. Fetch, offline
verification, tool execution and namespace isolation are implemented; see
[RUNTIME.md](RUNTIME.md). G02 adds closed disposable-image storage execution. Cross-family VM
installation validation remains pending. No bootable Borea environment is
required or created.

## Arch Omarchy profile

E06/D-033 adds an Arch-only Omarchy source profile with pinned Linutil/upstream
defaults, a read-only configure preview, explicit app preservation/removal/
replacement and browser choices. No changes are selected by default. See
[OMARCHY.md](OMARCHY.md) and `examples/omarchy.toml` for the full controls.
Config/plan schema 2 is emitted; basic schema 1 configs remain readable, but old
plans require regeneration. The defaults policy and recipe/matrix schemas stay 1.
The typed upstream install stage is not executable yet; live apply remains gated.

Runtime delivery results are recorded in [validation/runtime.md](validation/runtime.md).

## Working behavior

| Area | Implemented and tested | Remaining |
| --- | --- | --- |
| Workspace | Seven Rust crates, locked dependencies, fmt/clippy/tests, Cargo aliases | Static musl release artifacts |
| Config | TOML, unknown-field rejection, defaults, name/path/size validation, v1 non-goal gates | Separate sudo and swapfile policies |
| Matrix | Complete four-family cells, duplicate/evidence checks, collected gates, generated table | Upstream package revalidation and VM evidence |
| Recipes | Embedded family/component data, package maps, dependency sort/cycle detection, compatibility | Full repository/key/source/seed schemas |
| Plans | Canonical SHA-256, digest validation, UUIDv5 storage IDs, stable ordered phases, config/matrix fingerprints | Full typed action/probe model from architecture |
| Storage | Closed image GPT/FAT/ext4/Btrfs execution, UUID/layout and filesystem probes, subvolumes and fstab; draft LUKS intent | Encrypted image execution, installation integration and VM boot validation |
| Families | Verified independent tool bundles plus draft native argv, base files, account/package intent | Real bootstrap repository setup and chroot lifecycle |
| Boot | Auto policy, Arch Limine/fallback configs, family GRUB/initramfs intent | Encryption hooks, native GRUB/BLS details, EFI entries, VM boot checks |
| Sessions/apps | KDE/GNOME/COSMIC, compositor/shell selection, greetd/greeter intent, Firefox/Flatpak/codec package maps | Shell seeds/ownership and upstream package validation; unpinned paths refuse |
| Execution | Dry-run and file fixtures; image identity/locks, durable receipts, private runtime snapshot, mount namespace cleanup, timeout and bounded diagnostics | Live installation commands, target stdin secrets and chroot bootstrap |
| Verification | Offline file checks; SSH argv quoting and check reports; failures exit 5 | Offline native package/unit checks, remote timeouts and complete boot checks |
| Omarchy | Full documented trim catalogue preview, browser protection, policy transitions, status/restore/note-install | Live removals, automatic update-window detection and hooks |
| Harness | 29 golden cases, recipe/matrix checks, secret boundary canary, disposable-image storage selftests, restricted cached container runner | Known-good VM/media pins, QEMU/SSH e2e and artifact collection |

No area whose remaining column includes VM validation is claimed installable.
The release gate `cargo xtask matrix verify --strict` intentionally exits nonzero
with all 29 missing cells. It never trusts an arbitrary hand-written pass marker.

## Reproducible preview

Requirements: Rust 1.99.1 with rustfmt/clippy, Linux. Use the system Rust tools if
an unrelated `mise` shim has no configured toolchain. Dependency versions are
locked in `Cargo.lock`; only the approved D-025 set is used.

```sh
cargo build --workspace --locked
cargo run -p borea-cli -- --help
cargo run -p borea-cli -- catalogue --family debian --json
cargo run -p borea-cli -- explain compositor.niri --family debian
cargo run -p borea-cli -- plan --config recipes/profiles/minimal.toml \
  --fixture --identity fixtures/device.json --out /tmp/borea-first-plan.json
cargo run -p borea-cli -- apply /tmp/borea-first-plan.json --dry-run --json
cargo run -p borea-cli -- omarchy trim --list
cargo run -p borea-cli -- omarchy trim docker whatsapp --dry-run --json
```

`plan` creates only the named output, with mode 0600, and refuses an existing
file. It does not read passwords, query the network or execute any planned
commands. Without `--fixture`, it probes the named device read-only with lsblk,
requires a stable serial, and rejects mounted devices/descendants. No real device
was probed during implementation tests. Plan output is inspectable draft data;
do not turn it into a shell script.

`doctor --family FAMILY --json` probes tool availability and UEFI read-only. A
missing prerequisite exits 3. It does not install missing tools.

## Fixture execution and verification

`apply PLAN --fixture-root DIR --journal FILE` accepts **file-only** plans in
an existing non-host root. Installation plans contain commands/destructive
steps and are refused in fixture mode. Symlink components and parent traversal
are rejected before journal creation. The journal is exclusive, mode 0600,
append-only and synced after each record. A new run refuses an existing journal;
`--resume` requires an existing journal and matching plan/config/matrix digests.
A truncated journal is an error. Probes determine whether a file step is skipped;
a postcondition must pass before it is marked complete.

`verify PLAN --root DIR` checks files without executing target binaries. Command
checks are explicitly failed with an unsupported detail. `--ssh USER@HOST` runs
the declared checks using the system SSH client, batch mode and existing host
key policy. It is opt-in and was not invoked against an external machine.

The integration test `fixture_apply_resume_verify_and_stale_journal` constructs a
file-only plan, executes it, resumes it, verifies it, mutates the root to prove
verification fails, then proves a changed plan cannot reuse the journal.

## Omarchy policies

`borea-defaults status --policy FILE` reads schema 1 JSON, retaining unknown
legacy fields. `restore ID...` marks existing items restored without reinstalling
anything. `note-install --window update|user` reads package names from stdin and
records reintroduced or restored; restored items remain restored. Explicit
window input is for fixture/manual recording, not an installed package hook.
Mutating policy commands require filesystem write permission, lock the file,
keep a `.json.bak` and sync the write. Automatic hook installation is absent.
`trim` and `enforce` exit 3 before reading or mutating package state.

No code was copied from the old installer. The original trim mapping came from
the restart design; E06 refreshed it against pinned upstream package/webapp
evidence. Old policy source files remain unavailable, so compatibility is limited
to the documented schema and unknown-field round trips; full legacy compatibility
is not yet proven.

## Validation

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo xtask matrix check
cargo xtask recipe lint
cargo xtask runtime check
cargo xtask matrix render --check
cargo xtask plan golden
cargo xtask secret canary
cargo xtask disk selftest --stage create
cargo xtask e2e --profile omarchy-trim
```

The last command checks trim **preview** argv only, not removal/enforcement.
Golden results: 27 plans, four refusals (Debian Noctalia/DMS, Fedora DMS,
openSUSE DMS). No VM artifacts exist. Disk Btrfs/ext4 mutation selftests and
installation e2e return unsupported, not success. `matrix verify --strict`
returns the expected release-blocking error.

The initial results are in [validation/first-pass.md](validation/first-pass.md).
The Arch Omarchy addition is recorded in [validation/omarchy-profile.md](validation/omarchy-profile.md).

## Next executable cards

G01 runtime provisioning and G02 closed image storage execution are implemented.
G02 validates the unencrypted GPT/filesystem/mount lane without interpreting
installation-plan commands. Next deliver target chroot/bootstrap execution,
secret transport and encryption, then install Arch minimal in an image and boot
it in QEMU. Continue through the existing family/session cards only after their
exact VM cells pass.

Before enabling live apply, also fix the known draft planner gaps: GRUB config
integration on Arch/Fedora/openSUSE, Fedora signed-boot behavior, Debian package
index initialization, openSUSE repo initialization, encrypted initramfs hooks,
account/root-lock and home ownership, true package/layout probes, initramfs and
SSH postconditions, and EFI entry checks. The live gate is not a VM test substitute.

G02 adds `borea image create|apply` for closed disposable-image storage
execution. It does not execute draft installation plans or install a target
system. See [IMAGE-EXECUTION.md](IMAGE-EXECUTION.md) for the separate schema,
privileged runtime snapshot, destructive-step probes and remaining gates.
