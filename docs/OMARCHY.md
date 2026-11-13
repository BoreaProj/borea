# Omarchy in the Arch family

Omarchy is an optional **Arch-family profile**, with Borea app customisations
selected explicitly by the user. Borea remains an independent installer; this
does not require Omarchy media or a target-family live environment.

**Current status:** profile resolution, app-selection previews and deterministic
plans work. The user-context Omarchy installer and real app mutations are not
VM-validated or enabled. No applications are changed by these preview commands.

## Defaults and upstream evidence

Linutil's [pinned Omarchy entry](https://github.com/ChrisTitusTech/linutil/blob/fda64ff50cd33f5cf12d5b68613f39ce6643e9d4/core/tabs/system-setup/arch/omarchy.sh)
delegates to the upstream installer. It has no separate default app-removal
preset. Borea therefore starts from the upstream Omarchy application set and
keeps it intact unless you select a change. It does not implicitly apply other
Linutil cleanup, performance, dotfile or browser scripts.

The Omarchy commit is `f4378f0de5b44d331ee943746a97872b718a6c18`;
Linutil is `fda64ff50cd33f5cf12d5b68613f39ce6643e9d4`.
`recipes/omarchy/upstream.toml` records those pins, entrypoint SHA-256 hashes and
the observed package/webapp baseline. The trim catalogue is checked against that
data. Pinning the source does not pin every package repository or nested runtime
download performed by upstream; those remain execution integration work.

At this revision, upstream Omarchy requires fresh Arch, Btrfs and Limine, and
provides Hyprland/SDDM. Its own system/snapshot configuration is part of choosing
Omarchy, rather than a Borea app customisation. Non-Arch families, ext4 root,
GRUB and conflicting desktop/shell selections are refused for this profile.
Borea's source profile requires `--allow-source` for an installation plan.

## Choose exactly what changes

Start with `examples/omarchy.toml` or `recipes/profiles/omarchy.toml`. It is a
complete config with `target.family = "arch"`, `schema = 2`, and:

```toml
[session]
type = "omarchy"
greeter = "auto"

[omarchy]
preset = "linutil"
keep = []
remove = []
keep_removed_after_updates = false
```

Those settings remove or replace **nothing**. To request changes:

```toml
[omarchy]
preset = "linutil"
keep = ["spotify", "chromium"]
remove = ["libreoffice", "docker", "whatsapp"]
default_browser = "firefox"
keep_removed_after_updates = false

[omarchy.replace]
obsidian = "neovim"
```

- `keep` records explicit preservation of catalogue apps. Unlisted apps are also
  preserved; you do not need to enumerate everything you want to keep.
- `remove` requests only the named app's mapped packages or webapp launcher.
  Docker maps to `ufw-docker`, `docker-compose`, `docker-buildx`, `docker`;
  the selection preview shows all packages before anything could execute.
- `replace` names a replacement app recipe. The plan installs replacements
  before removing the selected originals. Currently Firefox and the Arch Neovim
  app mapping can be additions; other replacements need a reviewed app recipe.
- Chromium is retained because upstream web apps need it. Changing
  `default_browser` to Firefox is explicit and does not remove Chromium.
- The same id cannot appear twice or in conflicting lists. Unknown ids,
  removing a protected browser/system component, and requesting installation of
  an explicitly removed package fail validation.
- Removal intent uses exact package names with dependency checking, without
  recursive/orphan cleanup, `--nodeps`, wildcard removal or removal of keep items.
  An unsatisfied dependency must block the operation rather than broaden it.
- The legacy `trim` spelling is accepted as an explicit removal list in config 2;
  it cannot conflict with `keep`, `remove` or `replace`.

Use `borea omarchy trim --list` to see known catalogue ids. The mapping now
includes the pinned upstream's apps such as Kdenlive, OBS Studio, Xournal++,
Signal, Spotify, Typora, 1Password and its web apps. An item absent from the
upstream baseline remains absent unless another explicit install choice adds it.

## Preview before applying

```sh
cargo run -p borea-cli -- omarchy configure \
  --config examples/omarchy.toml --json

cargo run -p borea-cli -- plan --config examples/omarchy.toml \
  --allow-source --fixture --identity fixtures/device.json \
  --out /tmp/borea-omarchy-plan.json

cargo run -p borea-cli -- apply /tmp/borea-omarchy-plan.json --dry-run
```

Use a fresh output path. `configure` is read-only: it shows each app's decision,
whether the choice was explicit, package/launcher mappings, replacements,
Chromium retention and upstream evidence. It does not probe or mutate a disk,
read passwords, write config files, download anything or run an installer.

`plan` emits a typed `InstallOmarchy` intent and explicit Borea changes after
base/boot planning. It does not fetch `boot.sh`, pipe curl into a shell, or
pretend the upstream's interactive installer is already suitable for an
unattended chroot. The runner still needs user-context integration, repository
trust, noninteractive prompt handling, reboot control and exact VM evidence.
Live apply and installation e2e remain gated. No supported matrix cell or VM
artifact is claimed.

## Keep selected removals removed

`keep_removed_after_updates = true` is an explicit opt-in. The generated
`/var/lib/borea/defaults.json` policy contains only the selected removals and
replaced originals. Keep/default items are never added to enforcement. With the
default `false`, the policy has no enforced items.

The existing defaults manager keeps its schema 1 policy model:

| State | Meaning |
| --- | --- |
| `enforced` | Selected item is to remain removed |
| `reintroduced` | Reinstalled during an update window |
| `restored` | User reinstalled it outside an update; enforcement stops |
| `blocked { reason }` | Removal failed and must not be forced |

`borea-defaults status --policy FILE`, `restore ID...`, and manual
`note-install --window update|user` recording are implemented. Automatic update
window detection, package hooks and removal/enforcement execution are still
unfinished. The opt-in policy does not claim those hooks have been installed.
A user restore remains respected by the state machine.

## Existing-system tools

`borea omarchy trim ID... --dry-run` and `trim --list` remain available. The old
trim preview shows Omarchy helper argv with the reviewed source pin. Actual
trim/enforce remains gated. The new installation plan's exact-package removal
intent is separate from upstream's recursive `omarchy-pkg-drop` helper.

The detection helper still reads the selected root's Arch/Omarchy markers.
Runtime installation detection needs further integration with current upstream
paths before real mutations are enabled. This feature does not modify the
execution host's Omarchy installation or its configuration.

## Schema and validation

Config/plan schema 2 adds the Omarchy profile and typed install/probe/check
contracts. Basic config schema 1 remains readable; Omarchy requires schema 2.
Old schema 1 plans must be regenerated. Recipe/matrix and existing defaults
policy schemas remain 1.

Regression tests cover zero changes by default, explicit-only removals,
replacement-before-removal ordering, Chromium preservation, conflicts,
source consent, non-Arch/layout refusal, read-only previews and policy scope.
`cargo xtask plan golden` includes two optional Arch Omarchy cases in addition
to the unchanged 29 required v0.1 cases. These are planning fixtures, not VM passes.
