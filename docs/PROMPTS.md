# Prompts for Claude and Codex

These are copy-paste prompts for running the plan with agents. They assume the
agent can read the repository and run commands. Keep the task small: one card,
one branch, one verification run.

The shared rules in `AGENTS.md` still apply. A prompt cannot override them.

---

## 0. Before any prompt

Give the agent:

- the repository path (`/home/luke/Work/borea-v2`),
- the card id (e.g. `C03`),
- the family/profile if the card is a matrix cell,
- permission to run local builds and VM tests, **not** real disks,
- any credentials as files, never inline.

Do not paste upstream docs wholesale. Link the evidence URL in the recipe.

---

## 1. Kickoff (D0)

```text
You are starting work on borea-v2 at /home/luke/Work/borea-v2.

Read, in this order:
1. AGENTS.md
2. docs/PLAN.md sections 1-6 and 8 (D0)
3. docs/ARCHITECTURE.md sections 1-5
4. docs/TASKS.md cards A01-A07 and F01

Task: execute D0 of the plan, starting with A01.
Rules:
- One card at a time. Stop after each card and report.
- Do not add dependencies without recording a D-nnn entry.
- Do not run anything against a real block device.
- Keep plans deterministic; add golden fixtures.
- Do not create GUI, ISO, USB, daemon or simulation code.

For each card, report:
- files changed
- commands run and their result
- what is not done / blocked
- the next card

Start with A01 and show me the tree and CI command before moving on.
```

---

## 2. Generic task card

```text
Repository: /home/luke/Work/borea-v2
Card: <ID> — <title>

Read AGENTS.md, then the card in docs/TASKS.md, then the docs it names
(ARCHITECTURE/RECIPES/MATRIX/TESTING as relevant).

Scope:
- Implement only this card.
- Stay inside the file boundary listed on the card.
- If you need a change outside it, stop and tell me which file and why.
- Do not refactor unrelated code or update pins.

Acceptance:
- The card's "Done when" is true.
- Run the card's "Verify" command(s). Paste the output.
- Add or update tests for the new behaviour.
- Update docs only where the card requires it.

Safety:
- Use image files, loop devices, containers or VMs. Never a real disk.
- Never put a secret in argv, a plan, a journal or a log.

Report: summary, files, commands + results, remaining limits, suggested next card.
```

---

## 3. Family adapter (C02–C05)

```text
Repository: /home/luke/Work/borea-v2
Card: <C02 Arch | C03 Debian | C04 Fedora | C05 openSUSE> — <adapter>

Context to read:
- docs/ARCHITECTURE.md §5, §6, §9
- docs/TESTING.md §5-6
- recipes/families/<family>.toml
- the existing adapters in crates/borea-distro/src/

Implement the family adapter:
- bootstrap with the family's native tool
- base packages, microcode, NetworkManager
- package install/query
- repo add/refresh with pinned keys
- initramfs and bootloader paths for the family (Limine on Arch, GRUB on the
  others; `loader = "auto"` already resolved)
- Btrfs root with `rootflags=subvol=@`, ext4 `/boot`
- host requirements for `borea doctor`

Hard rules:
- No `if family ==` outside crates/borea-distro.
- No shell strings; argv arrays only.
- Every mutating step has a probe.
- The adapter must not know about KDE/GNOME/Hyprland by name; it installs
  packages and enables units.

Verify:
  cargo test -p borea-distro <family>
  cargo xtask e2e --family <family> --profile minimal
  cargo xtask e2e --family <family> --profile minimal --root-fs ext4

Paste the artifact path from target/e2e/.../result.json.
If the VM cannot boot, stop and report the serial log tail; do not weaken checks.
```

---

## 4. Session recipe (D02–D10)

```text
Repository: /home/luke/Work/borea-v2
Card: <D0x> — <component>

Read:
- docs/RECIPES.md §4-6 and §11
- docs/MATRIX.md for the component's cells
- docs/ARCHITECTURE.md §10
- an existing recipe of the same kind

Deliverable:
- recipes/<kind>/<id>.toml with packages for every family that has a path
- matrix cells for all four families (native/third-party/source/unsupported)
- seeds under recipes/files/<component>/
- at least one check that proves the component is present
- a profile under recipes/profiles/ if this is a required cell

Rules:
- A missing family key means unsupported; do not invent package names.
- Third-party repos need a pinned key fingerprint in recipes/repos/.
- No seed may overwrite another component's seed destination.
- Gates need a reason, a remedy and an evidence URL.
- Do not copy upstream configs without a NOTICE file and O-19 resolution.

Verify:
  cargo xtask recipe lint
  cargo xtask matrix check
  cargo xtask plan golden
  cargo xtask e2e --family <family> --profile <profile>

Report the matrix cells you changed and whether each was promoted by a VM run or
only recorded as gated.
```

