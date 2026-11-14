# Task cards


## Implementation checkpoint — 2026-11-13

The first pass is recorded in [IMPLEMENTATION.md](IMPLEMENTATION.md). It adds
working planning/fixture/policy behavior across the existing cards. It does
**not** complete the D0 exit contract or any VM milestone. No card whose
acceptance requires a live disk, package transaction or VM is marked done.

| Cards | Preview state | Blocking evidence / next experiment |
| --- | --- | --- |
| A01–A08 | Workspace, validation, matrix, recipes, plan/CLI implemented in draft form | Full intended recipe/plan schemas and golden session installs |
| A09/A10 | Secret boundary tests and file/SSH verification implemented | Command stdin/capture canary and complete offline/online checks |
| B01–B11 | Read-only identity parser and storage/boot command planning | Image/loop tests, layout probes, chroot lifecycle and boot validation |
| C01–C07 | Four native argv adapters and base configuration intent | Live roots, package indexes/repos, family-native boot details |
| D01–D13 | Selection/package/greeter/autostart planning; unpinned paths refused | Verified pins, seeds, session entries and exact VM cells |
| E01–E05 | Catalogue preview, documented policy model/operations and process tests | Old source/upstream commit unavailable; verify mapping, update flock and removals |
| F01–F05 | Sparse image creation, cached container runner, golden/render/release gates; F03 Arch Btrfs/ext4 boot-to-login smoke | SSH verification, generic e2e orchestration, and the remaining VM matrix |

The exact unsupported errors are documented in IMPLEMENTATION.md. No checks,
keys or matrix tiers were weakened to make a missing integration report green.

One card = one branch = one reviewable diff. Cards are ordered by dependency.
Work inside the file boundary; if a card needs a change outside it, add a
comment to the card and ask the owner rather than reaching across.

Legend: **WS** workstream, **Dep** blocking cards. Verification commands assume
the repository root.

---

## F06 — Repair Pages documentation deployment
- **Files:** `.github/workflows/pages.yml`, `.github/workflows/astro.yml`,
  `_config.yml`, `index.md`, `.gitignore`, `docs/DECISIONS.md`, `docs/TESTING.md`.
- **Problem:** the Astro template fails at package-manager detection because
  this Rust project has no Astro application, package.json or JavaScript lockfile.
- **Deliverable:** publish the existing README and Markdown docs through GitHub's
  supported Jekyll Pages action, retaining deployment and checking entry points.
- **Verify:** build the Pages artifact, assert index/plan/implementation HTML
  exists, then confirm build/deploy and Rust CI on GitHub Actions.
- **Out:** adding an Astro app, changing installer code, bypassing Rust checks.

---

## E06 — Arch Omarchy profile and manual app selection
- **Files:** `crates/borea-core/src/{config,omarchy,defaults,plan,recipe}.rs`,
  `crates/borea-session/src/lib.rs`, `crates/borea-distro/src/{lib,planner}.rs`,
  `crates/borea-cli/`, `crates/borea-run/src/{omarchy,verify}.rs`,
  `crates/xtask/src/main.rs`, `recipes/`, `examples/`, golden fixtures and docs.
- **Deliverable:** Arch-only Omarchy profile following pinned Linutil/upstream
  defaults, a read-only configure preview, explicit keep/remove/replace/browser
  controls, source consent, and deterministic install/customisation intent.
- **Acceptance:** unselected apps remain untouched; conflicts and unsupported
  families/layouts fail before output; browser retention and replacement ordering
  have regression tests; schema changes and all golden fixtures are versioned.
- **Current:** planning/selection and 2 optional golden cases implemented. No
  application or host configuration was changed during development.
- **Integration blocker:** upstream install.sh is user-context and interactive;
  it changes system/boot state and dependencies. The independent runtime and
  installer runner must handle these explicitly without mid-plan prompts,
  unreviewed repository trust or automatic reboot before execution is enabled.
- **Next experiment:** exact Arch/Omarchy VM cell from Borea's independent
  runtime, followed by default-preserving and selected-only customisation runs.
- **Verify:** workspace checks; `cargo xtask plan golden`;
  `borea omarchy configure --config examples/omarchy.toml --json`;
  `cargo xtask e2e --family arch --profile omarchy` (currently unsupported).

---

## G — Independent runtime

### G01 — Standalone runtime/tool provisioning contract
- **Status:** runtime delivery implemented; four bundles published and tool
  execution smoke-tested. Full installation acceptance still depends on B/F cards.
- **Files:** `crates/borea-cli/`, `crates/borea-core/src/{runtime,error,lib}.rs`,
  `crates/borea-run/`, `runtime/`, `.github/workflows/`, `Cargo.toml`, `Cargo.lock`,
  `recipes/families/`, `crates/xtask/src/`,
  `docs/ARCHITECTURE.md`, `docs/OPERATIONS.md`, `docs/TESTING.md`.
- **Deliverable:** Borea supplies or provisions verified bootstrap, storage and
  boot tools independently of the target distro and execution host package
  manager. Define pinned artifacts, runtime paths, isolation and offline/cache
  behavior before enabling execution; keep `plan` network-free and read-only.
