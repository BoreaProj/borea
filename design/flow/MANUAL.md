# Manual partitioner — deferred design

Manual partitioning is removed from the current [interactive HTML flow](borea-flow.html).
Disk choices are **Replace** and **Install alongside** only. The notes below
record the earlier prototype for possible future work; they are not active UI.

## What works in the mockup

- Two simulated disks: a 1 TB GPT NVMe drive with Windows, EFI, reserved/recovery
  partitions and free space; an empty 512 GB GPT SATA drive.
- Proportional disk maps, selectable partitions/free regions, labels, filesystem,
  size, mount point, encryption and planned-action details.
- Create partitions in free space, with MiB/GiB inputs and a leading-space offset.
- Edit filesystem, label, mount point, Format, LUKS and EFI System Partition flag.
- Resize/move with numeric boundaries and a size slider. Bounds cannot overlap
  neighbours or exceed the usable disk. Existing used-space estimates constrain
  shrink; XFS is grow-only in this fixture.
- Explicit confirmation before deleting existing partitions or replacing a disk's
  GPT table. Removing a new, unwritten partition cancels its queued operations.
- Suggested Omarchy layouts that preserve Windows, reuse a suitable EFI partition
  without formatting it, or create EFI and encrypted Btrfs root on the empty disk.
- EFI boot-partition selection, including cross-disk layouts with a dependency
  warning. The prototype mounts the selected EFI partition at `/boot`.
- Btrfs subvolume names and mounts, with add/remove controls and duplicate checks.
- Pending-operation review, destructive-operation indicators, Undo/Redo, reset,
  and JSON export. Export includes storage metadata only, never account passwords.
- Commit the draft with **Use this layout**. Disk and the installation overview
  then show the selected root device, disk identity, filesystem, encryption, boot
  partition and pending-operation count. Installation confirmation is reset.
- A single workspace keeps the disk map, partition list and compact details pane
  together. Create, Edit, Resize, boot settings and pending changes open focused
  floating windows; there are no partitioner tabs or numbered page panels.
- Smaller windows collapse the details pane into a Selection window. Lists adapt
  their visible row count with Previous/Next controls, preserving selection.
  Advanced partition flags live in More options to keep the main editor compact.

## Rules and boundaries

Coordinates are integer MiB; partition ends are exclusive. The first and last
MiB are reserved. Free regions are derived from the current partition geometry,
so deleting or shrinking partitions exposes usable space without a second source
of free-space state. Undo/Redo snapshots geometry, mounts, boot assignment and the
operation queue together.

The Omarchy draft requires one formatted, encrypted Btrfs root and an unencrypted
FAT32 EFI partition mounted at `/boot`. Encryption uses the account password
entered later in the flow; the partitioner never asks for a separate passphrase.
Existing unmounted, unformatted partitions are preserved. Duplicate mount points,
overlap, out-of-range geometry, missing root/EFI, and incompatible boot filesystem
or encryption settings block accepting the layout.

The 1 GiB root and 512 MiB EFI lower bounds are **preview planning checks**, not
verified Omarchy installation requirements. A small root produces a capacity
warning. Actual package-space requirements, filesystem health, BitLocker status,
LUKS resize ordering, firmware integration and upstream Omarchy compatibility are
outside this mockup. The used-space figures are fixtures, not filesystem scans.

This iteration covers GPT/UEFI partition planning. It does not claim working
MBR/legacy boot, LVM, RAID, filesystem conversion, data migration or an actual
storage backend. Native destructive operations remain a separate implementation
and verification milestone.
