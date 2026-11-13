# Decisions

Accepted decisions are binding until superseded by a later entry. Open
decisions are not silently inferred; each has a default and a trigger.

Format: `D-nnn` accepted, `O-nnn` open.

---

## Accepted

### D-001 — Build a clean-room v2, leave the old repository alone
**Date:** 2026-11-13
**Decision:** New repository `borea-v2`. The old `borea` tree, branches and
uncommitted work are reference material only.
**Why:** The old tree carries a live-ISO builder, GUI, daemon and simulation
engine that are out of scope. Cleaning it in place would cost more than starting
small and porting the two pieces worth keeping (trim catalogue, defaults policy).
**Rejected:** refactor in place; delete the old repo.

### D-002 — CLI only
**Decision:** The product surface is `borea` with text and JSON output. No GUI,
TUI dashboard, recorder or design system.
**Why:** The install path was never the bottleneck; the interface was. Agents
and operators can drive a CLI; screenshots cannot install a system.
**Rejected:** iced/Slint GUI, ratatui dashboard, web UI.

### D-003 — Run inside the distro's own live/rescue environment
**Superseded by:** D-031 (independent standalone Borea runtime).
**Decision:** No live ISO, initramfs, USB writer or boot menu. The user boots
the family's media (or a prepared environment) and runs `borea`.
**Why:** Every family already ships a tested environment with its bootstrap
tool. Rebuilding that is a second operating system.
**Rejected:** custom live image, Buildroot, embedded installer.

### D-004 — Use the distro's native tools; drop the pure-Rust constraint
**Decision:** `pacstrap`, `debootstrap`, `dnf5 --installroot`, `zypper --root`,
`sgdisk`, `mkfs.*`, `btrfs`, `cryptsetup`, `mount`, `grub-install`, `limine`,
`efibootmgr`, `chroot`.
**Why:** These tools encode years of distro-specific edge cases. Reimplementing
them in Rust is how the old project stalled.
**Rejected:** pure-Rust GPT/FAT/ext4/LUKS implementations; a custom package
manager.

### D-005 — Plan / apply / verify is the product contract
**Decision:** `plan` emits a deterministic JSON document; `apply` executes it;
`verify` checks the result. The plan is the API and the test fixture.
**Why:** It makes every install inspectable, diffable, resumable and testable
without a simulator.
**Rejected:** imperative install script, hidden state machine, daemon RPC.

### D-006 — Recipes are declarative TOML
**Decision:** Families, repos, desktops, compositors, shells, apps and profiles
are data compiled into the binary. Code only implements adapters and the
resolver.
**Why:** Adding a component should not require touching install logic. Reviewers
can audit data and evidence.
**Rejected:** per-distro code paths, YAML, a scripting language, remote catalog.

### D-007 — Capability matrix with four tiers and hard gates
**Decision:** Every (family, component) cell is `native`, `third-party`,
`source` or `unsupported`. Unsupported cells refuse with reason + remedy.
**Why:** Honesty is a feature. A hidden or fake option destroys trust and wastes
agent time.
**Rejected:** "best effort" warnings, hiding cells silently, a single boolean
"supported".

### D-008 — UEFI x86_64, single disk, replace only
**Decision:** GPT, one disk, `DiskMode::Replace`. No alongside, resize, MBR,
ARM or BIOS in v1.
**Why:** These are the safest, most testable defaults. Dual-boot is a separate
product decision with its own risk.
**Rejected:** dual-boot, automatic shrinking, multi-disk LVM.

### D-009 — GRUB on every family
**Superseded by:** D-026 (2026-11-13).
**Decision:** One bootloader adapter, GRUB, with family-specific package names
and config paths.
**Why:** Four bootloaders is four times the failure surface. GRUB works on all
four and reads an unencrypted `/boot`.
**Rejected:** systemd-boot, BLS, per-family defaults, rEFInd.

