# borea

A small, CLI-first Linux installer and customiser in Rust.

`borea` is an **independent, standalone Linux installer**. It installs a
**minimal system** without requiring a distro live environment or a
Borea-specific live environment, then adds one session: KDE, GNOME or COSMIC, or a Wayland compositor (Hyprland,
Niri, MangoWM) with a shell (Noctalia, DUX/Dank Material Shell, end-4). Its targets are
**Arch, Debian, Fedora and openSUSE**, and says plainly which combinations
it cannot install yet instead of offering buttons that fail.

The root is **Btrfs by default** (`@`, `@home`, `@snapshots`, `@log`, `@cache`,
zstd compression), with ext4 where it is the better tool: the small unencrypted
`/boot` partition and an explicit `root_fs = "ext4"` fallback. Boot is
**Limine on Arch** and **GRUB elsewhere**, because Arch's stable kernel filenames
suit a static Limine config and the other families have native GRUB/BLS/snapper
integration.

Borea owns its installation workflow and runtime tooling; the selected distro
is the installation target, not the environment Borea must run inside. Runtime
provisioning is still pending in this preview (see task G01).

It also carries the Omarchy tools: a **trimmer** for Omarchy defaults and a
**default manager** that keeps removed defaults from coming back after updates.

**Status: experimental implementation preview.** Config validation, matrix gates,
deterministic plans, family/session command generation, dry-run, file-only fixture
execution with resume journals, verification reports and Omarchy policy operations
are implemented. **Live installs and Omarchy removals are disabled. No VM cell
has passed.** Third-party/source paths without pins refuse planning.
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
- no snapshot manager or automatic rollback — the Btrfs layout is snapshot-ready
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
cargo run -p borea-cli -- catalogue --family arch --json
cargo run -p borea-cli -- plan \
  --config recipes/profiles/minimal.toml \
  --fixture --identity fixtures/device.json --out /tmp/borea-plan.json
cargo run -p borea-cli -- apply /tmp/borea-plan.json --dry-run
cargo run -p borea-cli -- omarchy trim docker whatsapp --dry-run
cargo xtask matrix check
cargo xtask recipe lint
cargo xtask plan golden
```

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

## Target packaging coverage

The v0.1 plan targets minimal, KDE, GNOME, Hyprland, Noctalia and DUX/DMS on every family.
These are packaging intentions, not working installs. All cells remain untested;
some third-party recipes are explicitly refused until pins are reviewed.
Everything else is a documented gate. The full table, evidence and dates are in
[`docs/MATRIX.md`](docs/MATRIX.md); the short version:

| | Arch | Debian | Fedora | openSUSE |
| --- | --- | --- | --- | --- |
| minimal, KDE, GNOME | yes | yes | yes | yes |
| Hyprland | yes | yes | yes | yes |
| Noctalia, DUX/DMS | yes | yes | yes | yes |
| Btrfs root, ext4 `/boot` | yes | yes | yes | yes |
| ext4 root fallback | yes | yes | yes | yes |
| Limine | yes (default) | gate | gate | gate |
| GRUB | yes (fallback) | yes (default) | yes (default) | yes (default) |
| Niri | yes | gate | yes | yes |
| MangoWM | yes | third-party | third-party | gate |
| COSMIC | yes | gate | yes | third-party |
| end-4 | source | gate | gate | gate |

A gate is a refusal with a reason, not a hidden feature and not a promise.

## Licence

MIT. See [`LICENSE`](LICENSE). Third-party recipes and upstream projects keep
their own licences.
