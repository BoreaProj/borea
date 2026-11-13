# Fixtures

Test data only. Nothing here is shipped at runtime.

```text
config/    valid and invalid borea.toml files
plans/     golden plan JSON
roots/     fake target roots (etc, var, usr, home)
pkg/       fake pacman/apt/dnf/zypper scripts
sysfs/     device identity fixtures
omarchy/   fake Omarchy install
images/    tiny prebuilt disk images for VM boot tests
```

Fixtures land with the cards that need them. See
[`docs/TESTING.md`](../docs/TESTING.md) for how each is used.