### D-010 — ext4, unencrypted `/boot`, optional LUKS2 root
**Superseded by:** D-027 (2026-11-13).
**Decision:** Three partitions (ESP, boot, root); ext4 root; LUKS2 on root only
when `encrypt = true`.
**Why:** Simple, debuggable, and compatible with GRUB. Encryption remains
optional and testable per family.
**Rejected:** btrfs + snapshots, encrypted `/boot`, ZFS, LVM.

### D-011 — Debian + Niri is gated in v0.1
**Decision:** `unsupported` on Debian. Reason and remedy recorded in
`MATRIX.md`.
**Why:** No Debian package or documented repository as of 2026-11-13. A source
build is a separate, heavy task.
**Revisit:** an official/backports package appears, or `O-01` is accepted.

### D-012 — openSUSE + MangoWM is gated in v0.1
**Decision:** `unsupported` on openSUSE.
**Why:** No Tumbleweed package or documented OBS project verified; upstream
targets Arch/Fedora and the wlroots/scenefx versions are fragile.
**Revisit:** a maintained package or `O-02` is accepted.

### D-013 — Debian + COSMIC is gated in v0.1
**Decision:** `unsupported` on Debian.
**Why:** No Debian packaging path found. COSMIC is a large Rust/Qt stack; a
source build is not a days-scale task.
**Revisit:** Debian packaging appears or `O-03` is accepted.

### D-014 — end-4 is Arch-only in v0.1
**Decision:** `source` on Arch (pinned installer); `unsupported` on Debian,
Fedora and openSUSE.
**Why:** Upstream documents Arch/Nix. Fedora community packaging is unverified;
Debian/openSUSE paths are WIP.
**Revisit:** a maintained Fedora COPR, Debian repo or openSUSE package.

### D-015 — Omarchy tools operate on an existing install only
**Superseded by:** D-033 for the Arch Omarchy profile; existing-system tools remain available.
**Decision:** `borea omarchy trim` and `borea omarchy defaults` require a
detected Omarchy system. Installing Omarchy is out of scope.
**Why:** Omarchy ships its own ISO and installer. Duplicating it is the old
project's mistake.
**Rejected:** Omarchy install path, Omarchy ISO builder, theme manager.

### D-016 — One process, no daemon
**Decision:** `apply` is a foreground process with a journal file. No socket,
service, worker or supervisor.
**Why:** Resume works from the journal; a daemon adds lifecycle, auth and
debugging cost for no installer benefit.
**Rejected:** Varlink, D-Bus, systemd worker, background prefetch.

### D-017 — Secrets travel on stdin, never in argv or the plan
**Decision:** Secret slots resolve to `Zeroizing` values passed to child stdin;
plans and journals store only slot names.
**Why:** `ps`, shell history, logs and crash dumps leak argv. A canary test
enforces this.
**Rejected:** environment variables, plan fields, command-line flags.

### D-018 — Package source precedence
**Decision:** official → official backports → pinned third-party → Flatpak
fallback. Source builds require `--allow-source` and a `source` matrix cell.
**Why:** Fewest moving parts and best upgrade story first.
**Rejected:** AUR-first, Flatpak-first, building everything from source.

### D-019 — Every step has an idempotency probe
**Decision:** A step without a probe is invalid unless it is destructive and in
the storage phase, where the journal alone authorises a skip.
**Why:** Resume and re-apply are core features. Untestable steps become
duplicate users, duplicate repos and broken bootloaders.
**Rejected:** "run once" markers scattered in shell, best-effort re-runs.

### D-020 — Containers for iteration, VMs for truth
**Decision:** Fast tests run in per-family containers against image files; a
cell is green only after a QEMU boot + SSH verify.
**Why:** Containers are fast enough for agents; VMs catch boot, kernel,
initramfs and firmware problems. The old simulator caught neither.
**Rejected:** simulation engine, host-only tests, manual installs as evidence.

### D-021 — No new dependency without a decision entry
**Decision:** A new crate or external runtime tool needs a `D-nnn` entry with
the reason and the alternative considered.
**Why:** Dependency sprawl is how a small installer becomes a platform.
**Rejected:** "it's just one crate".

