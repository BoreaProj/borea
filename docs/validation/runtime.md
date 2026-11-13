# Independent runtime validation — 2026-11-13

Runtime delivery was verified on an x86_64 Arch Linux host as an unprivileged
user, without installing target-family package managers on that host.

Published payloads: [runtime-2026-11-13.1](https://github.com/BoreaProj/borea/releases/tag/runtime-2026-11-13.1).
All four builds and publication passed in
[the runtime build run](https://github.com/BoreaProj/borea/actions/runs/43496306003).

| Runtime | Integrity/tool inventory | Bootstrap execution | Offline/cache and isolation |
| --- | --- | --- | --- |
| Arch | Passed, 17 tools | `pacstrap -h` passed | Passed |
| Debian | Passed, 17 tools | `debootstrap 1.0.141` | Passed |
| Fedora | Passed, 17 tools | `dnf5 5.2.18.0` | Passed |
| openSUSE | Passed, 17 tools | `zypper 1.14.102` | Passed |

Each real published bundle was fetched through Borea, checked against the
embedded size/SHA-256 and extracted-tree contract, reused offline, and executed
inside Bubblewrap. Probes confirmed that host files, disk devices, environment
sentinels, Docker socket and session bus were absent; network namespace differed
from the host; the root was read-only; private temporary/log/cache writes worked.

The extractor normalizes owner access to handle mode-000 image files without
root, strips privileged metadata, and cleans incomplete staging without following
symlinks. Unit tests exercise these cases alongside traversal, corruption,
extra-file detection, offline misses, cache permissions, lock descriptors and
manifest origin/inventory checks. Fedora's log writes occur in a private tmpfs.

Workspace validation: 60 tests passed, none failed or ignored; formatting,
Clippy with warnings denied, runtime/matrix/recipe checks, generated matrix,
golden plans, secret canary, sparse image creation and Omarchy preview checks
passed. The 29 required installation VM cells remain unvalidated.

Local evidence is retained at `target/validation/runtime-final/`, including
`summary.json`, per-family probe/version/isolation/offline logs and the workspace
test log. The four extracted runtime caches remain under `target/runtime-cache/`.
These are ignored artifacts, not files published with the documentation.

This is proof of independent tool provisioning and isolated execution. No target
filesystem was partitioned/formatted, no installation was performed and no VM
artifact path exists. Next: bind the approved runtime digest into executable
plans/journals and implement narrowly scoped image-target execution with
identity checks, mount/chroot lifecycle, interruption and resume before VM boot.
