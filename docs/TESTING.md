# Testing

> This document describes the full v0.1 target. Current executable behavior and
> remaining gates are listed in [IMPLEMENTATION.md](IMPLEMENTATION.md).


The installer is only as good as its last VM run. Unit tests protect the plan;
the e2e harness proves the plan installs and boots. Neither alone is enough.

---

## 1. Test layers

| Layer | What it proves | Runtime | Where |
| --- | --- | --- | --- |
| Unit | parsing, validation, digest, gates, argv construction | ms | `cargo test` |
| Golden plans | the plan for a fixture config is byte-stable | ms | `cargo xtask plan golden` |
| Fixture roots | file/seed/check logic against a fake target tree | ms | `cargo test` |
| Disk selftest | partition/format/mount/LUKS on a loop image | seconds | `cargo xtask disk selftest` |
| Family env | `borea` runs inside each family's toolchain | minutes | `cargo xtask env` |
| VM cell | the installed disk boots and verifies | minutes | `cargo xtask e2e` |
| Independent runtime lane | Borea can install a target without its distro live media or a Borea live environment | tens of minutes | manual/nightly |

The rule: a behaviour change needs a unit or fixture test. A matrix promotion
needs a VM artifact. A bootloader, kernel or encryption change needs both.

## 2. Unit and fixture tests

- `borea-core`: config validation, recipe lint, matrix gates, resolver order,
  canonical JSON, digest stability, secret redaction.
- `borea-distro`: argv for each family, fstab/crypttab rendering (UUID and
  PARTUUID), bootloader policy and paths (Limine on Arch, GRUB elsewhere),
  Btrfs subvolume steps, key fingerprint checks.
- `borea-session`: component merge, greeter selection, seed destinations,
  duplicate detection.
- `borea-run`: probe evaluation, journal replay, resume, redaction, chroot
  bind/unmount.
- `borea-defaults`: policy state machine, fake package manager, user reinstall,
  blocked removal.

Fakes live in `fixtures/`:

```text
fixtures/
  config/            valid and invalid borea.toml files
  plans/             golden plan JSON
  roots/             fake target roots (etc, var, usr, home)
  pkg/               fake pacman/apt/dnf/zypper scripts
  sysfs/             device identity fixtures
  omarchy/           fake Omarchy install
  images/            tiny prebuilt disk images for vm boot tests
```

Fake package managers are shell scripts that read a JSON state file, mutate it,
and exit with configurable codes. They are only used in tests; the real
adapters call the real tools.

## 3. Golden plans

```sh
cargo xtask plan golden          # compare
cargo xtask plan golden --update # rewrite after a reviewed schema change
```

Properties enforced:

- identical input → identical bytes, including map order and step ids;
- `id` and `created` are ignored by the digest;
- changing any config field that affects installation changes the digest;
- changing only `created` does not;
- every plan validates against `schema` before comparison.

Golden files: one per family for minimal (default Btrfs + resolved loader),
plus one per required session pair, plus the four ext4-root variants and the
Arch-GRUB variant. Keep them small; do not commit a plan with absolute host
paths.

## 4. Disk selftest

Runs on a sparse raw image, never a block device.

```sh
cargo xtask disk selftest --size 20G --stage all --fs btrfs
cargo xtask disk selftest --size 20G --stage all --fs ext4
cargo xtask disk selftest --encrypt
```

It asserts:

1. GPT has exactly three partitions with the expected sizes and type GUIDs;
   GUIDs are stable across a re-plan and differ when the disk serial changes.
2. ESP is FAT32, `/boot` is ext4 on both lanes, `/` is Btrfs on the default lane
   and ext4 on the fallback lane; labels are correct.
3. Btrfs has `@`, `@home`, `@snapshots`, `@log` and `@cache`; mount options are
   `compress=zstd:3,noatime,ssd,discard=async`.
4. Mount tree is root (subvol=@) → boot → ESP; fstab uses `UUID=` for
   ext4/Btrfs/LUKS and `PARTUUID=` for the ESP, plus `subvol=` and the mount
   options.
5. The kernel cmdline carries `rootflags=subvol=@` on Btrfs and no subvol flag
   on ext4.
6. LUKS2 opens with the canary key and closes cleanly; the LUKS UUID matches the
   plan.
7. A second run skips every step (journal shows `skipped`).
8. Cleanup detaches the loop device even after a panic.

The harness refuses to run if `--image` resolves to a device or if the image is
already attached.

## 5. Family environments

Borea runs independently of the target distro. For fast adapter iteration,
family containers provide native tools as test fixtures; they are not a product
requirement to boot or run the target family:

