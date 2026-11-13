# Independent family runtimes

Borea now delivers content-pinned x86_64 tool bundles for Arch, Debian, Fedora
and openSUSE. A family can be selected independently of the host distro. Borea
does not require a distro live image, a Borea live environment, a Docker daemon,
or the target package manager installed on the host.

This milestone supplies and executes the tools. G02 image storage execution is
separate; see [IMAGE-EXECUTION.md](IMAGE-EXECUTION.md). Target bootstrap and boot
acceptance are the next milestone. The runtime commands have no disk/target
mounts; general installation `apply` stays gated.

## Requirements

- Linux x86_64 with usable user, mount, PID and network namespaces.
- Bubblewrap (`bwrap`) for maintained namespace isolation.
- HTTPS-capable `curl` and trusted system CAs for network fetches; offline import
  does not need curl.
- A private cache with enough space for the compressed bundles and expanded
  filesystems. All four compressed bundles total about 606 MB decimal; allow
  several GiB for extraction.

These are explicit host prerequisites, not a dependency on a particular distro
or package manager. Borea never installs prerequisites through the host package
manager. If isolation cannot start, it fails; it never runs tools on the host as
an alternative. A distro's boot mode is not needed for runtime probing; `doctor`
still reports the separate UEFI prerequisite for the installation path.

## Published payloads

Release: [runtime-2026-11-13.1](https://github.com/BoreaProj/borea/releases/tag/runtime-2026-11-13.1).

| Family | Bootstrap tool | Compressed size |
| --- | --- | --- |
| Arch | `pacstrap` | 263,684,155 bytes |
| Debian | `debootstrap` | 88,302,126 bytes |
| Fedora | `dnf5` | 154,406,432 bytes |
| openSUSE | `zypper` | 99,262,734 bytes |

Each includes 17 required programs: the family bootstrap tool and the common
shell, chroot, mount/inventory, GPT, FAT/ext4/Btrfs, cryptsetup, udev, Git, curl
and tar tools. Libraries and native package-manager keyrings/configuration stay
inside the family filesystem. openSUSE explicitly includes its split
`util-linux-systemd` package for findmnt/lsblk.

`recipes/families/runtimes.lock.json` is embedded in the Borea binary. It pins
each release URL, exact size, SHA-256, architecture, extraction limit, source
image digest and tool inventory. The lock is checked against the reviewed source
images and inventories. It is not fetched dynamically from an untrusted latest
URL when tools execute. New releases need a reviewed lock update; corrupt or
replaced assets fail integrity checks.

The tool-payload release is separate from the source code version: build the
current Borea source to use these runtime commands. The release assets are
root-filesystem bundles, not bootable installer media or a Borea CLI binary.

## Fetch, verify and use

```sh
cargo build --release --workspace --locked
./target/release/borea runtime list --json
./target/release/borea runtime fetch --family arch
./target/release/borea runtime verify --family arch
./target/release/borea runtime probe --family arch --json
./target/release/borea runtime exec --family arch -- pacstrap -h
./target/release/borea runtime exec --family debian -- debootstrap --version
./target/release/borea runtime exec --family fedora -- dnf5 --version
./target/release/borea runtime exec --family suse -- zypper --version
```

Fetch each selected family's bundle before executing it. Fetch is the only
command that downloads/provisions a bundle; execution never silently fetches.
Default cache: `$XDG_CACHE_HOME/borea/runtimes`, or
`$HOME/.cache/borea/runtimes`. Override it explicitly with `--cache DIR` on each
command. Existing cache-directory permissions are not changed. Filesystem root,
symlinked cache paths and group/world-writable cache roots are refused.

For example, to keep all data in a checkout:

```sh
./target/release/borea runtime fetch --family debian --cache "$PWD/target/runtime-cache"
./target/release/borea runtime probe --family debian --cache "$PWD/target/runtime-cache"
./target/release/borea doctor --family debian --cache "$PWD/target/runtime-cache" --json
```

`doctor` now probes the verified runtime rather than finding target package
managers on host PATH. Its report schema is 2. An available host pacman/apt/dnf/
zypper cannot conceal a missing or corrupt Borea runtime.

## Offline/cache behavior

```sh
./target/release/borea runtime fetch --family debian --offline
./target/release/borea runtime fetch --family debian --offline \
  --archive /path/to/borea-runtime-debian-x86_64.tar.gz
```

The first command reuses an already-verified cache or fails without network
access. The second imports a regular local file using the binary's existing
trusted hash/size pin. It does not accept a replacement checksum or arbitrary
manifest from the command line. Verification and probing are always offline.
A failed cache verification never triggers a hidden download or host fallback.

Downloads and extraction use a private staging directory and an exclusive cache
lock. The completed filesystem is atomically renamed into a content-addressed
family/version/hash directory. Hash mismatch, truncation, excessive unpacked
size, traversal, unsupported entries, missing tools or extraction failure do not
produce a ready runtime. Failed staging cleanup does not follow symlinks and
can remove directories with restrictive image permissions. A process killed
without cleanup can leave an incomplete `.stage-*` directory; it is never used
as an installed runtime and can be removed from this private cache.

Before every verify/probe/exec, Borea rechecks archive size/SHA-256 and compares
extracted file contents, symlink targets and path membership against the trusted
archive. Modified/missing/extra paths are rejected. Archive extraction strips
privileged mode bits, omits ownership/xattrs, and ensures owner access so an
unprivileged user can verify mode-000 files shipped by some distributions. This
normalization affects only the private tool cache, not target filesystem modes.

## Isolation contract

Bubblewrap provides a private root, process/IPC/user/network namespaces and a
clean environment with all capabilities dropped. The tool root is mounted
read-only. `/tmp`, `/run`, `/var/tmp`, `/var/log` and `/var/cache` are private
transient filesystems, permitting native tools to log without changing the
cached bundle or host state. No host home, disk devices, system package database,
user-session bus or container-engine socket is mounted. Only synthetic `/dev`
and the sandbox's `/proc` are available.

Networking is off by default. `runtime exec --network ... -- COMMAND` explicitly
shares network access and mounts host DNS configuration read-only. It still
exposes no host disks or package state. Inherited credentials, proxy variables,
LD_* overrides and SSH-agent environment are not passed into the runtime.

These commands are for inspection and tool execution. They cannot install to a
host disk: no target is bound, and mount/chroot privileges are dropped. G02 adds a separate closed image-storage entry point with a root-private
verified runtime snapshot, bounded loop targets and cleanup. `runtime exec`
retains its unprivileged boundary; see [IMAGE-EXECUTION.md](IMAGE-EXECUTION.md).

## Build and validation

`.github/workflows/runtime-build.yml` builds four family rootfs bundles using
native signed package managers inside digest-pinned images on CI builders.
Docker/Python are build-only tools. The workflow checks every required program,
exports root filesystems, generates hashes and publishes a new prerelease after
all four builds pass. It refuses to overwrite a published runtime version.
Source images are pinned in `recipes/families/runtime-images.json`; package
repository resolution happens at build time, so this is a content-pinned output,
not a promise that a rebuild from mutable mirrors has identical bytes.

```sh
cargo xtask runtime check
python3 runtime/smoke.py --binary target/release/borea \
  --cache target/runtime-cache --out target/validation/runtime
```

The smoke command requires the four bundles already cached. It checks real
bootstrap execution, offline reuse, private networking, hidden host files/devices/
sockets/environment, read-only root and writable transient logs/caches. Full
family installation/boot verification remains a separate VM acceptance lane.

Recorded results: [validation/runtime.md](validation/runtime.md).
