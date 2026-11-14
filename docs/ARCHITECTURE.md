# Architecture

> This document describes the full v0.1 target. Current executable behavior and
> remaining gates are listed in [IMPLEMENTATION.md](IMPLEMENTATION.md).


This document defines the moving parts, the data that crosses between them, and
the invariants the executor enforces. Read it before changing the plan format,
the executor, the family trait or the CLI.

---

## Implemented schema versions

Config/plan schema 2 adds `session.type = "omarchy"`, explicit Omarchy app
choices, a typed `InstallOmarchy` source intent, Omarchy/package-absence probes,
and a package-absence check. Omarchy/Linutil pins and baseline data are embedded
under `recipes/omarchy/`. Basic config schema 1 remains readable, but Omarchy
requires 2 and plan schema 1 requires replanning. Recipe/matrix and defaults
policy schemas remain 1. The type sketches below describe the broader target;
[OMARCHY.md](OMARCHY.md) documents the implemented profile behavior.

## 1. Crates

Borea is an independent, standalone installer. Its execution host is separate
from the selected target family. Neither distro live/rescue media nor a
Borea-specific live environment is required. Borea must supply or provision its
installation tooling without assuming that the host uses the target package
manager. Family-native tools may be used as target implementation details.
G01 now delivers checksum-pinned family tool bundles with offline caching and
Bubblewrap execution. `doctor` probes those bundles, not host PATH. See
[RUNTIME.md](RUNTIME.md) for isolation, prerequisites, transient filesystem paths
and the boundary before privileged image execution. No host container daemon is
required by the runtime consumer.

```text
borea-cli ──> borea-core <── borea-distro
    │             ^  ^
    │             │  └──── borea-session
    └──> borea-run┘
borea-defaults ──> borea-core (defaults model only)
xtask ──────────> all
```

| Crate | Responsibility | Must not |
| --- | --- | --- |
| `borea-core` | config, recipes, matrix, plan model, checks, validation, `Family` trait | run commands, know package syntax |
| `borea-distro` | implement `Family` for arch/debian/fedora/suse; storage and boot steps | branch on session names |
| `borea-session` | resolve desktop/compositor/shell/app recipes into abstract requests | contain distro command strings |
| `borea-run` | execute steps, probes, journal, resume, redaction, chroot | decide policy or support |
| `borea-cli` | parse args, wire, render text/JSON, exit codes | contain install logic |
| `borea-defaults` | on-target trim enforcement | depend on the installer |
| `xtask` | matrix checks, golden plans, disk selftest, e2e | be required at runtime |

Dependency direction is one-way. `borea-core` has no dependency on the other
workspace crates.

## 2. Data flow

```text
borea.toml
  │ parse + validate
  ▼
Config ─────────────┐
                    ├── resolver ──> Plan ──> apply ──> Journal
recipes/*.toml ─────┤                 │
matrix.toml ────────┘                 └────> verify ──> Report
```

The resolver is pure. Given the same config, recipes and matrix, it produces the
same plan bytes. `plan` performs only read-only host probes (UEFI, tools, device
identity) and writes nothing except `--out`.

## 3. Core types

### 3.1 Config

```rust
pub struct Config {
    pub schema: u16,
    pub target: TargetConfig,
    pub disk: DiskConfig,
    pub boot: BootConfig,
    pub account: AccountConfig,
    pub session: SessionConfig,
    pub apps: AppsConfig,
    pub omarchy: Option<OmarchyConfig>,
}

pub struct TargetConfig {
    pub family: FamilyId,          // Arch | Debian | Fedora | Suse
    pub release: Option<String>,   // family default when None
    pub hostname: String,
    pub timezone: String,
    pub locale: String,
    pub keymap: String,
    pub arch: Arch,                // only X86_64 in v1
    pub firmware: Firmware,        // only Uefi in v1
}

pub struct DiskConfig {
    pub device: PathBuf,
    pub mode: DiskMode,            // Replace only in v1
    pub root_fs: Filesystem,       // Btrfs (default) | Ext4
    pub encrypt: bool,
    pub luks_password_file: Option<PathBuf>,
    pub esp_size_mib: u64,         // default 1024
    pub boot_size_mib: u64,        // default 2048; /boot is always ext4
    pub swap: Swap,                // None | Zram | File(u64)
}

pub enum Filesystem { Btrfs, Ext4 }

pub struct BootConfig {
    pub loader: Loader,            // Auto | Limine | Grub
    pub secure_boot: bool,         // only false in v1
}

pub enum Loader { Auto, Limine, Grub }

pub struct AccountConfig {
    pub username: String,
    pub full_name: String,
    pub password_file: PathBuf,
    pub sudo: SudoPolicy,          // Same | Separate
    pub sudo_password_file: Option<PathBuf>,
    /// When set, installs this key and enables sshd. Test profiles use it;
    /// production configs leave it unset (O-09).
    pub ssh_authorized_key_file: Option<PathBuf>,
}

pub enum SudoPolicy { Same, Separate }

pub struct SessionConfig {
    pub kind: SessionKind,         // Minimal | Desktop(DesktopId) | Shell { compositor, shell }
    pub greeter: Greeter,          // Auto | Sddm | Gdm | Greetd | None
}
```

