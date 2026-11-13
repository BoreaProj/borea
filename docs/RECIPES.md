# Recipes

> This document describes the full v0.1 target. Current executable behavior and
> remaining gates are listed in [IMPLEMENTATION.md](IMPLEMENTATION.md).


Recipes are declarative TOML data. They describe **what** a component needs;
family adapters decide **how** to install it. Adding a distro, desktop,
compositor, shell or app should be a data change plus a VM run, not a new code
path.

All recipes live under `recipes/` and are compiled into the binary with
`include_str!` at build time. There is no network catalogue and no dynamic
loading.

```text
recipes/
  matrix.toml            capability cells (source of truth)
  families/*.toml        base bootstrap per family
  repos/*.toml           third-party repositories and pinned keys
  desktops/*.toml        kde, gnome, cosmic
  compositors/*.toml     hyprland, niri, mangowm
  shells/*.toml          noctalia, dms, end4
  apps/*.toml            one file per app
  profiles/*.toml        test/config presets (minimal, kde, hyprland-noctalia)
  files/                 seeds copied into the target
```

## 1. Common fields

Every recipe starts with:

```toml
schema = 1
id = "compositor.hyprland"     # stable, dotted, never reused
kind = "compositor"            # family | desktop | compositor | shell | app
name = "Hyprland"
description = "Tiling Wayland compositor"
requires = ["component.wayland", "component.polkit"]

[evidence]
url = "https://wiki.hypr.land/Getting-Started/Installation/"
checked = 2026-11-13
```

- `id` is referenced by `requires`, `compatible_*` and the matrix.
- `kind` decides which other fields are legal; unknown fields are rejected.
- `evidence.checked` is the date a human last confirmed the upstream path.
  `matrix check` warns when it is older than 180 days.

## 2. Family recipe

`recipes/families/arch.toml`:

```toml
schema = 1
id = "family.arch"
kind = "family"
name = "Arch Linux"
release = "2026.11.01"          # ISO/mirror snapshot date, overridable in config
bootstrap = "pacstrap"

base_packages = [
  "base", "linux", "linux-firmware", "btrfs-progs", "networkmanager", "sudo",
  "cryptsetup", "zram-generator", "less", "nano",
]
microcode = { intel = "intel-ucode", amd = "amd-ucode" }

[host]
tools = ["pacstrap", "pacman", "sgdisk", "mkfs.btrfs", "mkfs.ext4", "mkfs.fat",
         "btrfs", "cryptsetup", "limine", "grub-install"]

[boot]
default_loader = "limine"       # auto resolves to this
loaders = ["limine", "grub"]    # offerable here; matrix cells must exist
limine_package = "limine"
limine_efi = "/usr/share/limine/BOOTX64.EFI"
limine_config = "/boot/efi/EFI/BOOT/limine.conf"
grub_package = "grub"
grub_install = "grub-install"
grub_mkconfig = "grub-mkconfig"
grub_cfg = "/boot/grub/grub.cfg"
initramfs = "mkinitcpio -P"
kernel_cmdline = "rootflags=subvol=@"

[evidence]
url = "https://archlinux.org/download/"
checked = 2026-11-13
```

Family fields:

| Field | Meaning |
| --- | --- |
| `release` | default release/snapshot; config may override |
| `bootstrap` | adapter id: `pacstrap`, `debootstrap`, `dnf5`, `zypper` |
| `base_packages` | minimum system, before session; includes `btrfs-progs` |
| `microcode` | CPU-vendor package map |
| `host.tools` | tools Borea must supply or provision in its independent runtime; `doctor` checks the subset the resolved plan uses (loader-aware) |
| `boot.default_loader` | what `loader = "auto"` resolves to |
| `boot.loaders` | loaders this family may offer; must match matrix cells |
| `boot.limine_*` | Arch only: package, EFI source and config path |
| `boot.grub_*` | package, install/mkconfig commands and config path |
| `boot.initramfs` | family initramfs command |
| `boot.kernel_cmdline` | flags appended to the family's normal cmdline |