### D-022 — Release as a static x86_64 binary
**Decision:** Build `borea` and `borea-defaults` for
`x86_64-unknown-linux-musl`, plus a `.sha256`. No installer script that pipes
the network into a shell.
**Why:** The standalone binary must run independently of the selected target
family without matching its glibc. Runtime tool provisioning is tracked in G01
and D-031; static linking alone does not supply external tools.
**Rejected:** distro packages first, curl-to-shell, container-only distribution.

### D-023 — One card, one branch, one owner
**Decision:** Agents work one task card at a time inside the file boundary in
`TASKS.md`. Cross-boundary changes go through the owner.
**Why:** Parallel Claude/Codex work without collisions.
**Rejected:** free-form parallel edits, a single shared branch.

### D-024 — Preserve the old repository
**Decision:** Do not delete, rewrite or force-push `/home/luke/Work/borea` or its
worktrees. Port only the trim catalogue and defaults policy, with attribution in
the commit message.
**Why:** It contains the user's uncommitted work and the design history.

### D-025 — Approved dependency set
**Decision:** The following crates are pre-approved; anything else needs a new
decision entry.
`clap` (derive), `serde`, `serde_json`, `toml`, `thiserror`, `tracing`,
`tracing-subscriber`, `sha2`, `zeroize`, `uuid`, `time`, `which`, `rpassword`.
Dev/test only: `tempfile`, `assert_cmd`, `predicates`, `insta`.
**Why:** Enough to build the CLI, schemas, logging, digests and tests without
turning the installer into a dependency graph.
**Rejected:** async runtimes (`tokio`), HTTP clients, GUI toolkits, database
crates, SSH crates.

### D-026 — Limine on Arch, GRUB on the other three
**Date:** 2026-11-13
**Decision:** `boot.loader = "auto"` resolves to Limine on Arch and GRUB on
Debian, Fedora and openSUSE. `loader = "limine"` is accepted only on Arch in
v0.1: Debian and Fedora ship no Limine package, and openSUSE packages it but
v0.1 does not offer it because GRUB is the tested Btrfs/snapper-aware path
there. `loader = "grub"` is accepted everywhere, including Arch as the fallback.
**Why:** Arch keeps stable `/boot/vmlinuz-linux` and
`/boot/initramfs-linux.img` names, so a static Limine config survives kernel
updates without a hook. Limine only reads the unencrypted ext4 `/boot`; the
initramfs unlocks LUKS2 and mounts Btrfs, so the loader needs neither. Debian,
Fedora and openSUSE use versioned kernels plus native GRUB integration (Fedora
BLS, openSUSE snapper), so regenerating entries with GRUB is the better path.
**Rejected:** GRUB everywhere (supersedes D-009), Limine everywhere,
systemd-boot, a Limine hook that rewrites the config on every kernel update.

### D-027 — Btrfs root by default, ext4 for `/boot` and as an explicit fallback
**Date:** 2026-11-13
**Decision:** Root is Btrfs with subvolumes `@` (root), `@home`, `@snapshots`,
`@log` and `@cache`; mount options `compress=zstd:3,noatime,ssd,discard=async`;
the kernel cmdline carries `rootflags=subvol=@`. `/boot` is ext4 on every
family. `disk.root_fs = "ext4"` remains accepted as an explicit choice.
**Why:** Btrfs gives compression, cheap subvolumes and a snapshot-ready layout
without LVM. `/boot` stays ext4 because every loader and distro tool reads it
without special support, it is small, and keeping it unencrypted means the
loader never needs LUKS support. ext4 root stays available for hardware or
policy constraints, but it is not the default.
**Rejected:** ext4-only root (supersedes D-010), Btrfs `/boot`, ZFS, LVM,
encrypted `/boot`.

### D-028 — Snapshot tooling is deferred; the layout is snapshot-ready
**Date:** 2026-11-13
**Decision:** v0.1 creates `@snapshots` but installs no snapper, timeshift or
boot-into-snapshot menu, and takes no automatic snapshots on package
transactions. Users may snapshot manually with `btrfs subvolume snapshot`.
**Why:** Snapshot management is a product of its own: retention, restore UX,
boot entries and a recovery contract. The subvolume layout means a later card
can add it without reformatting or changing the plan schema.
**Rejected:** bundling a snapshot manager in v0.1, openSUSE snapper integration
before the storage layout is tested.
**Revisit:** after the required cells are green; O-06.