- **Done when:** each target family installs in a VM from an independent host
  without distro live media or a Borea-specific live environment; missing or
  unverified runtime tools fail before mutation; host package state is unchanged.
- **Verify:** independent runtime lane in `docs/TESTING.md` §8, then each exact
  family VM cell and the workspace checks. Runtime artifacts are published under
  `runtime-2026-11-13.1`; see `docs/RUNTIME.md` and the runtime validation report.
- **Remaining:** integrate the runtime with the image executor, define precise
  target/device mounts and privileges, then prove each family installs/boots in
  a VM. Tool probes do not satisfy the VM installation acceptance condition.
- **Out:** assuming target-family live media; silently installing target package
  managers into the user's host; adding ISO/USB features without a separate card.

---

## Critical path

| Day | Must finish | Can run in parallel |
| --- | --- | --- |
| D0 | A01, A03, A04, A05, A06, F01 | A02, A07, F02 |
| D1 | A08, A09, B01, B02, B03, B05, B06, B07 | A10, F03 |
| D2 | C01, C02, C06, B08, B09, B10, F04 | A10 |
| D3 | A10, C03, C04 | E01, E02 |
| D4 | C05, C07, B11 | E03, E04 |
| D5 | D01, D02, D03, D12 | D13 |
| D6 | D05, D08, D09, D11 | D06, D07 |
| D7 | D04, D10, E05 | F05 |
| D8 | matrix run, docs, release | — |

If a card is not on the critical path, it may be deferred without blocking the
release. Deferred cards stay in this file marked `deferred`.

---

## A — Core and CLI

### A01 — Workspace, toolchain, CI gate
- **WS:** A · **Dep:** none
- **Files:** `Cargo.toml`, `rust-toolchain.toml`, `.github/workflows/ci.yml`,
  `crates/*/Cargo.toml`, `crates/*/src/lib.rs`
- **Deliverable:** seven crates compile with no logic; CI runs fmt, clippy
  `-D warnings`, tests, `cargo xtask matrix check`.
- **Done when:** a fresh clone runs the CI command locally.
- **Verify:** `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings && cargo test --workspace --all-features --locked`
- **Out:** any installer logic, extra crates, nightly features.

### A02 — Errors, logging, exit codes
- **WS:** A · **Dep:** A01
- **Files:** `crates/borea-core/src/error.rs`, `crates/borea-cli/src/main.rs`
- **Deliverable:** `BoreaError` variants and exit codes from `ARCHITECTURE.md`
  §14; `tracing` init with `RUST_LOG`; `--json` NDJSON events.
- **Done when:** an unsupported cell prints reason + remedy and exits 3; a
  failed step exits 4; a failed check exits 5.
- **Verify:** `cargo test -p borea-core error && cargo test -p borea-cli exits`
- **Out:** log shipping, structured log files beyond the journal.

### A03 — Config schema and validation
- **WS:** A · **Dep:** A01
- **Files:** `crates/borea-core/src/config.rs`, `fixtures/config/*.toml`
- **Deliverable:** `Config` with `deny_unknown_fields`, defaults, and the v1
  validation rules from `ARCHITECTURE.md` §3.1.
- **Done when:** valid fixtures parse; each invalid fixture returns a specific
  error; secret values are never stored in the config object.
- **Verify:** `cargo test -p borea-core config`
- **Out:** profile merging, environment overrides, GUI editing.

### A04 — Plan model, canonical JSON, digest
- **WS:** A · **Dep:** A03
- **Files:** `crates/borea-core/src/plan.rs`, `fixtures/plans/*.json`
- **Deliverable:** types for `Plan`, `Step`, `Action`, `Probe`, `Check`,
  `SecretSlot`; canonical serialization; sha256 digest excluding `id`/`created`.
- **Done when:** round-trip is lossless; key order is stable; changing one
  field changes the digest; fixture bytes match.
- **Verify:** `cargo xtask plan golden`
- **Out:** execution, storage actions.

### A05 — Recipe schema, loader, lint
- **WS:** A · **Dep:** A01
- **Files:** `crates/borea-core/src/recipe.rs`, `recipes/**`, `crates/xtask/src/recipe.rs`
- **Deliverable:** parse all recipe kinds from embedded TOML; implement the
  validation list in `RECIPES.md` §12.
- **Done when:** a deliberately broken recipe (missing key fingerprint, cycle,
  duplicate seed) fails with a pointed message.
- **Verify:** `cargo xtask recipe lint`
- **Out:** network fetching, user-defined recipes outside the binary.

### A06 — Matrix, resolver, gates
- **WS:** A · **Dep:** A04, A05
- **Files:** `crates/borea-core/src/matrix.rs`, `crates/borea-core/src/resolver.rs`, `recipes/matrix.toml`
- **Deliverable:** matrix loader; component expansion; gate collection; stable
  topological sort; `E_UNSUPPORTED` with reason + remedy.
- **Done when:** every asserted gap in `MATRIX.md` returns a gate; a supported
  combo resolves; the resolver is pure (no I/O).
- **Verify:** `cargo test -p borea-core matrix && cargo xtask matrix check`
- **Out:** family command generation.