| Family | Container base | Tools |
| --- | --- | --- |
| Arch | `archlinux:latest` | `arch-install-scripts`, `pacman`, `sgdisk`, `btrfs-progs`, `e2fsprogs`, `dosfstools`, `cryptsetup`, `limine`, `grub` |
| Debian | `debian:trixie` | `debootstrap`, `apt`, `sgdisk`, `btrfs-progs`, `e2fsprogs`, `dosfstools`, `cryptsetup`, `grub-efi-amd64` |
| Fedora | `fedora:latest` | `dnf5`, `sgdisk`, `btrfs-progs`, `e2fsprogs`, `dosfstools`, `cryptsetup`, `grub2-tools` |
| openSUSE | `opensuse/tumbleweed` | `zypper`, `sgdisk`, `btrfs-progs`, `e2fsprogs`, `dosfstools`, `cryptsetup`, `grub2` |

```sh
cargo xtask env --family debian -- borea doctor --json
cargo xtask env --family debian -- borea plan --config /work/fixtures/config/debian-minimal.toml --out /tmp/plan.json
```

The container is privileged only for loop devices and mounts. It mounts:

- the workspace at `/work` (read-only),
- the disk image at `/disk` (read-write),
- a scratch directory at `/scratch`.

Limits: family containers alone do not prove standalone runtime delivery,
udev, firmware or network setup. The independent runtime lane in §8 must
validate those requirements on a host separate from the selected target family.

## 6. VM e2e

```sh
cargo xtask e2e --family arch --profile minimal
cargo xtask e2e --family arch --profile minimal --loader grub
cargo xtask e2e --family arch --profile minimal --root-fs ext4
cargo xtask e2e --family debian --profile hyprland-noctalia --keep
cargo xtask e2e --family suse --profile kde --encrypt
```

`--loader` accepts `auto`, `limine` or `grub`; the harness refuses a loader the
family matrix gates. `--root-fs` accepts `btrfs` or `ext4`. Both flags override
the profile for one run and are recorded in `result.json`.

Pipeline:

```text
1. create sparse raw disk (default 20 GiB) under target/e2e/<cell>/<ts>/
2. attach loop device
3. run `borea plan` in the family container       -> plan.json
4. run `borea apply` in the family container      -> journal.jsonl
5. detach loop
6. boot the disk in QEMU with OVMF + test SSH key -> serial.log
7. wait for SSH, run `borea verify --ssh`         -> verify.json
8. write result.json {cell, commit, pass, artifacts, duration}
9. detach, keep or delete artifacts
```

QEMU invocation (sketch):

```sh
qemu-system-x86_64 \
  -machine q35,accel=kvm:tcg -m 4096 -smp 2 \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=<copy of OVMF_VARS.fd> \
  -drive file=<disk>.img,format=raw,if=virtio \
  -netdev user,id=n0,hostfwd=tcp::2222-:22 -device virtio-net-pci,netdev=n0 \
  -nographic -serial file:<cell>/serial.log
```

The test SSH key is injected by the plan in test profiles only
(`[account] ssh_authorized_key_file`). Production profiles do not enable SSH.

### Pass criteria

- `apply` exits 0 and the journal has no failed step.
- The VM reaches SSH within the timeout (default 180 s after boot).
- `borea verify` exits 0 with every check `ok`.
- The bootloader check passes: the EFI binary and config exist, the config names
  the right kernel/initramfs, and the booted `/proc/cmdline` matches the plan.
- `/proc/cmdline` contains `rootflags=subvol=@` and the expected root UUID on
  Btrfs; `findmnt /` shows `subvol=/@` and the compression option after systemd
  remounts the root.
- For desktop/shell cells, the session check passes: the greeter unit is
  enabled, the session `.desktop` file exists, and the shell/compositor package
  is installed. A headless check cannot prove pixels; a human spot-check is
  recorded once per release.
- Serial log has no kernel panic, no `Failed to start` for the target unit, and
  no emergency shell.

### Artifacts

```text
target/e2e/<family>/<profile>/<timestamp>/
  result.json
  plan.json
  journal.jsonl
  verify.json
  serial.log
  qemu.cmd
  disk.img            (only with --keep)
```

CI uploads the whole directory on failure and `result.json` on success.
`cargo xtask matrix verify --strict` scans artifacts and refuses to mark a cell
green without a `pass: true` result newer than the last recipe change.

## 7. What each cell actually asserts