Validation rejects unknown fields (`deny_unknown_fields`), non-absolute paths,
empty usernames, reserved usernames, NUL bytes in secrets, `swap=file` when
`encrypt` is false is allowed but documented, and any v1 non-goal value with
`E_UNSUPPORTED`. `secure_boot = true` is refused. `loader = "limine"` on
Debian, Fedora or openSUSE is refused with the matrix reason and remedy
(D-026); `loader = "auto"` resolves through the family recipe, never in core.

### 3.2 Recipe

Every installable thing is a recipe with a stable id and a kind:

```rust
pub enum RecipeKind { Family, Desktop, Compositor, Shell, App, Profile }

pub struct Recipe {
    pub schema: u16,
    pub id: String,
    pub kind: RecipeKind,
    pub name: String,
    pub description: String,
    pub requires: Vec<String>,          // component ids
    pub packages: BTreeMap<FamilyId, Vec<PackageRef>>,
    pub repos: Vec<String>,             // repo ids needed
    pub config: Vec<ConfigSeed>,        // files or commands applied after install
    pub session_entry: Option<SessionEntry>,
    pub evidence: Evidence,             // url + checked date
}
```

`PackageRef` names a package and a source class (`Official`, `Backports`,
`ThirdParty(repo_id)`, `Source(build_id)`). The family adapter maps the source
class to a command. See [`RECIPES.md`](RECIPES.md).

### 3.3 Matrix

```rust
pub struct Matrix { pub schema: u16, pub revision: String, pub cells: Vec<Cell> }

pub struct Cell {
    pub family: FamilyId,
    pub component: String,
    pub tier: Tier,                     // Native | ThirdParty | Source | Unsupported
    pub reason: Option<String>,         // required for Unsupported
    pub remedy: Option<String>,         // required for Unsupported
    pub evidence: Option<Evidence>,
}
```

The matrix is data, not code. `cargo xtask matrix check` fails on a missing
cell, a duplicate, an unsupported cell without reason/remedy, or a `native`
claim whose recipe has no official package for that family.

### 3.4 Plan

```rust
pub struct Plan {
    pub schema: u16,
    pub id: String,                     // uuid v4, stable for the run
    pub created: String,                // RFC 3339, excluded from digest
    pub borea_version: String,
    pub digest: String,                 // sha256 of canonical plan body
    pub config_digest: String,
    pub matrix_digest: String,
    pub target: TargetSummary,          // family, release, arch, firmware
    pub disk: DiskSummary,              // device, serial, size, layout id, root_fs
    pub boot: BootSummary,              // requested loader, resolved loader
    pub steps: Vec<Step>,
    pub checks: Vec<Check>,
    pub secrets: Vec<SecretSlot>,
    pub notes: Vec<String>,
}
```

Golden rule: **the plan is the API**. Tests compare it, agents read it, the
executor trusts it, `verify` re-derives its checks from it.

### 3.5 Step

