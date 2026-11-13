# Arch Omarchy profile validation — 2026-11-13

- Workspace tests: 45 passed, none failed or ignored.
- Formatting and Clippy with warnings denied: passed.
- Matrix/recipe lint and generated matrix check: passed.
- Golden cases: 27 deterministic plans and four explicit refusals; this includes
  two optional Omarchy cases alongside the unchanged 29 required release cases.
- Secret boundary canary: passed.
- Both entrypoint SHA-256 values match the retrieved, pinned upstream sources.
- Read-only configure preview: Arch, Linutil preset, zero default app changes,
  Chromium retained, no mutation performed.
- Tests prove explicit-only removals, install-before-remove replacement ordering,
  conflict/unknown-id rejection, source consent, repository denial, non-Arch and
  invalid filesystem/loader rejection, policy scope and schema compatibility.
- `cargo xtask e2e --family arch --profile omarchy`: unsupported (exit 3), because
  the executor/runtime/VM lane remains unfinished. No VM artifact path exists.

No upstream install script, app removal, target disk operation or host Omarchy
configuration change was executed. The baseline source pins do not replace
repository trust and user-context installer integration before actual execution.
