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
**Decision:** No live ISO, initramfs, USB writer or boot menu. The user boots
the family's media (or a prepared environment) and runs `borea`.
**Why:** Every family already ships a tested environment with its bootstrap
tool. Rebuilding that is a second operating system.
**Rejected:** custom live image, Buildroot, embedded installer.

### D-004 — Use the distro's native tools; drop the pure-Rust constraint
**Decision:** `pacstrap`, `debootstrap`, `dnf5 --installroot`, `zypper --root`,
`sgdisk`, `mkfs.*`, `cryptsetup`, `mount`, `grub-install`, `chroot`.
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
**Decision:** One bootloader adapter, GRUB, with family-specific package names
and config paths.
**Why:** Four bootloaders is four times the failure surface. GRUB works on all
four and reads an unencrypted `/boot`.
**Rejected:** systemd-boot, BLS, per-family defaults, rEFInd.

### D-010 — ext4, unencrypted `/boot`, optional LUKS2 root
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
**Why:** The binary must run in any of the four live environments without
matching glibc.
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
**Cost:** key custody, MOK enrolment, signing, shim, revocation story.

### O-06 — Btrfs and snapshots
**Default:** ext4.
**Trigger:** after the 24 required cells are green.
**Cost:** subvolumes, mount options, snapshot tooling, GRUB integration.

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
**Alternative:** verify offline from the live environment before first boot.

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