```rust
pub struct Step {
    pub id: String,                     // "storage.partition", "bootstrap.arch"
    pub title: String,                  // human sentence
    pub action: Action,
    pub needs: Vec<String>,             // step ids that must be complete
    pub probe: Probe,                   // idempotency
    pub danger: Danger,                 // Safe | Mutating | Destructive
    pub timeout_s: u64,
    pub phase: Phase,                   // Storage | Bootstrap | Base | Session | Boot | Finalise
}

pub enum Action {
    Command(CommandSpec),
    WriteFile { path: PathBuf, contents: String, mode: u32 },
    CopyIn { source: PathBuf, dest: PathBuf, mode: u32 },
    Mkdir { path: PathBuf, mode: u32 },
    Partition { device: PathBuf, table: PartitionTable },
    Format { device: PathBuf, fs: Filesystem, label: String, uuid: Uuid },
    Subvolume { device: PathBuf, name: String },
    LuksFormat { device: PathBuf, name: String, uuid: Uuid, key: SecretRef },
    LuksOpen { device: PathBuf, name: String, key: SecretRef },
    Mount { source: String, target: PathBuf, fstype: Option<String>, options: Vec<String> },
    Umount { target: PathBuf },
    EnableUnit { root: PathBuf, unit: String },
}
```

`Uuid` is a plan-time value derived with UUIDv5 (D-029), never generated during
`apply`. `Format` maps to `mkfs.ext4 -U <uuid>` or `mkfs.btrfs -U <uuid>`; for
FAT32 it maps to `mkfs.fat -F32 -i <volume-id>` (the ESP is referenced by
`PARTUUID=` in fstab); `LuksFormat` maps to `cryptsetup luksFormat --uuid
<uuid>`; `Subvolume` maps to `btrfs subvolume create`.

```rust
pub struct CommandSpec {
    pub argv: Vec<String>,              // never a shell string
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub chroot: Option<PathBuf>,        // root to chroot into
    pub stdin: Stdin,                   // Null | Secret(SecretRef) | File(PathBuf)
}
```

No `sh -c`. If a step truly needs shell syntax, it is a reviewed exception with
`argv = ["/bin/sh", "-c", script]`, `danger = Mutating`, and a comment in the
recipe. The linter warns on every occurrence.

`Probe` values:

```rust
pub enum Probe {
    None,                               // only for strict-once steps (never re-run)
    FileExists(PathBuf),
    FileContains { path: PathBuf, needle: String },
    PathMissing(PathBuf),
    BlockFsType { device: PathBuf, fstype: String },
    SubvolumeExists { device: PathBuf, name: String },
    LayoutMatches { device: PathBuf, layout: String },   // hash of GPT plan
    CommandOk(CommandSpec),
}
```

`None` is only allowed for `Danger::Destructive` steps in `Phase::Storage`, and
the executor refuses to resume past a `None` probe unless the journal says the
step completed. This is how formatting stays safe on resume.

### 3.6 Check

Checks are declarative and shared by offline (`--root`) and online (`--ssh`)
verification where possible:

```rust
pub enum Check {
    FileExists { path: String },
    FileContains { path: String, needle: String },
    PathMissing { path: String },
    PackageInstalled { package: String },
    UnitEnabled { unit: String },
    UserExists { name: String },
    UserInGroup { name: String, group: String },
    SessionFile { id: String },
    BootEntryExists { id: String },
    CommandOk { argv: Vec<String> },
}
```

`verify` renders each check as a command in the chosen context. Package checks
use the family adapter's query syntax, so verification is family-aware without
duplicating the whole installer.

## 4. Resolver

Ordered, pure stages:

1. **Load and validate** config, recipes, matrix.
2. **Expand** the session into component ids:
   - minimal → `base`
   - desktop → `base`, `desktop.<id>`, greeter, apps
   - shell → `base`, `compositor.<id>`, `shell.<id>`, greeter, apps
3. **Gate** every component against the matrix for the family. Collect *all*
   failures, not the first, and return one error listing each with reason and
   remedy.
4. **Resolve dependencies** from `requires`, detect cycles, topologically sort.
5. **Ask the family adapter** to emit steps for each phase. The adapter may add
   family-specific prerequisite packages and repos.
6. **Add checks** from recipes and from the base contract.
7. **Canonicalise**: sort maps, number steps, assign stable ids, compute the
   digest. Topological order is stable; ties break by step id.

The resolver never runs a command and never reads a block device. Device
identity for the plan comes from a read-only probe passed in by the CLI.

## 5. Executor

`borea apply` is a state machine over the step list.