### A07 — CLI surface and catalogue
- **WS:** A · **Dep:** A02, A06
- **Files:** `crates/borea-cli/src/*`, `crates/borea-core/src/catalogue.rs`
- **Deliverable:** `doctor`, `catalogue`, `explain`, `status` with human and
  `--json` output.
- **Done when:** `borea catalogue --family debian` shows the exact matrix cells
  and marks untested pairs.
- **Verify:** `cargo run -p borea-cli -- catalogue --family debian --json | jq .`
- **Out:** `plan`/`apply` wiring.

### A08 — `borea plan`
- **WS:** A · **Dep:** A06, B01
- **Files:** `crates/borea-cli/src/plan.rs`, `fixtures/plans/`
- **Deliverable:** read config, gate, probe device read-only, build steps and
  checks, write plan, refuse unsupported combinations.
- **Done when:** `plan` makes no writes except `--out`; golden plan matches;
  `--dry-run` prints the step list.
- **Verify:** `cargo xtask plan golden && cargo run -p borea-cli -- plan --config fixtures/config/arch-minimal.toml --out /tmp/p.json`
- **Out:** any execution.

### A09 — Secrets
- **WS:** A · **Dep:** A03, A04
- **Files:** `crates/borea-core/src/secret.rs`, `crates/borea-run/src/secret.rs`
- **Deliverable:** secret slots, file/stdin resolution, `Zeroizing`, redaction
  at the logging and journal boundary.
- **Done when:** a canary password never appears in plan, journal, stdout,
  stderr or debug output.
- **Verify:** `cargo test -p borea-run secret_canary`
- **Out:** keyring, TPM, interactive prompt beyond a simple `rpassword` read.

### A10 — Verification engine
- **WS:** A · **Dep:** A04, C01
- **Files:** `crates/borea-core/src/check.rs`, `crates/borea-run/src/verify.rs`
- **Deliverable:** offline (`--root`) and SSH (`--ssh`) backends for every
  `Check` variant; JSON report; exit 5 on failure.
- **Done when:** a fixture root passes and a mutated root fails with the exact
  check name.
- **Verify:** `cargo test -p borea-run verify`
- **Out:** remediation, a second installer, remote agents.

---

## B — Storage and boot

### B01 — Device identity and preflight
- **WS:** B · **Dep:** A04
- **Files:** `crates/borea-distro/src/device.rs`
- **Deliverable:** read-only identity (path, serial, size, sector size, mounted
  partitions, ESP holder) from sysfs/`lsblk --json`; refuse the boot disk and
  any mounted device.
- **Done when:** a loop image and a fake sysfs fixture produce stable identity;
  a mounted device is refused with a remedy.
- **Verify:** `cargo test -p borea-distro device`
- **Out:** hotplug watching, SMART, partition editing.

### B02 — GPT partition plan
- **WS:** B · **Dep:** B01
- **Files:** `crates/borea-distro/src/storage/partition.rs`
- **Deliverable:** `sgdisk` steps for ESP/boot/root; deterministic partition GUIDs
  (UUIDv5, D-029); layout hash; `LayoutMatches` probe; `--zap-all` only after
  identity confirmation.
- **Done when:** a 20 GiB image partitions correctly twice; second run skips.
- **Verify:** `cargo xtask disk selftest --stage partition`
- **Out:** MBR, alignment tuning, alongside/resize.

### B03 — Filesystems, subvolumes and mount tree
- **WS:** B · **Dep:** B02
- **Files:** `crates/borea-distro/src/storage/fs.rs`, `btrfs.rs`, `mount.rs`
- **Deliverable:** `mkfs.fat`, `mkfs.ext4` for `/boot`, `mkfs.btrfs` plus
  `@`, `@home`, `@snapshots`, `@log`, `@cache` for `/`, deterministic UUIDs
  (D-029), Btrfs mount options, `BlockFsType`/`SubvolumeExists` probes, mount
  root/boot/ESP under `--root`, safe unmount order. `root_fs = "ext4"` follows
  the same tree without subvolumes.
- **Done when:** selftest formats and mounts Btrfs and ext4 twice; subvolumes
  exist; probes skip on re-apply.
- **Verify:** `cargo xtask disk selftest --stage mount --fs btrfs` and
  `--fs ext4`
- **Out:** xfs, swap partition, snapshot tooling (D-028).

### B04 — LUKS2
- **WS:** B · **Dep:** B03, A09
- **Files:** `crates/borea-distro/src/storage/luks.rs`
- **Deliverable:** `luksFormat`/`luksOpen`/close with key on stdin and a
  deterministic UUID, Btrfs root inside the mapper, crypttab, family initramfs
  hooks.
- **Done when:** an encrypted Btrfs image opens with the canary key and `verify`
  detects the mapper and `rootflags=subvol=@`; wrong key fails cleanly.
- **Verify:** `cargo xtask disk selftest --encrypt`
- **Out:** detached headers, TPM, FIDO2, key slots beyond one.

### B05 — fstab, crypttab, swap
- **WS:** B · **Dep:** B03
- **Files:** `crates/borea-distro/src/storage/fstab.rs`
- **Deliverable:** UUID/PARTUUID-based fstab with Btrfs `subvol=` and mount
  options, crypttab when encrypted, zram config, optional swapfile. The ESP is
  referenced by `PARTUUID=`; ext4/Btrfs/LUKS by `UUID=`.
