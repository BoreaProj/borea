# borea build plan

Status: **proposed, ready to execute**. Owner: whoever picks up `D0`.
Last grounded: 2026-11-13.

This document is the contract for v0.1. If work is not described here or in a
task card in [`TASKS.md`](TASKS.md), it does not belong in the repository.

---

## 1. The problem

Installing a modern Linux desktop is either a distro-specific installer with
fixed choices, or a pile of post-install scripts that only work on one family.
The previous `borea` tried to solve this by building its own operating system:
a live image, a Rust initramfs, a GUI, a USB writer and a daemon. None of that
was needed, and none of it produced a working install.

borea v2 takes the opposite position:

> Use the environment the distro already ships, drive its own tools, express
> every choice as a plan, and prove each combination in a VM.

## 2. Goals

1. **Minimal, reproducible installs** on Arch, Debian, Fedora and openSUSE from
   the family's live or rescue environment.
2. **One session layer** on top: KDE, GNOME, COSMIC, or Hyprland / Niri /
   MangoWM with Noctalia / DUX (DMS) / end-4.
3. **Customisable defaults** through TOML recipes and a user config, without
   forking code per distro.
4. **Omarchy trim and default management** on existing Omarchy installs,
   preserving the behaviour already designed in the old repository.
5. **Agent-executable**: every task has a file boundary, a verification command
   and a VM acceptance cell. Claude and Codex can work in parallel without
   stepping on each other.
6. **Honest coverage**: a support matrix with evidence dates. Unsupported
   combinations fail with a reason, not a hidden button.

## 3. Non-goals (v1)

| Not doing | Why |
| --- | --- |
| GUI, TUI dashboard, animations, recorder | Text + JSON is enough; the old GUI consumed the project |
| Live ISO, initramfs builder, USB writer, boot menu | Each distro already ships a live/rescue environment; the target builds its own initramfs |
| Daemon, socket, worker, Varlink, D-Bus service | One process, one plan, one journal |
| Simulation engine | `--dry-run`, golden plans, VM matrix |
| Dual-boot / alongside / resize | Large, dangerous, not required for v1 |
| Secure Boot signing and key enrolment | Needs a trust story we do not have |
| Btrfs, snapshots, rollback | ext4 first; add only after v1 is green |
| ARM, RISC-V, BIOS/legacy | x86_64 UEFI only |
| Wi-Fi setup in the installer | Use the live environment's network tooling |
| Installing Omarchy itself | Omarchy has its own ISO; v1 trims/manages an existing install |
| Immutable distros, NixOS, Alpine (OpenRC) | systemd + package-manager model only |
| A plugin system or scripting language | Recipes are declarative TOML, not code |

## 4. Product surface

One binary, `borea`, plus one small on-target binary, `borea-defaults`.

```text
borea doctor   [--family F] [--json]        host readiness
borea catalogue [--family F] [--json]       what the matrix offers
borea explain  <component> [--family F]     why a cell is gated
borea plan     --config C --out P [--json]  resolve, validate, emit
borea apply    P [--dry-run] [--yes] [--resume] [--journal J]
borea verify   P (--root DIR | --ssh TARGET) [--json]
borea status   [--journal J]                last run summary
borea omarchy  trim|defaults ...            existing Omarchy installs
```

Exit codes: `0` ok, `2` usage, `3` preflight/unsupported, `4` apply failed,
`5` verification failed.

The config surface is one file, `borea.toml`:

```toml
schema = 1

[target]
family = "arch"              # arch | debian | fedora | suse
release = "2026.11.01"       # family default when omitted
hostname = "borea"
timezone = "Europe/London"
locale = "en_GB.UTF-8"
keymap = "uk"

[disk]
device = "/dev/vda"
mode = "replace"             # only "replace" in v1
root_fs = "ext4"             # ext4 only in v1
encrypt = false
# luks_password_file = "/run/borea/luks.pass"   # required when encrypt and != account password
esp_size = "1GiB"
boot_size = "2GiB"
swap = "zram"                # none | zram | file

[boot]
loader = "grub"
secure_boot = false

[account]
username = "luke"
full_name = "Luke Shaddick"
password_file = "/run/borea/user.pass"
sudo = "same"                # same | separate
# sudo_password_file = "/run/borea/sudo.pass"
# ssh_authorized_key_file = "/run/borea/test.pub"   # enables sshd only when set

[session]
type = "shell"               # minimal | desktop | shell
compositor = "hyprland"      # hyprland | niri | mangowm
shell = "noctalia"           # noctalia | dms | end4 | none
greeter = "auto"             # auto | sddm | gdm | greetd | none

[apps]
enable = ["firefox"]
codecs = true
flatpak = true

[omarchy]
trim = []
keep_removed_after_updates = true
default_browser = "chromium"
```

For `[session] type = "desktop"`, set `desktop = "kde" | "gnome" | "cosmic"`
and omit `compositor`/`shell`.

## 5. Architecture in one page