Loader policy is data, not code. The resolver reads `default_loader` and
`loaders`, then checks the `boot.*` matrix cells before emitting steps (D-026).
A family that lists `limine` in `loaders` must have a `boot.limine` cell that is
`native` and a passing VM artifact; otherwise `matrix check` fails.

Storage policy is shared by every family (D-027, D-029):

| Setting | Default | Other values |
| --- | --- | --- |
| root filesystem | `btrfs` with `@`, `@home`, `@snapshots`, `@log`, `@cache` | `ext4` (explicit fallback) |
| `/boot` filesystem | `ext4`, unencrypted | none in v1 |
| Btrfs mount options | `compress=zstd:3,noatime,ssd,discard=async` | none in v1 |
| kernel cmdline | `rootflags=subvol=@` | omitted for ext4 root |
| UUIDs | UUIDv5 from `(config digest, disk serial, role)` | none in v1 |
| snapshots | layout only, no tooling | deferred (D-028, O-06) |

Family recipes add `btrfs-progs` to `base_packages`; they do not list `grub` or
`limine` in `base_packages` because the bootloader package is chosen by the
resolved loader in the boot phase. The boot phase adds the loader package plus
`efibootmgr` (Limine on Arch and GRUB on every family) so the boot entry is
created in the target, never assumed from the execution host.

## 3. Repository recipe

Third-party repositories are separate files so their keys are reviewed in one
place.

```toml
schema = 1
id = "repo.debian.noctalia"
kind = "repo"
family = "debian"
name = "Noctalia APT"
type = "apt"                    # apt | copr | obs | terra | aur | butter | zypper
url = "https://pkg.noctalia.dev/debian"
suite = "stable"
components = ["main"]
key_url = "https://pkg.noctalia.dev/debian/key.gpg"
key_fingerprint = "0123456789ABCDEF0123456789ABCDEF01234567"
priority = 500

[evidence]
url = "https://docs.noctalia.dev/noctalia/getting-started/installation/"
checked = 2026-11-13
```

Rules:

- `key_fingerprint` is mandatory for every repository that is not an official
  distro repository. `matrix check` rejects a recipe without one.
- `aur` is special: it is not a repository with a key. It means "build these
  packages from the AUR with `makepkg` as the build user, pinned by commit".
  AUR recipes carry `commit` per package. `--allow-aur` is required.
- `source` builds are described in the component recipe (`[build]`), not here.
- Repository steps are emitted before the package steps that need them.

## 4. Desktop recipe

```toml
schema = 1
id = "desktop.kde"
kind = "desktop"
name = "KDE Plasma"
description = "Plasma desktop with the KDE application set"
requires = ["component.wayland", "component.pipewire", "component.networkmanager"]

packages = { arch = [
    { name = "plasma-meta", source = "official" },
    { name = "kde-applications-meta", source = "official" },
  ], debian = [
    { name = "task-kde-desktop", source = "official" },
  ], fedora = [
    { name = "@kde-desktop-environment", source = "official" },
  ], suse = [
    { name = "patterns-kde-kde_plasma", source = "official" },
  ] }

greeter = "sddm"
session = { exec = "startplasma-wayland", desktop_file = "plasmawayland.desktop" }

[[checks]]
kind = "package"
value = "plasma-workspace"

[[checks]]
kind = "session_file"
value = "plasmawayland.desktop"
```

`packages` is keyed by family. A missing family key means the component is not
installable there; the matrix cell must then be `unsupported`. The resolver
checks this consistency.

Package entry:

```toml
{ name = "hyprland", source = "official" }
{ name = "hyprland", source = "backports" }
{ name = "dms", source = "repo.repo.fedora.dms" }
{ name = "illogical-impulse", source = "source.end4" }
```

## 5. Compositor recipe

```toml
schema = 1
id = "compositor.hyprland"
kind = "compositor"
name = "Hyprland"
requires = ["component.wayland", "component.polkit", "component.portal"]
compatible_shells = ["shell.noctalia", "shell.dms", "shell.end4"]

packages = { arch = [
    { name = "hyprland", source = "official" },
  ], debian = [
    { name = "hyprland", source = "backports" },
  ], fedora = [
    { name = "hyprland", source = "official" },
  ], suse = [
    { name = "hyprland", source = "official" },
  ] }

session = { exec = "Hyprland", desktop_file = "hyprland.desktop" }
seeds = [
  { scope = "skel", source = "files/hypr/basic.conf", dest = ".config/hypr/hyprland.conf" },
]
```