- **Done when:** generated files are byte-stable and parse in a fixture;
  `fstab` never names a device path.
- **Verify:** `cargo test -p borea-distro fstab`
- **Out:** mount-by-label, network filesystems.

### B06 — Chroot helper
- **WS:** B · **Dep:** B03
- **Files:** `crates/borea-run/src/chroot.rs`
- **Deliverable:** bind `/dev`, `/dev/pts`, `/proc`, `/sys`, `/run`; resolv.conf;
  run argv; reverse unmount; one retry on busy.
- **Done when:** a fixture root runs `uname -a` and `/etc/os-release` reads
  correctly; mounts are gone after the call.
- **Verify:** `cargo test -p borea-run chroot`
- **Out:** namespaces, systemd-nspawn, container runtimes.

### B07 — Bootloader policy and selection
- **WS:** B · **Dep:** B06
- **Files:** `crates/borea-distro/src/boot/mod.rs`
- **Deliverable:** resolve `loader = "auto"` from the family recipe, gate
  explicit loaders against the `boot.*` matrix cells, emit `boot.select`, and
  keep step ids stable (`boot.limine.*`, `boot.grub.*`).
- **Done when:** Arch auto resolves to Limine, Debian/Fedora/openSUSE to GRUB;
  `loader = "limine"` on Debian/Fedora/openSUSE returns `E_UNSUPPORTED` with
  reason + remedy; `loader = "grub"` is accepted everywhere.
- **Verify:** `cargo test -p borea-distro boot::policy`
- **Out:** installing either loader, Secure Boot.

### B08 — Limine adapter (Arch)
- **WS:** B · **Dep:** B07
- **Files:** `crates/borea-distro/src/boot/limine.rs`
- **Deliverable:** package set (`limine`, `efibootmgr`); copy `BOOTX64.EFI` to
  the ESP fallback and `/EFI/borea`; `efibootmgr` entry; write
  `/boot/efi/EFI/BOOT/limine.conf` (first EFI candidate) plus an informational
  copy at `/boot/limine/limine.conf`, with `boot()` kernel paths into the ESP,
  the fallback initramfs entry and `rootflags=subvol=@` (or the ext4 cmdline).
- **Done when:** an Arch fixture root receives the expected EFI copy, entry and
  config bytes; a kernel package update leaves the config valid.
- **Verify:** `cargo test -p borea-distro limine && cargo xtask e2e --family
  arch --profile minimal`
- **Out:** Limine on other families, BIOS, Secure Boot, a config-regeneration
  hook.

### B09 — GRUB adapter
- **WS:** B · **Dep:** B07
- **Files:** `crates/borea-distro/src/boot/grub.rs`
- **Deliverable:** package set, `grub-install`/`grub2-install`,
  `grub-mkconfig`/`grub2-mkconfig` path per family, `/etc/default/grub` and
  Fedora `/etc/kernel/cmdline` with `rootflags=subvol=@`, EFI fallback copy.
- **Done when:** a fixture root receives the expected commands and config path;
  Debian, Fedora and openSUSE VMs boot on Btrfs.
- **Verify:** `cargo test -p borea-distro grub && cargo xtask e2e --family
  debian --profile minimal`
- **Out:** systemd-boot, rEFInd, Secure Boot, BLS parsers beyond the cmdline.

### B10 — initramfs per family
- **WS:** B · **Dep:** B04, B06
- **Files:** `crates/borea-distro/src/boot/initramfs.rs`
- **Deliverable:** `mkinitcpio -P`, `update-initramfs -u -k all`, `dracut -f`,
  `mkinitrd`, with the Btrfs module, encryption hooks and cmdline each needs.
- **Done when:** Btrfs and encrypted VMs boot to the root mount; `/proc/cmdline`
  contains `rootflags=subvol=@` and the expected root device.
- **Verify:** `cargo xtask e2e --family arch --profile minimal-encrypted`
- **Out:** UKI, unified kernel images, custom initramfs.

### B11 — ext4-root and GRUB fallback cells
- **WS:** B · **Dep:** B03, B09, F04
- **Files:** `crates/xtask/src/e2e.rs`, `recipes/profiles/*`
- **Deliverable:** `--root-fs ext4` and `--loader grub` overrides; one ext4-root
  minimal cell per family plus the Arch-GRUB cell; artifacts indexed.
- **Done when:** all five fallback cells are green and `matrix verify --strict`
  counts them.
- **Verify:** `cargo xtask e2e --family arch --profile minimal --root-fs ext4
  --loader grub`
- **Out:** xfs, zfs, dual-boot.

---

## C — Families

### C01 — Family trait, registry, host requirements
- **WS:** C · **Dep:** A04
- **Files:** `crates/borea-core/src/family.rs`, `crates/borea-distro/src/lib.rs`
- **Deliverable:** the trait from `ARCHITECTURE.md` §9; registry; `doctor`
  requirements per family.
- **Done when:** selecting an unknown family is a usage error; `doctor` lists
  missing tools with a Borea runtime provisioning remedy (G01).
