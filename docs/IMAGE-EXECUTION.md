# Disposable-image storage execution

G02 executes a closed storage program against a newly created raw image. It
creates GPT, a 1 GiB FAT32 ESP, a 2 GiB ext4 boot partition and an ext4 or Btrfs
root. Btrfs has `@`, `@home`, `@snapshots`, `@log` and `@cache`, with mount entries
in `/etc/fstab`. These are storage fixtures, not bootable installations.

Fetch the runtime without root, then execute on a disposable Linux test host
with root and Bubblewrap. The target family selects the verified tool bundle;
it does not require the host's package manager. No host `losetup`, formatter or
mount executable is used.

```sh
borea runtime fetch --family arch --cache /absolute/runtime-cache
sudo borea image create --family arch --image /absolute/test.img \
  --manifest /absolute/test.image.json --fs btrfs --size-gib 20
sudo borea image apply --family arch --manifest /absolute/test.image.json \
  --cache /absolute/runtime-cache --journal /absolute/test.journal --yes
sudo borea image apply --family arch --manifest /absolute/test.image.json \
  --cache /absolute/runtime-cache --journal /absolute/test.journal --yes --resume
```

Image and manifest creation never overwrite an existing file. Images remain
root-owned mode 0600: transferring ownership back and forth cannot revoke old
writable descriptors, so imported/user-owned images are refused. Apply checks
the schema, manifest digest, runtime hash, device/inode/size, file ownership,
permissions and link count, holds an exclusive lock, and configures bounded
loop mappings from the held image descriptor. There is no physical-device
entry point and no execution of arbitrary plan commands.

The privileged runtime is a new root-private snapshot. Only a bounded compressed
archive is copied from the cache; the private copy must match the compiled
SHA-256 before extraction. The cache's extracted tree is never elevated.
Bubblewrap exposes only the four allocated loop mappings and the held journal,
with private mounts, PID namespace, temporary paths and no network. It retains
the mount capability for native storage tools. This boundary depends on
the host kernel and trusted, pinned runtime tools; it is not a kernel sandbox
for hostile filesystem images or arbitrary programs.

The separate image manifest is schema 1; it does not alter config/plan schema 2.
Receipts bind the manifest digest, record ordered `start`/`ok` events and sync
each event before proceeding. Repeat execution probes GPT geometry, filesystem
UUIDs/types and read-only filesystem checks before skipping destructive work.
An interrupted destructive operation is accepted only if its complete probe
passes; otherwise discard and recreate the disposable image. The mount tree is
rebuilt and unmounted on each run. Mount namespaces and auto-clearing held loop
descriptors provide crash cleanup. Execution has a ten-minute deadline and a
bounded diagnostic tail. This lane accepts no secrets.

Encryption, secret transport into target commands, chroot bootstrap, package
installation, bootloaders and Omarchy installation remain separate gates.
Nothing here marks any installation matrix cell supported.

Validation: `sudo env BOREA_RUNTIME_CACHE=/absolute/runtime-cache \
  target/release/xtask disk selftest --fs btrfs` (repeat for `ext4`).
Add `--family debian`, `--family fedora` or `--family suse` to select another
family bundle; Arch is the default test bundle.
The `disposable-image-storage` GitHub workflow runs these real filesystem tests
on ephemeral runners and uploads receipts and results. Tests include pending
format receipts, a corrupted ESP whose failed probe must not trigger formatting,
exclusive locking, two parallel images, loop cleanup and stale image rejection.
A real SIGKILL test checks that image loop mappings disappear. Power-loss
recovery and every possible signal timing are not yet proven. SIGKILL can leave
root-private runtime staging under `/tmp/borea-image-PID-*`; ordinary temporary
file cleanup reclaims it. The ephemeral test runner reclaims its own staging.

## Target bootstrap and Limine bootloader (B-0.3 / G03)

G03 extends image execution with closed target bootstrap and bootloader
installation on the verified storage layout. It requires network access to
retrieve upstream packages into `/target` and installs `base`, `linux`,
`linux-firmware`, `btrfs-progs`, `networkmanager`, `sudo`, `zram-generator`,
`openssh`, `limine` and `efibootmgr`.

```sh
sudo borea image bootstrap --family arch --manifest /absolute/test.image.json \
  --cache /absolute/runtime-cache --password-file /absolute/private-password-file \
  --journal /absolute/test.bootstrap.journal --yes
```

Target configuration sets hostname, hosts, locale generation, default user,
password, sudoers drop-in, NetworkManager/sshd service activation, `mkinitcpio -P`
initramfs build and Limine EFI/config setup. The password file must be a regular,
single-link file with no group/world permissions; symlinks, empty passwords and
multi-line passwords are refused. Its contents are sent to the bootstrap process
over stdin and are not written to argv, environment, receipts or diagnostics.
Root is locked and sudo requires the account password. Receipts sync
`storage_probe`, `pacstrap`, `system_config`, `initramfs`, `bootloader` and
`bootstrap-complete`; the final receipt means the on-disk boot configuration
probes passed, not that a VM boot was tested. G04 rejects B-0.3 journals using
the old `verified-bootable` final marker.
Repeat execution probes target files before skipping completed steps. Supply
the same password file when resuming an incomplete bootstrap; secrets are
intentionally not bound into the image manifest or journal.
Images bootstrapped before G04 used fixed credentials and must be treated as
compromised: do not boot or resume them; create a fresh image instead.

Validation: `sudo env BOREA_RUNTIME_CACHE=/absolute/runtime-cache \
  target/release/xtask disk bootstrap --fs btrfs` (repeat for `ext4`).
The selftest uses a canary password and refuses to report success if that canary
appears in the bootstrap receipt. Bootstrap success reports
`bootloader_configured: true`, `bootable: false`, and
`vm_boot_verified: false`; QEMU/SSH boot acceptance is a separate card.

### Arch boot-to-login smoke (F03, partial)

The ephemeral image workflow retains each Arch bootstrap image long enough to
boot it with QEMU/OVMF, then removes the image. Btrfs and ext4 are tested
separately. The smoke check requires the expected root filesystem to mount, a
root UUID in the kernel command line, the Btrfs `rootflags=subvol=@` flag only
on the Btrfs lane, and the `borea` serial login prompt. It does not yet test SSH,
account login, service health, or any desktop/session matrix cell, so it does
not promote a matrix cell to supported.

For a prepared image, run:

```sh
sudo target/release/xtask vm boot --image /absolute/test.img --fs btrfs \
  --serial-log /absolute/btrfs.serial.log
```

The VM runner requires `qemu-system-x86_64` and OVMF firmware. It uses software
emulation, QEMU snapshot mode and no network device; serial and QEMU error logs
are the boot evidence. The source image remains unchanged.