### D-029 — Deterministic partition, filesystem and LUKS UUIDs
**Date:** 2026-11-13
**Decision:** The plan derives GPT partition GUIDs and filesystem/LUKS UUIDs
with UUIDv5 from `(config digest, target disk serial, role)`, and passes them to
`sgdisk -u`, `mkfs.ext4 -U`, `mkfs.btrfs -U` and
`cryptsetup luksFormat --uuid`. The ESP has no filesystem UUID in the
`mkfs.ext4` sense, so it is identified by its deterministic GPT partition GUID
(`PARTUUID=`) in fstab, and its FAT volume ID is set with `mkfs.fat -i` so
`blkid` output is stable too. fstab, crypttab, the Limine config and the GRUB
cmdline are therefore complete at plan time.
**Why:** The plan is the API and must be byte-stable. Random UUIDs created at
apply time would force fstab and boot config to be runtime templates, which is
harder to test and to resume. Including the disk serial keeps two machines with
the same config distinct.
**Rejected:** random UUIDs plus runtime templating, rewriting fstab and boot
config from `blkid` probes during apply.
**Note:** cloning a disk keeps its UUIDs; re-plan on the new disk to regenerate
them.

### D-030 — First implementation preview and explicit execution gates
**Date:** 2026-11-13
**Decision:** The user requested an initial implementation across the restarted
project and publication to `BoreaProj/borea`. Preserve the local restart history
and remote initial commit; this is now the canonical target, superseding the
repository name in D-001. Work sequentially across the existing task cards on
one initial integration branch for this user-authorized pass. Later work returns
to D-023's one-card branches.
**Contract:** Schema 1 is the first actual JSON/TOML implementation, with the
implemented subset documented in IMPLEMENTATION.md. The architecture describes
the full target. No previously implemented public plan schema existed to migrate.
Future serialized changes must bump the schema and golden fixtures together.
**Safety:** Draft family/storage/boot commands are inspectable; live apply and
Omarchy removal remain unsupported until their validation and trust requirements
are implemented. File-only fixture execution is a separate explicit mode.
Missing source pins/repository keys refuse planning. Golden refusal cases are
not passing installations. The release matrix gate stays red.
**Dependencies:** Use only D-025-approved serde/JSON/TOML/SHA-256/UUID/zeroize and
tempfile. UUIDv5 enables uuid's SHA-1 implementation as a transitive dependency;
SHA-1 is used only for standardized UUID derivation, not plan integrity.
**Why:** Publish reviewable, executable groundwork without presenting untested
root commands as a working installer. No full milestone acceptance is waived.
**Rejected:** publishing a fake successful executor, invented fingerprints, or
marking package availability as a VM pass.

### D-031 — Borea is entirely independent and standalone
**Date:** 2026-11-13
**Decision:** Borea owns its installer workflow and runtime/tool provisioning.
It does not require a target distro's live/rescue environment or a
Borea-specific live environment. The selected family is the installation
target, not a required execution host. Supersedes D-003 and the corresponding
live-environment prerequisite in the plan, operations and agent guidance.
**Tooling:** Family-native bootstrap/package/initramfs tools may remain adapter
implementation details, but Borea must supply or provision the installation
runtime instead of assuming that the user booted media containing those tools.
No target package-manager state may be confused with the execution host's state.
**Delivery:** This correction does not add a bootable ISO, USB writer or custom
runtime initramfs requirement. These would be separate delivery decisions.
**Status:** G01 tracks runtime delivery. The preview still checks host PATH;
this documentation change does not claim external tools are bundled or that
standalone installs have passed. Existing apply gates remain in place.
**Why:** Explicit user correction: Borea is an entirely independent project and
installer, not dependent on a distro or its live environment.