```text
                    recipes/*.toml        recipes/matrix.toml
                          │                      │
   borea.toml ── loader ──┴── resolver ──────────┘
                              │
                        plan.json  (steps + checks, deterministic)
                              │
                    ┌─────────┴─────────┐
                 executor            verifier
                    │                    │
        b o r e a - d i s t r o     offline / SSH checks
        arch | debian | fedora | suse
                    │
        distro tools: pacstrap, debootstrap,
        dnf5 --installroot, zypper --root,
        sgdisk, mkfs, cryptsetup, mount,
        grub-install, chroot
```

- `borea-core` owns config, recipes, matrix, plan, checks and validation.
- `borea-distro` turns abstract package/repo/service requests into family
  commands. It is the only crate allowed to branch on family.
- `borea-session` builds the session portion of the plan (desktop, compositor,
  shell, greeter, apps) from recipes, using the distro adapter interface.
- `borea-run` executes steps, probes idempotency, journals, resumes, redacts.
- `borea-defaults` is shipped to the target; it enforces trim policy after
  distro updates.

Full details: [`ARCHITECTURE.md`](ARCHITECTURE.md). Recipe schema:
[`RECIPES.md`](RECIPES.md). Capability table: [`MATRIX.md`](MATRIX.md).

## 6. Execution model

1. **Preflight** (read-only): UEFI, x86_64, root, network, family host tools,
   target disk identity (path, serial, size, mounted state), free space.
2. **Resolve**: config + recipes + matrix → ordered steps. Unsupported cells
   fail here with `E_UNSUPPORTED` and the gate reason.
3. **Confirm**: print summary and the exact device. `--yes` is required for
   destructive steps. Plan identity (serial + size) is re-checked at apply.
4. **Apply**: storage prefix (partition, format, mount) → bootstrap → base
   config → session → bootloader → finalise. Every step is journaled before and
   after execution. A failure stops the run, keeps mounts, and prints the resume
   command.
5. **Verify**: offline checks against the mounted/closed target, then a boot
   check in the VM harness and online checks over SSH.
6. **Report**: `borea status` summarises the journal and any failed checks.

Steps are JSON objects. Destructive steps must appear before the first
`bootstrap` step. The executor refuses a plan where they do not.

## 7. Coverage contract

Tier definitions:

| Tier | Meaning |
| --- | --- |
| `native` | packaged in the family's official repositories |
| `third-party` | documented repository/COPR/OBS/APT repo with a pinned key |
| `source` | built in the target from a pinned upstream commit |
| `unsupported` | no documented path; `plan` refuses |

v0.1 **must** pass, on all four families, in a VM:

- minimal
- KDE
- GNOME
- Hyprland (compositor only)
- Hyprland + Noctalia
- Hyprland + DUX/DMS

v0.1 **may** pass (gated in `MATRIX.md`, not promised):

- Niri on Debian
- MangoWM on openSUSE
- COSMIC on Debian
- end-4 on Arch as a `source` cell (experimental, `--allow-source`)

Every other cell must be explicitly `unsupported` with a reason. The rule is:
a green row in `MATRIX.md` requires a passing VM artifact path in the release
notes. No artifact, no green.

## 8. Milestones

Days are focused engineering days for one agent. With two or three agents on
parallel workstreams, calendar time compresses to roughly half. The critical
path is **D0 → D1 → D2 → D3 → D5 → D6 → D8**; Omarchy (E) and sessions can run
in parallel once the core API freezes at the end of D1.

### D0 — Skeleton, schema, matrix (critical)

- Workspace, CI, formatting/clippy/test gates.
- `borea.toml` schema, plan schema v1, recipe schema v1.
- Matrix file with every cell and its tier + evidence date.
- `doctor`, `catalogue`, `plan --dry-run` against fixtures.
- Golden plan fixture for one minimal Arch config.

**Exit:** `cargo test` green; `cargo xtask matrix check` reports no unknown
cells; `borea plan` emits the golden JSON byte-for-byte.

### D1 — Executor, storage, bootloader plan (critical)

- Journal, probes, resume, secret slots, redaction.
- GPT layout, ESP, `/boot`, root, ext4, optional LUKS2, mount tree, fstab.
- GRUB step set for all four families (paths differ; adapter owns them).
- Loopback test: create a 20 GiB image, partition it, format it, mount it,
  write fstab, unmount it. Re-apply is a no-op.

**Exit:** `cargo xtask disk selftest` passes twice; second run skips every
storage step.

### D2 — Arch minimal end to end (critical)

- `pacstrap` adapter, base packages, locale/timezone/user/services.
- Container runner + QEMU boot + SSH verification.
- First real VM: boots, logs in, `borea verify` passes.

**Exit:** `cargo xtask e2e --family arch --profile minimal` green, artifact
kept.

### D3 — Debian and Fedora minimal (critical)

- `debootstrap`, `dnf5 --installroot`, per-family fstab/initramfs/boot paths.
- Same e2e cell for both.

**Exit:** both cells green. Any family-specific divergence documented in the
adapter, not leaked into core.

### D4 — openSUSE, encryption, resume

- `zypper --root`, openSUSE boot paths.
- LUKS2 e2e on Arch and Debian; document Fedora/openSUSE if they lag.
- Kill-and-resume test mid-bootstrap.