```text
preflight -> for each step:
    needs complete?           no  -> E_PLAN_ORDER
    probe satisfied?          yes -> journal skipped, continue
    danger == destructive?    yes -> re-check disk identity
    execute                   fail -> journal failed, keep mounts, stop
    journal ok
-> run checks? (optional; verify is separate)
-> finalise (write journal copy into target, print summary)
```

Invariants:

- The plan digest is recomputed from the body and must match `digest`.
- `config_digest` and `matrix_digest` must match the running binary's inputs.
  Mismatch is `E_PLAN_STALE`; the user must re-plan.
- Disk identity (serial + size + path) is captured at plan time and re-checked
  before the first destructive step. Any mismatch is `E_DISK_MISMATCH`.
- A destructive step is only legal in `Phase::Storage`, before the first
  `Phase::Bootstrap` step.
- Steps are executed in list order; `needs` is validated but does not reorder.
- The journal is append-only JSONL, fsynced after each record.
- On failure, mounts are left in place for debugging. `borea apply --cleanup`
  unmounts in reverse order. `--resume` never re-runs a step whose probe passes
  or whose journal says `ok` with a `None` probe.
- `SIGINT` is honoured at step boundaries. A running package manager is allowed
  to finish or fail; the journal records the interruption. This avoids corrupt
  package databases.
- Every command's stdout/stderr is captured to the journal as a bounded tail
  (default 64 KiB per step, configurable) after redaction.

Journal record:

```json
{"ts":"2026-11-13T10:00:00Z","plan":"...","step":"storage.partition","event":"start","argv":["sgdisk","--zap-all","/dev/vda"]}
{"ts":"...","plan":"...","step":"storage.partition","event":"ok","exit":0,"ms":412}
```

No secret value is ever written. Secret slots are rendered as `<secret:name>`.

## 6. Storage plan

v1 layout, GPT, x86_64 UEFI:

| # | Size | Type | FS | Mount | Notes |
| --- | --- | --- | --- | --- | --- |
| 1 | 1 GiB | EFI System | FAT32 | `/boot/efi` | `esp` label |
| 2 | 2 GiB | Linux | ext4 | `/boot` | unencrypted; loaders read it |
| 3 | rest | Linux / LUKS | Btrfs | `/` | ext4 when `disk.root_fs = "ext4"`; LUKS2 when `encrypt = true` |

`/boot` is the explicit ext4 exception on every family: it is small, every
loader and distro tool reads it without special support, and keeping it
unencrypted means Limine and GRUB never need LUKS2 or Btrfs read support. The
kernel and initramfs on `/boot` unlock and mount the root.

Btrfs subvolumes (default root):

| Subvolume | Mount | Why separate |
| --- | --- | --- |
| `@` | `/` | root; kernel cmdline `rootflags=subvol=@` |
| `@home` | `/home` | user data out of root snapshots |
| `@snapshots` | `/.snapshots` | reserved; no tooling in v1 (D-028) |
| `@log` | `/var/log` | logs excluded from root snapshots |
| `@cache` | `/var/cache` | cache excluded from root snapshots |

Mount options: `compress=zstd:3,noatime,ssd,discard=async` for every Btrfs
mount. The root is mounted with `subvol=@`; systemd remounts it from fstab with
the full option set after the initramfs hands over. `fsck.btrfs` is not run;
Btrfs checks are read-only and manual.

- No swap partition. `swap = "zram"` writes a `zram-generator` config (or the
  family equivalent); `swap = "file"` creates `/.swapfile` after root is
  mounted (a subvolume path, never `/swapfile` on a subvolume root).
- Root is mounted at `/mnt`, boot at `/mnt/boot`, ESP at `/mnt/boot/efi`. For
  Btrfs, the top-level volume (`subvolid=5`) is mounted at a temporary
  `/mnt/.btrfs` to create subvolumes, then unmounted before `@` is mounted at
  `/mnt`.
- `/etc/fstab` and `/etc/crypttab` are generated from UUIDs and PARTUUIDs,
  never device names. The ESP uses `PARTUUID=`; ext4/Btrfs/LUKS use `UUID=`.
- `sgdisk` creates the table; `mkfs.fat -F32 -i`, `mkfs.ext4 -U`,
  `mkfs.btrfs -U`, `btrfs subvolume create` and `cryptsetup luksFormat/open` do
  the rest. These are the distro's tools, not reimplementations.