- **Verify:** `cargo test -p borea-distro registry`
- **Out:** any concrete adapter.

### C02 — Arch adapter
- **WS:** C · **Dep:** C01, B02, B05
- **Files:** `crates/borea-distro/src/arch.rs`, `recipes/families/arch.toml`
- **Deliverable:** `pacstrap` bootstrap, pacman install/query, mirror check,
  microcode, base packages (`btrfs-progs`), `mkinitcpio`, loader packages from
  the resolved boot policy.
- **Done when:** Arch minimal VM boots and verifies.
- **Verify:** `cargo xtask e2e --family arch --profile minimal`
- **Out:** AUR helper, arch-chroot.

### C03 — Debian adapter
- **WS:** C · **Dep:** C02
- **Files:** `crates/borea-distro/src/debian.rs`, `recipes/families/debian.toml`
- **Deliverable:** `debootstrap` supplied/provisioned by the independent runtime (G01),
  sources.list, backports, apt install/query, initramfs.
- **Done when:** Debian minimal VM boots and verifies.
- **Verify:** `cargo xtask e2e --family debian --profile minimal`
- **Out:** Ubuntu, derivatives, testing/sid beyond the configured suite.

### C04 — Fedora adapter
- **WS:** C · **Dep:** C02
- **Files:** `crates/borea-distro/src/fedora.rs`, `recipes/families/fedora.toml`
- **Deliverable:** `dnf5 --installroot`, releasever, repo config, weak-deps off,
  `/etc/fstab`, `btrfs-progs`, initramfs, grub2 paths and `/etc/kernel/cmdline`.
- **Done when:** Fedora minimal VM boots and verifies.
- **Verify:** `cargo xtask e2e --family fedora --profile minimal`
- **Out:** Silverblue/atomic, rpm-ostree.

### C05 — openSUSE adapter
- **WS:** C · **Dep:** C02
- **Files:** `crates/borea-distro/src/suse.rs`, `recipes/families/suse.toml`
- **Deliverable:** `zypper --root` repo add/refresh/install, patterns,
  `btrfs-progs`, initrd, grub2 paths, Btrfs root layout.
- **Done when:** openSUSE minimal VM boots and verifies.
- **Verify:** `cargo xtask e2e --family suse --profile minimal`
- **Out:** Leap vs Tumbleweed matrix beyond the pinned snapshot, YaST.

### C06 — Base target configuration
- **WS:** C · **Dep:** C02, B06
- **Files:** `crates/borea-distro/src/base.rs`
- **Deliverable:** hostname, locale, timezone, keymap, user + groups, sudoers
  drop-in, root lock, NetworkManager enable, machine-id, journald basics.
- **Done when:** verify checks pass for each item on a fixture root and a VM.
- **Verify:** `cargo test -p borea-distro base && cargo xtask e2e --family arch --profile minimal`
- **Out:** SSH server by default, shell dotfiles beyond `/etc/skel`.

### C07 — Repository adapters and key pinning
- **WS:** C · **Dep:** C03, C04, C05
- **Files:** `crates/borea-distro/src/repo/{apt,copr,obs,zypper,aur,butter}.rs`
- **Deliverable:** add/refresh a repo, import only the pinned key, verify the
  fingerprint, fail closed; AUR build as unprivileged user with pinned commit.
- **Done when:** a wrong fingerprint is refused before any package install.
- **Verify:** `cargo test -p borea-distro repo::fingerprint`
- **Out:** automatic key discovery, trust-on-first-use.

---

## D — Sessions

### D01 — Session resolver, greeter, session entries, seeds
- **WS:** D · **Dep:** A06, C06
- **Files:** `crates/borea-session/src/lib.rs`, `session.rs`, `seed.rs`
- **Deliverable:** merge compositor/shell/desktop requests; pick greeter;
  write `/usr/share/wayland-sessions/*.desktop`; copy seeds to `/etc/skel` and
  the user's home with correct ownership.
- **Done when:** a fixture plan has one session entry, one greeter, no duplicate
  seed destinations.
- **Verify:** `cargo test -p borea-session`
- **Out:** display manager theming, autologin.

### D02 — KDE recipe
- **WS:** D · **Dep:** D01
- **Files:** `recipes/desktops/kde.toml`
- **Deliverable:** packages per family, SDDM, Plasma Wayland session, checks.
- **Done when:** KDE VM reaches SDDM and a Plasma session.
- **Verify:** `cargo xtask e2e --family arch --profile kde`
- **Out:** KDE theming, PIM, discover integration.

### D03 — GNOME recipe
- **WS:** D · **Dep:** D01
- **Files:** `recipes/desktops/gnome.toml`
- **Deliverable:** packages per family, GDM, GNOME session, checks.
- **Done when:** GNOME VM reaches GDM and a GNOME session.
- **Verify:** `cargo xtask e2e --family arch --profile gnome`
- **Out:** extensions, dash-to-dock, GNOME tweaks.