**Exit:** openSUSE minimal green; at least two encryption cells green; resume
from a killed apply completes without repeating destructive steps.

### D5 — Desktops (critical)

- KDE and GNOME on all four families.
- SDDM/GDM selection and session entries.
- Apps, Flatpak, codecs.

**Exit:** 8 cells green, each with a boot + login check.

### D6 — Compositors and shells (critical)

- Hyprland on all four, greetd, Noctalia and DMS on all four.
- Niri where native; MangoWM where native/third-party.
- Gates for the rest with reasons.

**Exit:** 12 required cells green (3 per family); gates return
`E_UNSUPPORTED` in a unit test.

### D7 — COSMIC, end-4, Omarchy tools

- COSMIC where a path exists; end-4 on Arch; gates elsewhere.
- Port trim catalogue and default manager; fixtures; Omarchy VM or fake root.
- `borea omarchy` CLI.

**Exit:** Omarchy trim/defaults tests pass against a fixture root; COSMIC and
end-4 cells either green or gated with evidence.

### D8 — Matrix, docs, release

- Full matrix run; artifacts indexed.
- README, MATRIX and release notes reflect only green cells.
- Tag `v0.1.0`; publish static binaries (musl) for x86_64.

**Exit:** every promised cell has an artifact; `cargo xtask matrix verify`
refuses to mark a cell green without one.

## 9. Workstreams and file ownership

Parallel agents must stay inside their column. Cross-column changes go through
the owner or a small, reviewed interface PR.

| WS | Owns | May not touch |
| --- | --- | --- |
| A Core | `crates/borea-core`, `crates/borea-cli`, schemas | distro commands, recipes |
| B Storage | storage/boot modules in `borea-distro`, `borea-run` probes | session recipes |
| C Families | `crates/borea-distro` per-family modules, `recipes/families` | plan schema |
| D Sessions | `crates/borea-session`, `recipes/{desktops,compositors,shells,apps,profiles}` | family command syntax |
| E Omarchy | `crates/borea-defaults`, `crates/borea-core/src/omarchy`, `docs/OMARCHY.md` | installer path |
| F Harness | `crates/xtask`, `fixtures`, `.github/workflows`, `docs/TESTING.md` | product code |

Interfaces freeze at end of D1: `Config`, `Plan`, `Step`, `Check`, `Recipe`,
`Family`, `Session`. After that, additive changes only; breaking changes need a
decision entry and a schema bump.

## 10. Acceptance criteria

A release is acceptable when all of these are true:

1. The six required cells per family pass in a VM from a clean disk, using the
   family's own bootstrap tool, and the installed system boots to a login
   prompt or greeter.
2. `borea verify` passes over SSH after the first boot for every green cell.
3. Re-running `apply` on a completed plan is a no-op; re-running after a killed
   apply resumes correctly.
4. Every gated cell returns a reason, a remedy and a link to the evidence.
5. No secret appears in any plan, journal, log or artifact (test with a canary).
6. The Omarchy trim/default flow passes against a fixture root and (if a VM is
   available) a real Omarchy VM.
7. CI is green: fmt, clippy `-D warnings`, tests, matrix check, golden plans.
8. Docs match reality; nothing in README claims an untested cell.

## 11. Risks and mitigations

| Risk | Likelihood | Mitigation |
| --- | --- | --- |
| Family bootstrap differs more than expected | high | D2 proves Arch first; D3 timeboxed per family; divergence stays in adapter |
| Third-party repos change or break keys | high | pin key fingerprints; `matrix check` validates recipes; fail closed |
| Shells need compositor versions the distro lacks | medium | tier `source` or gate; never silently install a mismatched pair |
| VM automation eats the schedule | medium | build harness on D0–D1, not D7; container lane for fast iteration |
| Encryption + GRUB + initramfs differs per family | medium | treat as separate acceptance cells, not a checkbox |
| Agent drift / scope creep | high | one card at a time, file ownership, "no new crate without decision" |
| Testing on the host damages a real disk | low | image files, loop devices, `--yes` + identity check |
| Package names drift before release | medium | recipes carry evidence date + upstream URL; nightly matrix check |

## 12. Definition of done

- Six required cells × four families green, artifacts linked.
- `borea` builds a static x86_64 binary; `borea-defaults` builds for the target.
- Docs (`PLAN`, `ARCHITECTURE`, `RECIPES`, `MATRIX`, `TASKS`, `TESTING`,
  `DECISIONS`, `OMARCHY`, `PROMPTS`) are current.
- All open decisions are either closed or have a named trigger and owner.
- The tree is clean, the tag exists, and the release notes list exactly what
  works and what is gated.

## 13. What "done in days" means

The plan is deliberately narrow. The only moving parts are:

- a schema and resolver,
- four bootstrap adapters,
- one storage/boot path,
- a handful of session recipes,
- a VM loop.

There is no image to build, no GUI to render, no daemon to supervise, and no
new operating system to maintain. If a task cannot be verified by a plan
fixture or a VM cell, it is not in v1.