- GPT GUIDs and filesystem/LUKS UUIDs are derived with UUIDv5 from
  `(config digest, target disk serial, role)` and passed to `sgdisk -u`,
  `mkfs.ext4 -U`, `mkfs.btrfs -U`, `mkfs.fat -i` and `cryptsetup --uuid`
  (D-029). fstab, crypttab and the loader config are therefore deterministic
  and complete at plan time.
- The partition layout is hashed into the plan. `LayoutMatches` compares the
  current GPT GUIDs/sizes before allowing a re-apply to skip partitioning.

Encryption details are per family and live in the adapter:

| Family | Packages | Initramfs | Kernel cmdline |
| --- | --- | --- | --- |
| Arch | `cryptsetup` | `encrypt` hook + `cryptdevice=UUID=...:cryptroot` | `root=/dev/mapper/cryptroot rootflags=subvol=@` |
| Debian | `cryptsetup-initramfs` | `cryptsetup` initramfs hook | `root=/dev/mapper/cryptroot rootflags=subvol=@` |
| Fedora | `cryptsetup` | dracut `rd.luks.uuid=...` | `root=/dev/mapper/cryptroot rootflags=subvol=@` |
| openSUSE | `cryptsetup` | `cryptsetup` in initrd | `root=/dev/mapper/cryptroot rootflags=subvol=@` |

With `encrypt = false` the cmdline uses `root=UUID=<root-fs-uuid> rootflags=subvol=@`
(Btrfs) or `root=UUID=<root-fs-uuid>` (ext4). The LUKS passphrase is a secret
slot (`disk.luks`). It is normally the account password when `sudo = "same"`,
but the config may reference a separate file.

The Btrfs initramfs requirement is satisfied by the family's `filesystems`
hook/module set; `btrfs-progs` is installed in the target for administration
but is not required to mount the root.

## 7. Bootloader

One policy, two adapters (D-026). `boot.loader = "auto"` resolves per family;
`limine` or `grub` may be forced where the matrix allows it. UEFI only, Secure
Boot off.

| Family | `auto` | Limine | GRUB | Why |
| --- | --- | --- | --- | --- |
| Arch | Limine | native, required | native, fallback | stable `/boot/vmlinuz-linux` names; simple static config |
| Debian | GRUB | unsupported (no package) | native | versioned kernels; `update-grub` is native |
| Fedora | GRUB | unsupported (no package) | native | BLS entries; `grub2-mkconfig` is native |
| openSUSE | GRUB | gated (O-21) | native | Btrfs/snapper-aware GRUB integration |

### 7.1 Limine (Arch)

Limine is installed from the `limine` package. The adapter:

1. copies `/usr/share/limine/BOOTX64.EFI` to the ESP at
   `/boot/efi/EFI/BOOT/BOOTX64.EFI` (firmware fallback path) and
   `/boot/efi/EFI/borea/limine_x64.efi`;
2. creates a boot entry with `efibootmgr --create --disk <disk> --part 1
   --label borea --loader '\EFI\BOOT\BOOTX64.EFI'`;
3. writes `/boot/efi/EFI/BOOT/limine.conf`, next to the EFI application. On
   EFI, Limine tries `<EFI app path>/limine.conf` first, so this is the first
   candidate it sees;
4. writes an informational copy to `/boot/limine/limine.conf` on the ext4
   `/boot` partition for offline inspection. Limine does not read this copy on
   the borea layout because it finds the ESP config first.

The config references the ext4 `/boot` filesystem as partition 2 on the boot
drive, matching Borea's fixed ESP/boot/root layout:

```text
timeout: 3
default_entry: 1

/Arch Linux
    protocol: linux
    path: boot(2):/vmlinuz-linux
    module_path: boot(2):/initramfs-linux.img
    cmdline: root=UUID=<root-fs-uuid> rootflags=subvol=@ rw

/Arch Linux (fallback initramfs)
    protocol: linux
    path: boot(2):/vmlinuz-linux
    module_path: boot(2):/initramfs-linux-fallback.img
    cmdline: root=UUID=<root-fs-uuid> rootflags=subvol=@ rw
```

No Limine hook is required on Arch because the kernel and initramfs filenames
are stable across package updates. `mkinitcpio -P` refreshes the initramfs in
place. If `linux-lts` is added later, a card extends the entry list.