### D04 — COSMIC recipe and gate
- **WS:** D · **Dep:** D01
- **Files:** `recipes/desktops/cosmic.toml`, matrix cells
- **Deliverable:** Arch/Fedora native, openSUSE OBS third-party, Debian gate.
- **Done when:** Arch and Fedora cells boot; Debian returns `E_UNSUPPORTED`
  with the documented reason.
- **Verify:** `cargo xtask e2e --family fedora --profile cosmic`
- **Out:** COSMIC apps beyond the session, Debian source build.

### D05 — Hyprland
- **WS:** D · **Dep:** D01
- **Files:** `recipes/compositors/hyprland.toml`, `recipes/files/hypr/`
- **Deliverable:** packages for all four (Debian backports), basic config,
  session entry, portal/xwayland deps.
- **Done when:** all four compositor-only cells boot into a usable Hyprland
  session.
- **Verify:** `cargo xtask e2e --family debian --profile hyprland`
- **Out:** a full dotfiles experience, plugins, NVIDIA tuning.

### D06 — Niri
- **WS:** D · **Dep:** D05
- **Files:** `recipes/compositors/niri.toml`, `recipes/files/niri/`
- **Deliverable:** native on Arch/Fedora/openSUSE; Debian gate.
- **Done when:** three cells boot; Debian gate is tested.
- **Verify:** `cargo xtask e2e --family suse --profile niri`
- **Out:** Debian source build (D-011).

### D07 — MangoWM
- **WS:** D · **Dep:** D05
- **Files:** `recipes/compositors/mangowm.toml`, `recipes/files/mango/`
- **Deliverable:** Arch native, Fedora Terra, Debian ButterRepo; openSUSE gate.
- **Done when:** three cells boot; openSUSE gate is tested; ButterRepo wlroots
  isolation is documented.
- **Verify:** `cargo xtask e2e --family debian --profile mangowm`
- **Out:** openSUSE source build (D-012).

### D08 — Noctalia
- **WS:** D · **Dep:** D05
- **Files:** `recipes/shells/noctalia.toml`, `recipes/repos/*noctalia*`, `recipes/files/noctalia/`
- **Deliverable:** all four families; Debian APT repo; autostart; checks.
- **Done when:** all four Hyprland+Noctalia cells boot to a visible shell.
- **Verify:** `cargo xtask e2e --family debian --profile hyprland-noctalia`
- **Out:** shell theming, plugin ecosystem, shell upgrades.

### D09 — DUX / Dank Material Shell
- **WS:** D · **Dep:** D05
- **Files:** `recipes/shells/dms.toml`, `recipes/repos/*dms*`, `recipes/files/dms/`
- **Deliverable:** Arch native, Fedora COPR, openSUSE OBS, Debian OBS +
  backports quickshell; autostart; checks.
- **Done when:** all four Hyprland+DMS cells boot.
- **Verify:** `cargo xtask e2e --family suse --profile hyprland-dms`
- **Out:** DMS plugins, migration from DMS v1 config.

### D10 — end-4 source build
- **WS:** D · **Dep:** D05, C07
- **Files:** `recipes/shells/end4.toml`, `recipes/files/end4/`
- **Deliverable:** pinned installer as `source` tier on Arch; gates elsewhere;
  `--allow-source`; VM run.
- **Done when:** Arch end-4 boots into Hyprland with the shell; gates are
  tested; the pinned commit and script hash are in the plan.
- **Verify:** `cargo xtask e2e --family arch --profile end4 --allow-source`
- **Out:** Fedora community path, Nix, Debian/openSUSE.

### D11 — greetd and session entries
- **WS:** D · **Dep:** D05
- **Files:** `recipes/greeters/greetd.toml`, `crates/borea-session/src/greeter.rs`
- **Deliverable:** greetd + tuigreet for compositor/shell profiles; SDDM/GDM for
  desktops; conflict detection.
- **Done when:** only one greeter unit is enabled; login starts the chosen
  session.
- **Verify:** `cargo xtask e2e --family arch --profile hyprland --assert greeter`
- **Out:** graphical greeter themes, autologin.

### D12 — Apps, Flatpak, codecs
- **WS:** D · **Dep:** C07
- **Files:** `recipes/apps/*.toml`, `crates/borea-session/src/apps.rs`
- **Deliverable:** per-app matrix resolution, Flatpak fallback, Flathub setup,
  codec repo selection (RPM Fusion, Packman).
- **Done when:** a fixture profile installs Firefox + Steam and skips an
  unsupported app with a reason.
- **Verify:** `cargo test -p borea-session apps && cargo xtask e2e --family fedora --profile kde --with-apps`
- **Out:** app configuration, gaming tweaks, proprietary drivers.

### D13 — Seeds and skel
- **WS:** D · **Dep:** D01
- **Files:** `crates/borea-session/src/seed.rs`, `recipes/files/`
- **Deliverable:** copy seeds with owner/mode, no escaping paths, idempotent.
- **Done when:** a second apply changes nothing; a `../` dest is rejected.
- **Verify:** `cargo test -p borea-session seed`
- **Out:** dotfile managers, chezmoi, user-supplied archives.

---

## E — Omarchy