Compositor-only installs get the seed and the session entry. A shell recipe may
add its own seeds and override the compositor's startup file only through an
explicit `requires` relationship; recipes never silently overwrite each other
(`matrix check` detects two seeds with the same `dest`).

## 6. Shell recipe

```toml
schema = 1
id = "shell.noctalia"
kind = "shell"
name = "Noctalia"
description = "Quickshell-based desktop shell"
requires = ["component.quickshell", "component.fonts"]
compatible_compositors = ["compositor.hyprland", "compositor.niri", "compositor.mangowm"]

packages = { arch = [
    { name = "noctalia", source = "official" },
  ], debian = [
    { name = "noctalia", source = "repo.repo.debian.noctalia" },
  ], fedora = [
    { name = "noctalia", source = "official" },
  ], suse = [
    { name = "noctalia", source = "official" },
  ] }

service = { exec = "noctalia-shell", autostart = true }
seeds = [
  { scope = "skel", source = "files/noctalia/config.json", dest = ".config/noctalia/config.json" },
]
```

`compatible_compositors` is enforced by the resolver. A pair not listed is
refused even if both components are individually supported.

`service.autostart = true` means the session entry starts the shell; the
adapter chooses the mechanism (exec line, systemd user unit, or compositor
autostart). v1 prefers a session `Exec=` wrapper so there is one place to debug.

## 7. Source builds

Source recipes are allowed only when the matrix cell is `source` and the user
passes `--allow-source`.

```toml
[build]
kind = "git"
url = "https://github.com/end-4/dots-hyprland"
commit = "0123456789abcdef0123456789abcdef01234567"
script = "install.sh"
args = ["--yes"]
user = "builder"                # run as this user, not root
deps = { arch = ["git", "base-devel", "quickshell", "hyprland"] }
timeout_s = 1800
```

Rules:

- `commit` is a full 40-character SHA. Tags are not enough.
- The build runs in the target chroot as an unprivileged user.
- The script is fetched at that commit, its hash recorded in the journal, and
  the plan notes say the cell is experimental.
- Source builds never run on the host and never in `plan`.

## 8. App recipe

```toml
schema = 1
id = "app.firefox"
kind = "app"
name = "Firefox"
category = "browser"

packages = { arch = [{ name = "firefox", source = "official" }],
             debian = [{ name = "firefox-esr", source = "official" }],
             fedora = [{ name = "firefox", source = "official" }],
             suse = [{ name = "MozillaFirefox", source = "official" }] }

fallback = { flatpak = "org.mozilla.firefox" }
```

Resolution order per family:

1. An explicit family package, if the matrix cell is `native`/`third-party`.
2. A Flatpak fallback, if `flatpak = true` in config.
3. Otherwise the app is not offered; naming it in config is `E_UNSUPPORTED`
   with the reason.

## 9. Matrix file

```toml
schema = 1
revision = "matrix-2026-11-13.2"

[[cell]]
family = "debian"
component = "compositor.niri"
tier = "unsupported"
reason = "No niri package in Debian 13 or backports; no documented third-party repository as of 2026-11-13"
remedy = "Use Hyprland, or wait for a tracked source build (D-011)"
evidence = "https://packages.debian.org/search?keywords=niri"

[[cell]]
family = "arch"
component = "compositor.hyprland"
tier = "native"
evidence = "https://archlinux.org/packages/extra/x86_64/hyprland/"
checked = 2026-11-13

[[cell]]
family = "arch"
component = "boot.limine"
tier = "native"
evidence = "https://archlinux.org/packages/extra/x86_64/limine/"
checked = 2026-11-13

[[cell]]
family = "debian"
component = "boot.limine"
tier = "unsupported"
reason = "No Limine package in Debian 13 or backports as of 2026-11-13"
remedy = "Use loader = \"grub\"; Limine on Debian is post-v1"
evidence = "https://qa.debian.org/madison.php?package=limine"
```

