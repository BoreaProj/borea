# borea

A small, CLI-first Linux installer and customiser in Rust.

`borea` is an **independent, standalone Linux installer**. It installs a
**minimal system** without requiring a distro live environment or a
Borea-specific live environment, then adds one session. Its targets are
**Arch, Debian, Fedora 45 and openSUSE Tumbleweed**, and it says plainly which
combinations it cannot install yet instead of offering buttons that fail.

Each family starts from a **minimal worker base** built with that family's own
bootstrap tool (`pacstrap`, `mmdebstrap`, `dnf --installroot`,
`zypper --root`). On top of it Borea installs exactly one session
([D-039](docs/DECISIONS.md)):

- a **desktop**: KDE Plasma, GNOME or COSMIC; or
- a **shell**: Noctalia v5, DMS (DankMaterialShell) or End4 (Arch only), on
  Hyprland by default. Noctalia and DMS can run on Niri instead.

A bare compositor without a shell is refused, and MangoWM is refused until a
shell supports it. Third-party repositories are used only where a family has
no native package, and then only with a reviewed pin.

The root is **Btrfs by default** (`@`, `@home`, `@snapshots`, `@log`, `@cache`,
zstd compression), with ext4 where it is the better tool: the small unencrypted
`/boot` partition and an explicit `root_fs = "ext4"` fallback. Boot is
**Limine on Arch** and **GRUB elsewhere**, because Arch's stable kernel filenames
suit a static Limine config and the other families have native GRUB/BLS/snapper
integration.

Borea owns its installation workflow and runtime tooling; the selected distro
is the installation target, not the environment Borea must run inside. Verified tool bundles are delivered for all four families; see
[Independent runtimes](docs/RUNTIME.md).

Standalone flavours are planned for v0.2: **Omarchy** (Arch), **PikaOS**
(Debian) and **Nobara** (Fedora), each adjustable by Borea in the same way as
Omarchy trim. openSUSE has no flavour; it is base plus desktop or shell.

The Arch family already offers an experimental **Omarchy profile**, following
Linutil/upstream defaults with explicit `keep`, `remove`, replacement and browser
choices. Unselected apps stay unchanged. See [Omarchy controls](docs/OMARCHY.md).

It also carries the existing-system Omarchy tools: a **trimmer** for Omarchy defaults and a
**default manager** that keeps removed defaults from coming back after updates.

**Status: experimental, pre-release.** Config validation, matrix gates,
deterministic plans, family/session command generation, dry-run, resume
journals, verification reports and Omarchy policy operations are implemented.

The **image lane** works end to end: `cargo xtask e2e` partitions a raw disk
image, runs the sealed plan inside a sandbox, boots the result in QEMU/UEFI and
verifies it over the serial console and SSH. These cells have passed in CI:

| Cell | Root | Loader |
| --- | --- | --- |
| `arch/minimal` | Btrfs | Limine |
| `arch/minimal-ext4` | ext4 | Limine |
| `arch/minimal-grub` | Btrfs | GRUB |
| `debian/minimal` | Btrfs | GRUB |
| `debian/minimal-ext4` | ext4 | GRUB |

A cell only counts as validated while its record in
[`recipes/validated.json`](recipes/validated.json) matches its current golden
plan. The C06 base hardening changed every plan, so the earlier records are
stale and `matrix check` reports **0/29 required cells validated** until the
nightly run on `main` passes and the cells are re-recorded. Fedora, openSUSE
and every desktop/shell cell are planned but not yet run in a VM.

