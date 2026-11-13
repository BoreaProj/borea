# Agent guidance for borea

This is the canonical guide for Codex and Claude Code. `CLAUDE.md` points here.
Read it before editing, then read the docs for the area you touch. Explicit
user instructions override this file.

## 1. Start from the plan, not from the code

1. Read [`docs/PLAN.md`](docs/PLAN.md) — scope and milestones.
2. Find your task in [`docs/TASKS.md`](docs/TASKS.md). Work on **one card**.
3. Check [`docs/MATRIX.md`](docs/MATRIX.md) before claiming any combination works.
4. Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) before touching the plan
   model, executor, or CLI surface.

If the task is not a card, stop and add or request one. Do not invent scope.

## 2. Hard boundaries

These are the mistakes that made the previous repository unshippable. Do not
repeat them.

- **No GUI, TUI dashboard, recorder, screenshots or design system.** Output is
  text and JSON.
- **No live ISO, custom initramfs builder, USB writer, boot splash or release
  attestation.** Borea runs inside a distro's own live environment and uses the
  target's initramfs tooling.
- **No daemon, socket protocol, Varlink, worker process or service supervision.**
  One process runs one plan.
- **No simulation engine.** `--dry-run`, golden plans and the VM matrix are the
  simulation story.
- **No dual-boot, alongside resizing, Secure Boot signing, Btrfs, snapshots,
  ARM, BIOS/legacy or non-systemd families** in v1. Refuse, do not guess.
- **No new crate or dependency** without recording it in `docs/DECISIONS.md`.
  Prefer the standard library and the distro's own tools.
- **No `if family == ...` outside `crates/borea-distro`.** Session and recipe
  code describes intent; family adapters produce the commands.

## 3. Capability honesty

- The matrix is the source of truth. A component is only `supported` when a VM
  run in `docs/TESTING.md` passed for that exact cell.
- Never add a recipe cell marked `supported` because a package name exists in a
  search result. Use `third-party`, `source` or `unsupported` with evidence.
- Gates return a `BoreaError::Unsupported` with a reason and a remedy. They do
  not warn and continue.
- Do not describe a plan as an install. A plan is a plan until `apply` passes a
  VM run.

## 4. Plan/apply discipline

- `plan` is pure: no disk access beyond read-only identity probes, no network,
  no writes outside the `--out` file.
- Every step has a stable `id`, a `probe`, and a `danger` level. If you cannot
  write a probe, the step is not idempotent and must be redesigned.
- Destructive steps (`partition`, `format`, `luks_format`) are only valid in the
  storage prefix of the plan and only after the target identity check passes.
- `apply` never prompts mid-plan. Confirmation happens before the first step.
- Resuming with a different plan or config digest must fail, not merge.
- Secrets never enter the plan, journal, argv, environment or logs. Use the
  secret-slot mechanism and redact every command line.

## 5. Safety when running things

- Tests use image files, loop devices, containers and VMs. They never touch a
  real block device.
- `borea apply` against a real device requires `--yes` **and** the device path
  matching the plan exactly (serial + size). Development runs use `--dry-run`
  or `xtask e2e`.
- Do not reboot, format, mount or erase anything on the host unless the user
  names the device and asks for it.
- Network access in `plan` is forbidden. Network access in `apply` is expected
  because package managers need it; tests use local mirrors or cached packages.
- Do not fetch and execute upstream install scripts unless the task card says
  so and the commit is pinned. Prefer packages.

## 6. Determinism and schemas

- Plan JSON is deterministic: sorted maps, stable ids, no wall-clock values in
  the digest. `created` is metadata, not content.
- Any schema change bumps `schema` and updates the golden fixtures in the same
  commit. No silent field additions to public JSON.
- Recipes are data. Changing a recipe is a reviewed data change, not a code
  change; include the upstream evidence and date.
- Version pins (upstream commits, key fingerprints, ISO checksums) live in one
  place per family under `recipes/` and `xtask/`.

## 7. Verification before you report

Run the smallest set that proves your change, then the workspace set:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo xtask matrix check
cargo xtask plan golden
```

For a distro or session card, also run the exact cell:

```sh
cargo xtask e2e --family debian --profile minimal
cargo xtask e2e --family arch --profile hyprland-noctalia
```

Report what passed, what failed, what was not run, and the VM artifact path.
"A plan exists" is not "it installs".

## 8. Working style

- Keep diffs focused. Do not reformat or refactor unrelated files.
- Match existing names and patterns. One concept, one name.
- Add a regression test for every behavioural fix; do not test implementation
  details twice.
- Update docs when a decision, command, schema or matrix cell changes.
- Leave the tree clean: `git status` should show only your task's files.
- Commit in small, reviewable units with a subject that names the card, e.g.
  `C05: openSUSE zypper root adapter`.

## 9. When blocked

Stop and write down the blocker in the task card or a new decision entry:
what was tried, the exact error, the evidence needed, and the smallest next
experiment. Do not weaken a check, pin or gate to get green.
