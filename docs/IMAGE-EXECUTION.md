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
  --cache /absolute/runtime-cache --journal /absolute/test.bootstrap.journal --yes
```

Target configuration sets hostname, hosts, locale generation, default user,
sudoers drop-in, NetworkManager/sshd service activation, `mkinitcpio -P` initramfs
build and Limine EFI/config setup. Receipts sync `storage_probe`, `pacstrap`,
`system_config`, `initramfs`, `bootloader` and `verified-bootable`.
Repeat execution probes target files before skipping completed steps.

Validation: `sudo env BOREA_RUNTIME_CACHE=/absolute/runtime-cache \
  target/release/xtask disk bootstrap --fs btrfs` (repeat for `ext4`).