### D-032 — Build Pages from the existing Markdown documentation
**Date:** 2026-11-13
**Decision:** Replace the added Astro deployment template with GitHub's
`actions/jekyll-build-pages@v1`, Primer theme, relative Markdown link handling
and optional front matter for the existing documents. Use the supported Pages
artifact upload/deploy actions. These are documentation build dependencies,
not installer runtime dependencies.
**Why:** The failed Astro workflow expects package.json/yarn.lock that do not
exist. The repository already has the documentation that Pages should publish;
no frontend application or npm dependency graph is needed. Rust CI stays intact.
**Validation:** Assert generated index, implementation and plan HTML before
upload. Pull requests build/check without deployment permissions; main publishes.
**Reference:** https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages

### D-033 — Arch Omarchy profile with Linutil baseline and manual app control
**Date:** 2026-11-13
**Decision:** Offer Omarchy as `profile.omarchy` inside the Arch family,
selected with config schema 2 and `session.type = "omarchy"`. Other families
refuse it. This supersedes the installation exclusion in D-015 at the user's
explicit request. Borea remains independent of target-family live media.
**Baseline:** Linutil commit `fda64ff50cd33f5cf12d5b68613f39ce6643e9d4` delegates
its Omarchy entry directly to the upstream installer; it does not define a
separate application-removal preset. Record this evidence and Omarchy commit
`f4378f0de5b44d331ee943746a97872b718a6c18` in one embedded metadata file with
entrypoint SHA-256 hashes and a package/webapp baseline. No scripts are copied
or executed by planning. These pins freeze reviewed inputs, not all upstream
package repositories or runtime network downloads.
**Manual control:** Preserve everything by default. Explicit `keep`, `remove`
and `replace` choices are validated for duplicates/conflicts; replacements are
installed before selected originals are removed. Unknown ids fail. Chromium
is retained for web apps; default browser is a separate explicit choice.
Removal intent names exact packages and uses no recursive/orphan/forced removal.
Keep-removed-after-update policies are opt-in and contain only selected removals.
**Upstream system defaults:** The profile uses upstream SDDM/Hyprland and requires
Btrfs/Limine at the pinned revision. Upstream Omarchy includes snapshot/boot and
system configuration; that is a profile-specific exception to D-028's plain-family
snapshot tooling exclusion. Borea adds no automatic Linutil system tweaks,
cleanup, dotfile replacements or application debloat.
**Schemas:** Config 2 adds the explicit Omarchy choices; basic schema 1 configs
remain readable, but Omarchy requires 2. Plan 2 adds typed `InstallOmarchy`,
Omarchy/package probes and a package-absence check. Schema 1 plans require
replanning. Recipe/matrix and existing defaults-policy JSON stay at schema 1.
Golden fixtures are updated in the same change, with two optional Omarchy cases;
the 29 required release VM cases are unchanged.
**Execution status:** The typed installer stage is intent, not a runnable
curl-to-shell command. User-context integration, upstream prompt/reboot handling,
repository trust, independent runtime and exact VM evidence are still required.
Live apply and mutation gates stay in place; no supported cell is claimed.
**References:** pinned files under `recipes/omarchy/upstream.toml`; no new dependency.

### D-034 — Independent family tool bundles with namespace isolation
**Date:** 2026-11-13
**Decision:** Build x86_64 root filesystem tool bundles from digest-pinned
family images using native signed package managers on CI builders. Publish new,
versioned gzip/tar assets and SHA-256 manifests; never overwrite pinned assets.
The runtime consumer uses reviewed SHA-256/size pins before extraction, supports
an offline cache, and executes tools through a read-only Bubblewrap filesystem
and private namespaces with transient log/cache directories. Cache extraction
normalizes owner access for root-only files and strips privileged metadata.
No host distro package manager or Docker daemon is
required for runtime consumption. Host kernel, curl/TLS and Bubblewrap are
explicit prerequisites; inability to isolate is a hard failure.
**Build-only tooling:** Docker and Python on GitHub-hosted runners. Native
package repository results are recorded in each resulting content-pinned bundle;
the build is not claimed reproducible solely from a mutable package mirror.
**New dependencies approved:** `tar` (default features disabled) and `flate2`
for bounded, path-confined extraction/verification of runtime archives;
`sha2` already approved for integrity. `curl` handles HTTPS transport without
adding an HTTP stack; Bubblewrap supplies maintained namespace isolation.
**Separation:** This delivers runtime tools; privileged disk execution, image
installation and VM acceptance remain separate cards. No host disks, home
folders, package databases or container-engine sockets are mounted by runtime
inspection/probe commands. Networking is disabled unless explicitly requested.
**References:** https://github.com/containers/bubblewrap and
https://docs.rs/tar/latest/tar/struct.Entry.html#method.unpack_in