Required fields by tier:

| Tier | Required |
| --- | --- |
| `native` | `evidence`, `checked` |
| `third-party` | `evidence`, `checked`, recipe `repos` |
| `source` | `evidence`, `checked`, recipe `[build]` |
| `unsupported` | `reason`, `remedy`, `evidence` |

`matrix check` validates that every recipe referenced by a profile has a cell
for each family it claims, and that a `native` cell has an official package or
backports entry in that family's map.

## 10. Profiles

Profiles are config presets for tests and examples, not a second configuration
system. `borea plan --config borea.toml` is the only user path; `xtask e2e
--profile` loads `recipes/profiles/<name>.toml` as a full config.

```toml
# recipes/profiles/hyprland-noctalia.toml
schema = 2
[target]
family = "arch"
hostname = "borea-test"
timezone = "UTC"
locale = "en_US.UTF-8"
keymap = "us"

[disk]
device = "/dev/vda"
mode = "replace"
root_fs = "btrfs"
encrypt = false

[boot]
loader = "auto"
secure_boot = false

[account]
username = "tester"
full_name = "Borea Test"
password_file = "/run/borea/user.pass"
sudo = "same"

[session]
type = "shell"
compositor = "hyprland"
shell = "noctalia"
greeter = "greetd"

[apps]
enable = []
codecs = true
flatpak = false
```

The profile's `family` is overridden by `--family` in the harness so one file
runs on all four.

## 11. Adding a component: checklist

1. Add the recipe file with `schema`, `id`, `kind`, packages for every family
   that has a path, and `[evidence]`.
2. Add a matrix cell for **every** family: `native`, `third-party`, `source` or
   `unsupported` with reason + remedy.
3. Add required repositories to `recipes/repos/` with pinned fingerprints.
4. Add seeds under `recipes/files/`.
5. Add at least one check that proves the component is present (package,
   session file, service).
6. Add a profile if the component is part of the required v0.1 set.
7. Run:

```sh
cargo xtask matrix check
cargo xtask recipe lint
cargo xtask plan golden
cargo xtask e2e --family <family> --profile <profile>
```

8. Update [`MATRIX.md`](MATRIX.md) only if the generated table changed.

## 12. Validation rules (implemented in `cargo xtask recipe lint`)

- ids are unique, dotted, lowercase, and match `^(family|repo|desktop|compositor|shell|app)\.[a-z0-9._-]+$`
- every `requires` and `compatible_*` id exists
- no dependency cycles
- every family key is one of `arch`, `debian`, `fedora`, `suse`
- every package `source` is `official`, `backports`, `repo.<id>` or `source.<id>`
- every `repo.<id>` and `source.<id>` exists
- every third-party repo has `key_fingerprint` (except `aur`/`source`)
- every seed `dest` is relative and does not escape the home/root
- every `[build].commit` is a 40-char SHA
- no two recipes seed the same `dest` in the same scope unless one `requires`
  the other
- every `checks` entry has a kind the verifier implements
- every profile validates against the config schema with no unknown fields
- every `boot.loaders` entry has a `boot.<loader>` matrix cell for that family,
  and `boot.default_loader` is one of them
- every family lists `btrfs-progs` in `base_packages`; `/boot` is not listed as
  a configurable filesystem
- `root_fs` is `btrfs` or `ext4`; `loader` is `auto`, `limine` or `grub`

## Arch Omarchy source profile

`components/profile.omarchy.toml` selects the Arch source profile; its package
map holds bootstrap prerequisites, not the complete upstream app set.
`omarchy/upstream.toml` holds the two upstream commits, entrypoint SHA-256 hashes
and observed baseline in one place. Recipe lint verifies pin syntax and that
catalogued package/webapp mappings belong to the pinned baseline. No Linutil
script is executed or used to silently mutate application config.

The profile uses config schema 2; recipe/matrix schemas stay 1. See
[OMARCHY.md](OMARCHY.md) for the explicit app-choice fields and execution limits.