---

## 5. Fix a red matrix cell

```text
Repository: /home/luke/Work/borea-v2
Cell: <family> / <profile>
Artifact: <path to target/e2e/.../result.json>

The cell is red. Diagnose it using docs/TESTING.md §10.

Rules:
- Read result.json, journal.jsonl, serial.log before changing code.
- Identify the phase and the exact failing step/check.
- Fix the cause. Do not remove the check, hide the cell, or mark it unsupported
  to get green.
- If the cause is a genuine capability gap, stop and propose a decision entry
  with evidence.
- Re-run the same cell and paste the new result.json.

Report: root cause, fix, before/after artifact paths, any adjacent cells at risk.
```

---

## 6. Omarchy port (E01–E05)

```text
Repository: /home/luke/Work/borea-v2
Card: <E01-E05>

Reference (read-only): /home/luke/Work/borea
- crates/borea-core/src/catalog.rs
- crates/borea-core/src/omarchy.rs (trim_action, default_item, defaults_policy)
- crates/borea-core/src/defaults.rs
- crates/borea-defaults/src/
- docs/development/DEFAULTS.md

Read docs/OMARCHY.md and port only the catalogue, trim mapping and defaults
policy/state machine.

Do not port:
- the Arch -> Omarchy handoff, archinstall JSON, disk IDs
- Layer/LayerStep, live ISO file sources
- anything that installs Omarchy

Rules:
- Keep ids, labels and policy schema 1 compatible with the old files.
- Browser is never removed.
- No forced removal; blocked is reported.
- Keep the process-level fake-pacman test.
- Add a table test asserting every TrimItem maps to exactly one action.

Verify:
  cargo test -p borea-core omarchy
  cargo test -p borea-defaults --locked
  cargo xtask e2e --profile omarchy-trim

Report what was ported, what was deliberately dropped, and any behaviour change.
```

---

## 7. Code review

```text
Repository: /home/luke/Work/borea-v2
Review the diff for card <ID>.

Check, in order:
1. Scope: only the card's files; no unrelated refactors.
2. Plan purity: `plan` does no writes/network; deterministic output; digest.
3. Idempotency: every step has a probe; destructive steps are storage-only.
4. Secrets: no value in argv/env/plan/journal/log; canary test covers it.
5. Capability honesty: no cell promoted without an artifact; gates have reason,
   remedy, evidence.
6. Family boundary: no `if family ==` outside crates/borea-distro.
7. Tests: the card's verify command exists and is meaningful; no weakened checks.
8. Docs: only the docs the change requires.

Output: blocking issues, non-blocking suggestions, and a clear approve/request
changes. Do not rewrite the diff unless asked.
```

---

## 8. Release (D8)

```text
Repository: /home/luke/Work/borea-v2
Task: prepare v0.1.0.

1. Run the full required matrix:
   cargo xtask matrix verify --strict
2. For every green cell, confirm an artifact exists and is newer than the last
   recipe change. List any cell without one.
3. Build static binaries and hashes:
   cargo build --release --target x86_64-unknown-linux-musl
   sha256sum target/x86_64-unknown-linux-musl/release/borea
4. Update README, docs/MATRIX.md and docs/releases/v0.1.0.md with exactly what
   is green and what is gated. No unverified claims.
5. Run the live lane at least once per family, or record the skip reason.
6. Do not tag or push until I confirm.

Report: matrix summary, missing artifacts, release notes draft, commands run.
```

---

## 9. Handoff between agents

```text
Handoff for card <ID>.

State:
- branch: <name>
- last commit: <sha> <subject>
- done: <bullets>
- failing/blocked: <exact command + error>
- artifacts: <paths>
- next step: <one sentence>

Read AGENTS.md and the card before continuing. Do not redo completed work.
```

---

## 10. Prompt anti-patterns

Do not ask an agent to:

- "make it work on all distros" in one task — split by family/cell.
- "add support for X" without naming the matrix tier and evidence.
- "clean up" or "refactor" without a card; that is how scope grows.
- "just test it on your machine" — use the harness.
- "skip the check for now" — a skipped check is a false green.
- "copy the old implementation" — port only the named data and policy.

If a prompt would need more than one card, stop and split it.