### E01 — Trim catalogue port
- **WS:** E · **Dep:** A04
- **Files:** `crates/borea-core/src/omarchy/mod.rs`, `trim.rs`
- **Deliverable:** port `TrimItem`, `trim_action`, labels and group data from the
  old `omarchy.rs`; pin `OMARCHY_COMMIT`; refuse items not preinstalled.
- **Done when:** every old item maps to a package, web app or
  `NotPreinstalled`; a table test covers all variants.
- **Verify:** `cargo test -p borea-core omarchy::trim`
- **Out:** Omarchy installation, theme management.

### E02 — Omarchy detection and CLI
- **WS:** E · **Dep:** E01
- **Files:** `crates/borea-cli/src/omarchy.rs`
- **Deliverable:** `borea omarchy trim`, `status`; detect Omarchy by
  `/etc/omarchy` + `omarchy` command; require root for trim.
- **Done when:** on a non-Omarchy root it refuses with a clear message; on a
  fixture root it emits the expected `omarchy-pkg-drop`/`omarchy-webapp-remove`
  steps.
- **Verify:** `cargo test -p borea-cli omarchy`
- **Out:** install, update, config editing.

### E03 — Defaults policy and manager
- **WS:** E · **Dep:** E01
- **Files:** `crates/borea-core/src/defaults.rs`, `crates/borea-defaults/src/*`
- **Deliverable:** port `DefaultsPolicy` schema 1 (readable by old policies);
  `status`, `trim`, `restore`, `enforce`, `note-install`; fake package manager
  test with real file locks.
- **Done when:** the process-level test drives the real binary: trim, update
  reinstalls, enforce removes, user reinstall is respected, blocked is reported.
- **Verify:** `cargo test -p borea-defaults --locked`
- **Out:** apt/dnf/zypper update windows wired to a live system (design only).

### E04 — Hooks and package-manager adapters
- **WS:** E · **Dep:** E03
- **Files:** `crates/borea-defaults/src/hooks/*`, `crates/borea-defaults/src/pkg/*`
- **Deliverable:** pacman hook + Omarchy post-update hook; apt/dnf/zypper
  adapters behind the same trait; no forced removal.
- **Done when:** a package that now has a dependent is `blocked`, not removed.
- **Verify:** `cargo test -p borea-defaults pkg`
- **Out:** systemd timers, daemons, automatic enforcement outside the update.

### E05 — Omarchy fixtures and e2e
- **WS:** E · **Dep:** E02, E03
- **Files:** `fixtures/omarchy/*`, `crates/xtask/src/e2e.rs`
- **Deliverable:** fixture Omarchy root; optional Omarchy VM profile; trim +
  defaults acceptance run.
- **Done when:** fixture run passes; VM run documented (or explicitly skipped
  with a reason).
- **Verify:** `cargo xtask e2e --profile omarchy-trim`
- **Out:** Omarchy ISO download in CI by default (manual profile only).

---

## F — Harness and CI

### F01 — Disk image and loop helper
- **WS:** F · **Dep:** A01
- **Files:** `crates/xtask/src/disk.rs`
- **Deliverable:** create sparse raw image, attach loop with `losetup
  --partscan`, detach, cleanup on panic; never touch a non-image device.
- **Done when:** two parallel runs use distinct loop devices and clean up.
- **Verify:** `cargo xtask disk selftest --stage create`
- **Out:** qcow2, NBD, real disks.

### F02 — Family containers
- **WS:** F · **Dep:** A01
- **Files:** `xtask/containers/*.Containerfile`, `crates/xtask/src/envrun.rs`
- **Deliverable:** one container per family with its bootstrap tools; run
  `borea apply` inside with the loop device and workspace mounted.
- **Done when:** `cargo xtask env --family debian -- borea doctor` passes.
- **Verify:** `cargo xtask env --family debian -- borea doctor --json`
- **Out:** rootless magic, Podman-in-Podman.

### F03 — QEMU boot and SSH verify
- **WS:** F · **Dep:** F01
- **Files:** `crates/xtask/src/{main,vm}.rs`,
  `.github/workflows/image.yml`,
  `docs/{IMAGE-EXECUTION,IMPLEMENTATION,TASKS,TESTING}.md`
- **Deliverable:** boot a raw disk with OVMF, forward SSH, inject a test key,
  wait for the system, run `borea verify --ssh`, capture serial log and exit
  code.
- **Current:** the disposable-image workflow boots Arch Btrfs and ext4 images
  with OVMF and checks the expected root filesystem, kernel root arguments and
  serial login prompt. SSH verification and the corrupted-image negative case
  remain pending; this partial smoke does not support a matrix cell.
- **Done when:** a known-good image passes; a corrupted image fails with the
  serial log attached.
- **Verify:** `cargo xtask vm boot --image /absolute/minimal.img --fs btrfs --serial-log /absolute/minimal.serial.log`
- **Out:** graphical console testing, screenshot comparison.

### F04 — e2e orchestrator and artifacts
- **WS:** F · **Dep:** F01, F02, F03
- **Files:** `crates/xtask/src/e2e.rs`
- **Deliverable:** `create → plan → apply → boot → verify → collect`, per-cell
  artifact directory (`target/e2e/<family>/<profile>/<timestamp>/`), pass/fail
  JSON.