Limine does not read Btrfs or LUKS: it loads the kernel and initramfs from the
unencrypted ext4 `/boot`, and the initramfs unlocks the root.

### 7.2 GRUB (Debian, Fedora, openSUSE, Arch fallback)

`grub-install`/`grub2-install` runs inside the chroot after `/proc`, `/sys` and
`/dev` are bound. `/etc/default/grub` carries the root subvolume flag:

```text
GRUB_CMDLINE_LINUX="rootflags=subvol=@"
```

`grub-mkconfig`/`grub2-mkconfig` then writes the family's config path. On
Fedora the adapter also writes `/etc/kernel/cmdline` so BLS entries inherit
`rootflags=subvol=@`. The adapter owns the path differences; plan step ids are
always `boot.grub.install` and `boot.grub.config`.

| Family | Packages | Install command | Config output |
| --- | --- | --- | --- |
| Arch | `grub efibootmgr` | `grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=borea` | `/boot/grub/grub.cfg` |
| Debian | `grub-efi-amd64` | same | `/boot/grub/grub.cfg` |
| Fedora | `grub2-efi-x64 grub2-tools` | `grub2-install ...` | `/boot/grub2/grub.cfg` |
| openSUSE | `grub2-x86_64-efi` | `grub2-install ...` | `/boot/grub2/grub.cfg` |

GRUB reads Btrfs and ext4 roots, so it needs no extra filesystem package for
the root. Verification checks that the generated config contains
`rootflags=subvol=@` (Btrfs) and the root UUID, and that the EFI fallback binary
exists.

### 7.3 Step order

`boot.select` resolves `Auto` and gates unsupported combinations during
planning. The remaining steps run after `initramfs`:

```text
boot.select -> boot.limine.install | boot.grub.install
            -> boot.limine.config   | boot.grub.config
            -> boot.verify
```

`boot.verify` is a check step, not a mutation: it asserts the EFI binary, the
config file and the cmdline content before the plan is allowed to finish.

## 8. Chroot plumbing

`borea-run` provides one `Chroot` helper used by every family:

1. bind-mount `/dev`, `/dev/pts`, `/proc`, `/sys`, `/run` into the target;
2. copy the execution host's `/etc/resolv.conf` into the target if missing;
3. run `chroot <root> <argv>` with a clean environment;
4. unmount in reverse order, tolerating busy mounts by retrying once.

This replaces `arch-chroot` and is available before any family tool is
installed. It is intentionally boring: no `systemd-nspawn`, no namespace
tricks, no daemon.

## 9. Family adapters

```rust
pub trait Family: Send + Sync {
    fn id(&self) -> FamilyId;
    fn default_release(&self) -> &'static str;
    fn host_requirements(&self) -> &'static [HostRequirement];
    fn bootstrap(&self, ctx: &Ctx) -> Result<Vec<Step>>;
    fn add_repo(&self, ctx: &Ctx, repo: &Repo) -> Result<Vec<Step>>;
    fn install(&self, ctx: &Ctx, packages: &[ResolvedPackage]) -> Result<Vec<Step>>;
    fn configure_base(&self, ctx: &Ctx) -> Result<Vec<Step>>;
    fn initramfs(&self, ctx: &Ctx) -> Result<Vec<Step>>;
    fn bootloader(&self, ctx: &Ctx) -> Result<Vec<Step>>;
    fn package_check(&self, package: &str) -> Check;
}
```

`Ctx` carries the validated config, the target root, the resolved release,
secret refs, the chosen loader and the storage layout. Adapters are stateless.
`bootloader` dispatches on `ctx.loader`, which is already resolved and gated.

`host_requirements` powers `borea doctor`: for Arch it checks `pacstrap`,
`pacman`, `sgdisk`, `mkfs.btrfs`, `mkfs.ext4`, `mkfs.fat`, `btrfs` and
`cryptsetup`, then the tools for the *resolved* loader (`limine` and
`efibootmgr`, or `grub-install`); Debian checks `debootstrap`, `apt-get`,
`mkfs.btrfs`, `mkfs.ext4`, `sgdisk`, `grub-install`, etc. `doctor` reports
missing tools with a Borea runtime provisioning remedy, not a request to boot
the target distro or install its package manager into an unrelated host. A family recipe lists the union of tools its adapters may call; `doctor`
only requires the subset the current plan will use, so a Limine install does not
demand `grub-install` and a GRUB fallback does not demand `limine`.