---

## Open

Each open decision has a **default** used until it is closed, and a **trigger**
that forces a decision.

### O-01 — Debian Niri via source build?
**Default:** gate (D-011).
**Trigger:** a user needs Niri on Debian, or Debian packaging appears.
**Cost:** smithay/niri build chain, Rust toolchain in target, update story.

### O-02 — openSUSE MangoWM via source build?
**Default:** gate (D-012).
**Trigger:** upstream/OBS package, or a user request with a test machine.
**Cost:** wlroots 0.19/scenefx compatibility, C build deps.

### O-03 — Debian COSMIC?
**Default:** gate (D-013).
**Trigger:** Debian packaging or an official COSMIC apt repo.
**Cost:** large build, session/greeter integration.

### O-04 — Fedora end-4?
**Default:** gate (D-014).
**Trigger:** a maintained COPR with a pinned commit.
**Cost:** Quickshell/Hyprland version coupling.

### O-05 — Secure Boot
**Default:** off, documented.
**Trigger:** a user requires it on hardware with Secure Boot enforced.
**Cost:** key custody, MOK enrolment, signing, shim, revocation story. Limine
needs an enrolled config hash in addition to a signed EFI binary; GRUB needs a
signed shim path.

### O-06 — Snapshot tooling and rollback
**Default:** layout only; no tooling (D-028). `@snapshots` exists but is empty.
**Trigger:** after the required cells are green, or a user needs rollback.
**Cost:** retention policy, snapper/timeshift choice, boot-menu integration,
restore UX, per-family differences (openSUSE expects `@/.snapshots`).

### O-07 — Alongside / dual boot
**Default:** replace only (D-008).
**Trigger:** explicit product decision after v1.
**Cost:** free-space analysis, resize, boot-entry preservation, Windows testing.

### O-08 — Greeter for shell sessions
**Default:** greetd + tuigreet.
**Trigger:** first shell VM cell (D6).
**Alternatives:** `ly`, `sddm` with a Wayland session, autologin + compositor.
**Note:** decide by what starts the session reliably under QEMU and a laptop.

### O-09 — SSH in production images
**Default:** no SSH server unless a test profile adds it.
**Trigger:** remote verification needs it.
**Alternative:** verify offline from an independent execution host before first boot.

### O-10 — SELinux during Fedora bootstrap
**Default:** leave Fedora's default policy; relabel at first boot if needed.
**Trigger:** first Fedora VM cell fails to boot due to labels.
**Alternative:** `--setopt=tsflags=noscripts` is not acceptable; do not disable
SELinux permanently.

### O-11 — openSUSE snapshot pinning
**Default:** pin a Tumbleweed snapshot URL per release; refresh monthly.
**Trigger:** repository drift breaks a cell.
**Owner:** family adapter maintainer.

### O-12 — AUR policy
**Default:** AUR only for apps/shells whose matrix cell is `third-party` and
whose recipe pins a commit; built as an unprivileged user with `makepkg`.
**Trigger:** first AUR package in the required set (Helium, Vesktop are
optional).
**Alternative:** no AUR in v1; use Flatpak.

### O-13 — Third-party repo consent
**Default:** `plan` prints every third-party repo and its fingerprint; `apply`
requires `--yes`; `--deny-third-party` refuses them.
**Trigger:** first third-party cell (D6).
**Open:** whether `--yes` is enough or each repo needs an explicit flag.