| Profile | Extra assertions beyond minimal |
| --- | --- |
| `minimal` | boot, root login, network unit enabled, no failed units, Btrfs subvolumes |
| `minimal-ext4` | same, but `/` is ext4 and the cmdline has no `rootflags` |
| `minimal-grub` | same, but GRUB is the resolved loader (Arch fallback) |
| `kde` | `sddm` enabled, `plasmawayland.desktop` exists, `plasma-workspace` installed |
| `gnome` | `gdm` enabled, `gnome.desktop` exists, `gnome-shell` installed |
| `cosmic` | `cosmic-greeter`/session file, `cosmic-session` installed |
| `hyprland` | `hyprland` installed, `hyprland.desktop` exists, greetd enabled |
| `hyprland-noctalia` | above + `noctalia` package + shell autostart seed |
| `hyprland-dms` | above + `dms` package + quickshell present |
| `niri` | `niri` installed, `niri.desktop` exists |
| `mangowm` | `mangowm` installed, session file exists |
| `end4` | pinned commit in plan, shell files in home, compositor starts |
| `omarchy-trim` | fake root only; live VM optional |

A cell that cannot assert its session is not green. "Package installed" alone is
not enough for a shell: the session entry and autostart must be present.

## 8. Independent runtime lane

Once per release, use Borea's independent runtime to install at least one cell
for each target family. The execution host must not be required to match the
target family, and the test must not require distro live/rescue media or a
Borea-specific live environment.

Record host details, Borea/runtime versions and checksums, target release,
UEFI mode, exact commands and VM artifacts in `docs/releases/<version>.md`.
Prove that Borea supplies/provisions its required tools, keeps target package
manager state separate from the host, and fails before disk mutation when a
runtime dependency cannot be verified. G01 defines the implementation work;
the runtime tool smoke lane is implemented; full VM installation acceptance
remains pending. See [RUNTIME.md](RUNTIME.md).

## 9. CI

Pull requests:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo xtask matrix check
cargo xtask recipe lint
cargo xtask plan golden
cargo xtask secret canary
```

Nightly (or manual):

- disk selftest, Btrfs and ext4, unencrypted and encrypted;
- one family per night on a rotating schedule, all four by the end of the week;
- the 24 session cells plus the five storage/boot cells on release candidates;
- artifacts uploaded and indexed.

A red nightly does not block unrelated PRs, but it blocks a release. The
release checklist reads the latest artifact index, not memory.

## 10. Debugging a failed cell

1. Read `result.json` — it names the failing phase.
2. `journal.jsonl` — the last `start` without `ok` is the step. The captured
   tail has the command output.
3. `serial.log` — boot failures, missing initramfs, wrong root, failed units.
   For a Btrfs root, check that the cmdline has `rootflags=subvol=@`; for
   Limine, check that the config's `guid(...)` matches the `/boot` UUID.
4. Re-run with `--keep` and inspect the disk:
   `cargo xtask vm shell --image <cell>/disk.img` (boots a rescue shell) or
   attach the image read-only on the host. Mount Btrfs with
   `mount -o subvol=@ <img> /mnt` (use a loop with `--partscan`).
5. Reproduce the plan only:
   `cargo xtask env --family <f> -- borea plan --config <cell>/config.toml --out /tmp/p.json`
6. If the failure is family-specific, fix the adapter, not the core. If it is
   session-specific, fix the recipe/seed.

Never "fix" a red cell by removing its check or marking it `unsupported`
without a decision entry and a reason.

## 11. Safety rules for tests

- Only image files and loop devices. `xtask` asserts the target is a regular
  file before attaching.
- No test downloads an ISO by default; cached media is checksum-pinned in
  `xtask/media.toml`.
- No test writes to `$HOME`, `/etc` or a real `/boot`.
- The canary password is `canary-not-a-real-password`; CI greps every artifact.
- Tests that need root say so and skip with a clear message otherwise; they do
  not silently pass.

## 12. Documentation deployment

`.github/workflows/pages.yml` builds the README entry page and Markdown docs
using GitHub's Jekyll Pages action. `_config.yml` supplies the theme and relative
link conversion; no package.json, npm or Astro build is involved. Source and
build artifacts are excluded from publication. The workflow checks generated
index, implementation and plan pages before uploading a Pages artifact.

Pull requests build documentation without deploying. Main and manual runs
publish through the `github-pages` environment. Confirm both build/deploy jobs
and the separate Rust CI; a successful Rust run alone does not verify Pages.

## 13. Runtime delivery smoke tests

Run the offline/unit checks in normal CI, including `cargo xtask runtime check`.
The real runtime smoke lane uses `runtime/smoke.py` on an x86_64 Linux host with
Bubblewrap, the release binary and four cached bundles. It executes only help/
version and isolation probes; it mounts no target disks and counts no VM cells.
Results and build-run links are recorded in [validation/runtime.md](validation/runtime.md).