- **Done when:** one command runs a cell and leaves plan, journal, serial log
  and verify report.
- **Verify:** `cargo xtask e2e --family arch --profile minimal`
- **Out:** parallel scheduler, cloud runners.

### F05 — Matrix runner, render and nightly CI
- **WS:** F · **Dep:** F04
- **Files:** `crates/xtask/src/matrix.rs`, `.github/workflows/nightly.yml`
- **Deliverable:** `matrix check`, `matrix render` (regenerate `MATRIX.md`
  tables), `matrix verify --strict` (run required cells, compare artifacts,
  refuse a green cell without one); nightly rotation; artifact upload.
- **Done when:** a missing artifact fails the release gate and `matrix render`
  leaves the docs unchanged on a clean tree.
- **Verify:** `cargo xtask matrix render --check && cargo xtask matrix verify --strict`
- **Out:** paid runners for every required cell nightly (start with a subset).

---

## Deferred (explicitly not v0.1)

| Card | Why deferred | Trigger |
| --- | --- | --- |
| Debian Niri source build | heavy Rust build, no upstream support | upstream package or D-011 accepted |
| openSUSE MangoWM source build | wlroots/scenefx mismatches | upstream package or D-012 accepted |
| Debian COSMIC source build | large Rust/Qt stack, no packaging | upstream Debian packaging |
| Fedora end-4 | community path unverified | maintained COPR |
| Secure Boot | key custody and enrolment story missing | O-05 accepted |
| Snapshot tooling and rollback | v0.1 creates `@snapshots` but installs no manager (D-028) | O-06 accepted |
| Limine on openSUSE | packaged, but GRUB is better integrated and no VM cell exists (O-21) | O-21 accepted + VM cell |
| alongside/resize | dangerous, large | after v1 |
| GUI | explicitly removed | never in this repo |

### G02 — Closed image storage executor
- **Scope:** Borea-created root-owned raw images only; private verified runtime
  snapshot; held image/loop descriptors; GPT, ext4/Btrfs, subvolumes, fstab,
  cleanup, durable receipts and probe-based repeat execution.
- **Out:** physical disks, arbitrary plan commands, encryption, bootstrap and
  bootloaders. These remain gated pending separate execution/VM validation.
- **Verify:** privileged disposable-image CI for both filesystems; rejection
  and stale-identity tests; existing workspace checks.

### G03 — Closed image Arch bootstrap and Limine bootloader executor
- **Release:** B-0.3 (workspace version 0.3.0).
- **Scope:** Borea-created raw images with verified G02 storage layout;
  pinned Arch runtime bundle; root-private snapshot with shared host networking
  for package retrieval; execute `pacstrap` to install base packages (`base`,
  `linux`, `linux-firmware`, `btrfs-progs`, `networkmanager`, `sudo`, `zram-generator`,
  `openssh`, `limine`, `efibootmgr`); target chroot configuration (fstab,
  hostname, hosts, locale-gen, timezone, user, sudoers, NetworkManager/sshd services);
  initramfs generation (`mkinitcpio -P`); Limine EFI binary install and
  `limine.conf` config generation; ordered durable receipts and complete unmount cleanup.
- **Out:** non-Arch distro bootstraps (Debian/Fedora/openSUSE image bootstrap remain
  subsequent cards), physical disks, encrypted root, Omarchy script runner.
- **Verify:** privileged disposable-image bootstrap CI validates the target and
  bootloader configuration with repeat probes. This is not a VM boot result;
  QEMU/SSH acceptance remains separate.

### G04 — Secure Arch image bootstrap credentials
- **Release intent:** B-0.4; keep the workspace at 0.3.0 until this milestone
  and its release checks are complete.
- **Dep:** G03.
- **Files:** `crates/borea-cli/src/main.rs`,
  `crates/borea-run/src/image.rs`, `crates/borea-run/src/image-bootstrap.sh`,
  `crates/xtask/src/main.rs`, `crates/borea-cli/tests/cli.rs`,
  `docs/{DECISIONS,IMAGE-EXECUTION,IMPLEMENTATION,RUNTIME,TASKS}.md`.
- **Deliverable:** require a bounded, private, non-symlink password file for
  the Arch bootstrap account; transport its contents only over stdin; never
  include the value in argv, environment, receipts or diagnostics. Lock root,
  require a password for sudo, and report bootloader configuration separately
  from VM boot verification.
- **Done when:** hard-coded credentials and `NOPASSWD` are absent; targeted
  tests cover password-file validation, redaction and the mandatory CLI option;
  privileged disposable-image CI exercises both filesystems with a canary that
  is absent from receipts and output. Legacy G03 receipts are refused, so old
  images cannot be resumed as if they were secured. No VM boot is claimed by
  this card.
- **Verify:** `cargo test -p borea-run image::tests`,
  `cargo test -p borea-cli --test cli image_bootstrap_requires_a_password_file`,
  `sudo env BOREA_RUNTIME_CACHE=/absolute/runtime-cache target/release/xtask disk bootstrap --fs btrfs`
  and the same command with `--fs ext4`; existing workspace checks.
- **Out:** QEMU/SSH boot acceptance, LUKS2, and non-Arch image bootstrap.