### O-14 — sudo model
**Default:** the account user is in `wheel`/`sudo`; `sudo = "same"` reuses the
account password; `separate` requires a second secret slot. No passwordless sudo.
**Trigger:** first VM login test.
**Open:** whether root login is locked by default (proposed: yes).

### O-15 — Swap default
**Default:** zram, no swap partition.
**Trigger:** low-memory VM testing.
**Alternative:** swapfile sized to RAM/2 on encrypted roots.

### O-16 — Fedora release cadence
**Default:** read `release` from the family recipe, update it in a reviewed
change when Fedora branches.
**Trigger:** Fedora EOL or a new release.
**Open:** support N and N-1, or only current.

### O-17 — NVIDIA drivers
**Default:** open-source Nouveau only; no proprietary driver in v1.
**Trigger:** a user needs it; would be an app recipe with family-specific repos.
**Cost:** DKMS, Secure Boot, Wayland compatibility.

### O-18 — borea's own updates
**Default:** none. A new binary is downloaded manually.
**Trigger:** after v1.
**Rejected:** self-update daemon.

### O-19 — Recipe/seed licensing
**Default:** seeds authored in-repo are MIT; upstream configs retain their
licence and carry a notice in `recipes/files/<component>/NOTICE`.
**Trigger:** first copied upstream config (Noctalia/DMS/end-4).
**Open:** whether to ship upstream configs at all versus generating minimal
ones.

### O-20 — Telemetry / crash reports
**Default:** none. Ever.
**Trigger:** none.

### O-21 — Limine on openSUSE
**Default:** gate (D-026). `loader = "limine"` is refused on openSUSE.
**Trigger:** a VM cell proves Limine with Btrfs and versioned
`/boot/vmlinuz-*-default` kernels, and we decide it beats GRUB/snapper.
**Cost:** config regeneration on kernel updates, snapshot-menu story, an extra
boot cell per release.
**Alternative:** stay on GRUB; the package remains available for manual use.

---

## Rejected alternatives (kept for context)

| Alternative | Why rejected |
| --- | --- |
| Reuse the old architecture | live ISO + GUI + daemon is the failure mode |
| Custom OS image per family | four operating systems to maintain |
| One shell script per distro | untestable, no plan, no resume |
| Ansible/Chef for install | wrong layer, needs a running target |
| Nix/Guix | different model, not requested |
| Build every package from source | months, not days |
| Distro-specific installer wrappers | hides choices, breaks the matrix |
| GUI-first | slow, inaccessible to agents, huge surface |
| Simulation-first | proves nothing about boot |
| Varlink/worker | daemon lifecycle with no benefit |
| Pure Rust for disk/boot | reimplements mature tools |
| GRUB everywhere | worse than Limine on Arch; superseded by D-026 |
| Limine everywhere | no package on Debian/Fedora, extra integration risk on openSUSE |
| ext4-only root | leaves compression and subvolumes unused; superseded by D-027 |
| Btrfs `/boot` | no benefit for a small partition and more loader assumptions |
| Snapshot manager in v0.1 | retention/restore UX is its own product; see D-028 |
| Random UUIDs + runtime config templating | plan stops being byte-stable; see D-029 |

### D-035 — Closed image storage execution

G02 uses a separate strict image manifest (schema 1), never the draft command
plan. Images must be created by Borea as root-owned mode-0600 regular files.
The executor retains an exclusive image lock and configures auto-clearing loop
devices directly from that descriptor, including bounded partition slices.
Only these loops enter a foreground Bubblewrap mount namespace. The namespace
keeps the capabilities needed to mount, with no network, host filesystem or
physical disk access. It runs one reviewed fixed storage program, not a daemon
or generic worker. Native family tools still come from the pinned runtime.

A private root-owned temporary copy of the compressed runtime is rehashed
before extraction; a writable user cache is never elevated. `libc` 0.2 (already
transitive) is approved for Linux loop ioctls, locks, root checks and process
signals. `tempfile` is promoted to a runtime dependency for private staging.
Incomplete destructive steps fail closed if their exact UUID probe cannot
prove completion. Recreate the disposable image rather than guessing.
Encryption and target package/bootstrap/boot execution remain gated.
