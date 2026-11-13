# Fixtures

`device.json` is an invented 20 GiB `/dev/vda` identity for planning only.
`plans/` has one JSON golden for each of the 29 required v0.1 cases: 25 draft
plans and four explicit unsupported results for unpinned third-party paths.
Two optional Arch Omarchy schema 2 plans cover upstream defaults and explicit
customisations, for 27 draft plans and four refusals total. A refusal fixture prevents silently inventing repository trust. It does not
count as a successful plan or a passing VM cell.

`cargo xtask plan golden` compares bytes and replans to check determinism.
Use `--update` only after reviewing the underlying change.

The CLI process tests create temporary roots, journals and policy files. They
never execute target binaries, package managers or host disk commands.
No bootable image or VM result exists yet.
