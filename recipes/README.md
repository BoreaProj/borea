# Recipes

Data, not code. The schema is in [`docs/RECIPES.md`](../docs/RECIPES.md).

```text
matrix.toml       capability cells (source of truth)
families/         base bootstrap per family
repos/            third-party repositories and pinned keys
desktops/         kde, gnome, cosmic
compositors/      hyprland, niri, mangowm
shells/           noctalia, dms, end4
apps/             one file per app
profiles/         test/config presets
files/            seeds copied into the target
```

Only `matrix.toml` and the two profiles exist at plan stage. Component recipes
land with their task cards in [`docs/TASKS.md`](../docs/TASKS.md), each with an
`[evidence]` block. Do not add a recipe without matrix cells for all four
families.