## 10. Session layer

`borea-session` does not know package managers. It produces abstract requests:

```rust
pub struct SessionPlan {
    pub packages: Vec<ResolvedPackage>,
    pub repos: Vec<RepoId>,
    pub services: Vec<String>,
    pub session_entries: Vec<SessionEntry>,
    pub seeds: Vec<ConfigSeed>,
    pub checks: Vec<Check>,
}
```

The resolver merges this with the family's base plan. Three session kinds:

- **Desktop**: one recipe (`desktop.kde`, `desktop.gnome`, `desktop.cosmic`),
  a greeter recipe, apps.
- **Shell**: one compositor recipe, one shell recipe, a greeter, apps.
- **Minimal**: base plus apps, no greeter.

Config seeds are files copied into `/etc/skel` and the created user's home, or
small commands run as the user. Shell setup scripts (end-4) are modelled as
`Source` builds with pinned commits and are only allowed in the `source` tier.

## 11. Secrets

```rust
pub struct SecretSlot {
    pub name: String,                  // "account.password", "disk.luks"
    pub source: SecretSource,          // File(PathBuf) | Stdin | Prompt
}
```

- The plan stores the slot name and source kind, never the value or the path's
  contents.
- `apply` resolves slots into a `SecretStore` of `Zeroizing<String>`.
- Steps that need a secret declare `Stdin::Secret(name)`; the value is written to
  the child's stdin and dropped after the step.
- `cryptsetup` uses `--key-file -`; `chpasswd` reads `user:password` from stdin;
  no password ever appears in `argv`, `env`, the journal or a log.
- A canary test in CI greps every artifact for the fixture password.

## 12. Verification

`borea verify` has two backends behind one `Check` list:

- **Offline** (`--root DIR`): reads files, chroots read-only for commands,
  queries the target package manager with `--root` where supported.
- **Online** (`--ssh TARGET`): runs a generated, quoted command per check and
  reads a single status line. Uses the system `ssh` binary; no SSH crate.

Verification is not a second installer. It only observes. If a check needs a
mutation to pass, the plan is wrong.

## 13. Determinism and schemas

- All maps in serialized structures are `BTreeMap`.
- Steps are identified by stable strings, not indices.
- `created` and `id` are excluded from `digest`.
- Canonical JSON: sorted keys, no insignificant whitespace, UTF-8, LF.
- Schema changes bump `schema` and update `fixtures/plans/*.json` in the same
  commit. `cargo xtask plan golden` fails otherwise.
- Recipe and matrix revisions are dates (`2026-11-13`) plus an upstream URL and
  commit/tag. No floating "latest".

## 14. Errors and exit codes

```rust
pub enum BoreaError {
    Usage(String),                 // 2
    Unsupported { cell: String, reason: String, remedy: String }, // 3
    Preflight(String),             // 3
    Apply { step: String, source: String }, // 4
    Verify(Vec<CheckFailure>),     // 5
    Internal(String),              // 1
}
```

Every user-facing error answers three questions: what failed, why, and the next
command to run. No raw `io::Error` reaches the terminal without context.

## 15. Logging

- Human output to stderr, progress lines only.
- `--json` emits NDJSON events for agents.
- `RUST_LOG` controls `tracing`; default `info`.
- Journal is separate from logs and is the durable record.
- Redaction is applied at the logging boundary, not at the call site.

## 16. What is intentionally absent

No plugin ABI, no dynamic recipe loading from the network, no remote catalog,
no telemetry, no update checker, no config daemon, no D-Bus, no Polkit helper,
no GUI toolkit, no Borea snapshot manager or automatic rollback (D-028; the
optional Omarchy profile retains upstream defaults under D-033), and no
bootloader outside Limine-on-Arch and GRUB (D-026). The binary is a function
from config to plan to target.

G02's separate image manifest schema 1 binds a Borea-created regular image and
the runtime pin. Its closed storage executor does not interpret draft plan
commands. See [IMAGE-EXECUTION.md](IMAGE-EXECUTION.md) and D-035.
