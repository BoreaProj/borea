# Embedded recipe inputs

`families/` contains bootstrap and boot defaults for four families.
`components/` contains draft package maps, session metadata, compatibility and
upstream evidence URLs. These are compiled into `borea-core`; no plan command
fetches remote data. `profiles/` contains full config presets, not merge layers.

The matrix describes packaging intentions. It does not imply a passed install.
Package maps were transcribed from the restart plan and require upstream and VM
validation. Missing third-party pins/source commits are hard planning gates.

Run `cargo xtask recipe lint`, `cargo xtask matrix check`, and
`cargo xtask matrix render --check`. See `docs/IMPLEMENTATION.md` for schema
limits and `docs/RECIPES.md` for the complete intended v0.1 schema.

`omarchy/upstream.toml` pins Linutil/Omarchy source evidence and the observed
Omarchy app baseline. `profiles/omarchy.toml` preserves all apps by default;
explicit Borea customisations are described in `docs/OMARCHY.md`.
