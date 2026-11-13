# Operations runbook

> This document describes the full v0.1 target. Current executable behavior and
> remaining gates are listed in [IMPLEMENTATION.md](IMPLEMENTATION.md).


How to run a real install. This is the operator's view; the design lives in
[`ARCHITECTURE.md`](ARCHITECTURE.md).

> v0.1 is not released. Do not run this on a machine you care about until the
> matrix cell you need has a VM artifact and the release notes list it.

---

## 1. Before you start

You need:

- x86_64 machine, UEFI boot, Secure Boot **off**,
- one disk you can erase completely,
- wired or working Wi-Fi in the live environment,
- the family's live or rescue media,
- a `borea` binary matching the release, on a USB stick or reachable over the
  network,
- the account password and (if encrypting) the LUKS passphrase in files on the
  live system, mode `0600`.

Back up anything you need. `borea` does not detect or preserve other operating
systems in v1.

## 2. Per-family preparation

### Arch

Boot the Arch ISO. It provides `pacman`, `pacstrap`, `sgdisk`, `mkfs.*`,
`btrfs`, `cryptsetup`, `limine` and `grub-install`.

```sh
pacman -Sy --needed arch-install-scripts btrfs-progs limine
./borea doctor --family arch
```

For the GRUB fallback cell, add `grub`: `pacman -S --needed grub`. `doctor` only
asks for the loader the plan resolves.

### Debian

Boot Debian 13 live. `debootstrap` may not be installed:

```sh
sudo apt update && sudo apt install -y debootstrap
./borea doctor --family debian
```

### Fedora

Boot the Fedora Workstation/Everything live image. `dnf5` is the bootstrap
tool; `--installroot` works from the live environment.

```sh
./borea doctor --family fedora
```

### openSUSE

Boot openSUSE Tumbleweed rescue/live. `zypper --root` is the bootstrap tool.

```sh
./borea doctor --family suse
```

`doctor` prints every missing tool with the command to install it in that live
environment. Fix those before planning.

## 3. Write the config

```sh
cp /path/to/borea/examples/borea.toml ./borea.toml
$EDITOR borea.toml
```

Keep secrets out of the file. Point at files:

```toml
[account]
password_file = "/run/borea/user.pass"
```

```sh
install -m 600 /dev/null /run/borea/user.pass
printf '%s\n' 'the-password' > /run/borea/user.pass
```

For encryption, either reuse the account password (`encrypt = true` with no
separate slot) or add a `disk.luks` secret file as documented in the config
reference.

## 4. Plan and review

```sh
./borea plan --config borea.toml --out plan.json
./borea apply plan.json --dry-run
```

Read the summary:

- family and release,
- device path, serial, size,
- partition layout,
- every third-party repository and key fingerprint,
- the session and greeter,
- every gated component (should be none for a promised cell),
- the number of steps per phase.

`plan` does not touch the disk. If the plan is wrong, edit the config and
re-plan. Do not hand-edit `plan.json`: the digest will fail.

## 5. Apply

```sh
sudo ./borea apply plan.json --yes
```

The executor:

1. re-checks the device identity (serial + size),
2. partitions, formats and mounts (Btrfs subvolumes on the default lane; ext4
   `/boot` always),
3. bootstraps the base system,
4. configures locale, user, services,
5. installs the session,
6. installs and configures the bootloader (Limine on Arch, GRUB elsewhere) and
   the initramfs,
7. writes the journal under `/mnt/var/log/borea/` and on the live system.

If it fails, it stops, keeps mounts and prints:

```text
failed: session.hyprland.packages (exit 1)
journal: /var/log/borea/journal.jsonl
next:    borea apply plan.json --resume
```

Do not reboot a half-installed target. Read the journal tail, fix the cause
(network, mirror, package name), then resume.

## 6. Verify before reboot

```sh
sudo ./borea verify plan.json --root /mnt
```

This checks files, packages, units, users, session entries, the filesystem
layout (`findmnt`, subvolumes, mount options) and the bootloader (EFI binary,
config and cmdline) offline. Fix any failure before rebooting.

## 7. First boot

Unmount and reboot:

```sh
sudo umount -R /mnt
sudo reboot
```

Remove the live media. The target should reach the greeter (desktop/shell) or a
login prompt (minimal). Then, if SSH is enabled in the profile:

```sh
borea verify plan.json --ssh user@<target>
```

Production profiles do not enable SSH. Use the console for the first login.

## 8. After install

- Change the password if the temporary one was reused.
- Enable full-disk encryption unlock at boot (already configured).
- Add your own dotfiles; borea only seeds a minimal working session.
- The root is Btrfs. Inspect it with `btrfs subvolume list /` and
  `btrfs filesystem usage /`. `@snapshots` is empty by design; v0.1 does not
  install snapshot tooling (D-028). A manual snapshot is:
  `sudo btrfs subvolume snapshot -r / /.snapshots/root-$(date +%F)`. Do not
  snapshot `/` while it is being written heavily, and do not expect a boot-menu
  entry: limine/GRUB menus are not snapshot-aware in v0.1.
- For Omarchy, see [`OMARCHY.md`](OMARCHY.md).

## 9. Recovery

| Symptom | Likely cause | Action |
| --- | --- | --- |
| Limine prompt/menu but no boot | wrong `guid(...)` or kernel path | boot live, `blkid` the `/boot` partition, compare with `/boot/efi/EFI/BOOT/limine.conf`, re-run the Limine steps |
| GRUB prompt, no menu | `grub.cfg` not generated or wrong path | boot live, `borea verify --root /mnt`, re-run the boot steps |
| Kernel panic, no root | wrong `root=`/UUID, missing `rootflags=subvol=@`, or missing initramfs | check `fstab`, `crypttab`, cmdline and initramfs command in the journal |
| `mount: unknown filesystem type 'btrfs'` | initramfs lacks the Btrfs module | chroot, add `btrfs` to the initramfs config, regenerate, re-verify |
| LUKS prompt loops | wrong key slot or crypttab UUID | open manually with `cryptsetup open`, compare UUIDs |
| Greeter but no session | session `.desktop` missing or Exec wrong | check `/usr/share/wayland-sessions/`, journal |
| Shell starts then exits | compositor/shell version mismatch | check the pair in `MATRIX.md`; use a supported pair |
| Network missing | NetworkManager not enabled | `systemctl enable --now NetworkManager` in a chroot |

To mount a Btrfs target manually from the live environment:

```sh
cryptsetup open /dev/<root-part> cryptroot      # only when encrypt = true
mount -o subvol=@,compress=zstd:3,noatime /dev/mapper/cryptroot /mnt
mount -o subvol=@home /dev/mapper/cryptroot /mnt/home
mount /dev/<boot-part> /mnt/boot
mount /dev/<esp-part> /mnt/boot/efi
```

`borea apply --resume` is safe after a failure. It skips completed steps and
probes the current state. If the disk was changed outside borea, re-plan; do not
resume.

## 10. Cleanup / abort

```sh
sudo ./borea apply plan.json --cleanup   # unmount in reverse order
```

`--cleanup` never formats or deletes. It only unmounts what the plan mounted.
To start over, re-plan and apply from the storage phase; the layout probe will
detect the existing table and skip partitioning unless the plan says otherwise.

## 11. Reporting a failure

Include:

- `result.json` / `journal.jsonl` / `serial.log` from the VM run, or the journal
  path from a real run,
- the exact config (redact secrets and serial numbers),
- the family media and checksum,
- the failing step id and exit code,
- whether `--resume` changed anything.

Do not include passwords, keys or full disk serials in a public issue.