**Not available yet:** `apply` against a real disk (it refuses before any
disk access), LUKS, and Omarchy removals on a live system. Third-party and
source paths without pins refuse planning. The image lane needs an Ubuntu
24.04 (or similar) host; Ubuntu 26.04 denies the sandbox's mounts (G08).
See [`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md) for the exact working
commands, limitations and remaining work. The full v0.1 contract remains in
[`docs/PLAN.md`](docs/PLAN.md).

## Why this exists

The previous `borea` grew a live ISO builder, an initramfs, an iced GUI, a USB
writer, a Sigstore release pipeline, a Varlink daemon and a simulation engine
before a single real install worked. This repository is the clean-room
replacement: one binary, one plan format, four family adapters, a VM test loop,
and nothing else until the matrix is green.

What is deliberately **not** here:

- no GUI, TUI dashboard or recorder
- no live ISO, custom initramfs, USB writer or release attestation
- no daemon, socket protocol or background worker
- no simulation engine — `--dry-run` and golden plan tests replace it
- no Borea snapshot manager or automatic rollback; the optional Omarchy profile
  retains its upstream-managed defaults
- no dual-boot, Secure Boot signing or ARM in v1

## The model

```text
borea.toml ──> borea plan ──> plan.json ──> borea apply ──> journal.jsonl
                   │                              │
              recipes + matrix               distro-native tools
                   │                              │
                   └──────── borea verify <───────┘
```

- **`plan`** resolves the config against the capability matrix and emits a
  deterministic JSON plan. It touches nothing.
- **`apply`** executes the plan with a journal, idempotent probes and resume.
- **`verify`** checks the finished target offline (`--root`) or over SSH.

The plan is the only contract between the CLI, the tests and an agent. If a
change cannot be expressed as plan steps and checks, it is out of scope.

## Try the first implementation

```sh
cargo build --workspace --locked
cargo run -p borea-cli -- runtime list --json
cargo run -p borea-cli -- catalogue --family arch --json
cargo run -p borea-cli -- plan \
  --config recipes/profiles/minimal.toml \
  --fixture --identity fixtures/device.json --out /tmp/borea-plan.json
cargo run -p borea-cli -- apply /tmp/borea-plan.json --dry-run
cargo run -p borea-cli -- omarchy trim docker whatsapp --dry-run
cargo run -p borea-cli -- omarchy configure --config examples/omarchy.toml --json
cargo xtask matrix check
cargo xtask recipe lint
cargo xtask plan golden
```

The image lane (needs `bwrap`, QEMU and OVMF; uses an image file and loop
device, never a real disk). Each run writes `target/e2e/<family>/<profile>/`:

```sh
cargo xtask e2e --family arch --profile minimal
cargo xtask e2e --family debian --profile minimal --root-fs ext4
cargo xtask matrix verify --strict
```

Only CI runs are recorded as validated (`cargo xtask matrix record --result
... --run ...`); see [`docs/TESTING.md`](docs/TESTING.md).

Use a fresh output path: `plan` refuses to overwrite files. Fixture identities
are for inspection only. `apply --yes --device ...` returns an unsupported error
before any disk access or writes. File-only fixture execution and policy
examples are documented in [`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md).

## Repository map

| Path | What lives there |
| --- | --- |
| `crates/borea-cli` | argument parsing, human output, exit codes |
| `crates/borea-core` | config, plan, recipes, matrix, checks, validation |
| `crates/borea-distro` | Arch / Debian / Fedora / openSUSE adapters |
| `crates/borea-session` | KDE, GNOME, COSMIC, compositors and shells |
| `crates/borea-run` | executor, journal, probes, resume, redaction |
| `crates/borea-defaults` | the on-target default manager binary |
| `recipes/` | TOML data: families, sessions, apps, profiles, repos |
| `docs/` | plan, architecture, matrix, tasks, testing, decisions |
| `fixtures/` | golden plans, fake roots, fake package managers |

## Read this first

| If you are… | Read |
| --- | --- |
| new to the repo | [`docs/PLAN.md`](docs/PLAN.md) |
| picking up a task | [`docs/TASKS.md`](docs/TASKS.md), then `AGENTS.md` |
| changing the plan format | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) |
| adding a distro or session | [`docs/RECIPES.md`](docs/RECIPES.md), [`docs/MATRIX.md`](docs/MATRIX.md) |
| working on Omarchy | [`docs/OMARCHY.md`](docs/OMARCHY.md) |
| running the test loop | [`docs/TESTING.md`](docs/TESTING.md) |
| running a real install | [`docs/OPERATIONS.md`](docs/OPERATIONS.md) |
| an agent (Claude/Codex) | `AGENTS.md` and [`docs/PROMPTS.md`](docs/PROMPTS.md) |

## Coverage

29 cells are required for v0.1: the minimal base on every family, plus each
D-039 desktop and shell where its packages exist. The generated table, with
evidence and dates, is [`docs/MATRIX.md`](docs/MATRIX.md); in short:

| | Arch | Debian | Fedora | openSUSE |
| --- | --- | --- | --- | --- |
| minimal, KDE, GNOME | native | native | native | native |
| COSMIC | native | refused | native | third-party |
| Noctalia v5 | native | third-party | native | third-party |
| DMS | native | third-party | third-party | third-party |
| End4 | source | refused | refused | refused |
| Hyprland | native | native | native | native |
| Niri (Noctalia, DMS) | native | refused | native | native |
| Btrfs root, ext4 `/boot` | yes | yes | yes | yes |
| ext4 root fallback | yes | yes | yes | yes |
| Limine | default | refused | refused | refused |
| GRUB | yes | default | default | default |

`native`, `third-party` and `source` describe where packages come from, not
that an install works; only cells listed under Status have booted. A refusal
is an error with a reason and a remedy, not a hidden feature.

## Licence

MIT. See [`LICENSE`](LICENSE). Third-party recipes and upstream projects keep
their own licences.
