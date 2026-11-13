# Omarchy trim and defaults

borea keeps two Omarchy tools from the old project:

1. **Trimmer** — remove selected Omarchy defaults (packages and web apps) from an
   existing install.
2. **Default manager** — keep removed defaults from coming back after a distro
   update, without fighting the user's own reinstall.

Both operate on an **existing Omarchy system**. Installing Omarchy is out of
scope (D-015); Omarchy's ISO and installer remain the only supported way to
create one.

---

## 1. Detection

`borea omarchy` refuses unless all of these are true:

- `/etc/omarchy` exists (or `omarchy` is on `PATH` and resolves to a file under
  `/usr/`),
- `/etc/os-release` reports Arch Linux,
- the user is root for mutating commands,
- `pacman` is present.

The version/commit is read from `/etc/omarchy` when it records one; otherwise
`unknown` is printed and the pinned mapping is still used, with a warning in the
plan output.

Detection is read-only and runs before any step is emitted.

## 2. Trim catalogue

Ported from the old `crates/borea-core/src/catalog.rs`
(`omarchy-design-fixture-v2`), with the same ids so existing plans keep working.
The upstream mapping is pinned to `OMARCHY_COMMIT`; re-check every entry when the
pin moves.

| Group | Item | id | Action on Omarchy |
| --- | --- | --- | --- |
| Apps | LibreOffice | `libreoffice` | `omarchy-pkg-drop libreoffice-fresh` |
| Apps | Obsidian | `obsidian` | `omarchy-pkg-drop obsidian` |
| Apps | Kdenlive | `kdenlive` | not preinstalled |
| Apps | OBS Studio | `obs-studio` | not preinstalled |
| Apps | Pinta | `pinta` | `omarchy-pkg-drop pinta` |
| Apps | Xournal++ | `xournal` | not preinstalled |
| Apps | Signal | `signal` | not preinstalled |
| Apps | Spotify | `spotify` | not preinstalled |
| Apps | Typora | `typora` | not preinstalled |
| Apps | LocalSend | `localsend` | `omarchy-pkg-drop localsend` |
| Apps | 1Password | `1password` | not preinstalled |
| Web apps | HEY | `hey` | `omarchy-webapp-remove HEY` |
| Web apps | Basecamp | `basecamp` | `omarchy-webapp-remove Basecamp` |
| Web apps | WhatsApp | `whatsapp` | `omarchy-webapp-remove WhatsApp` |
| Web apps | YouTube | `youtube` | `omarchy-webapp-remove YouTube` |
| Web apps | X | `x` | `omarchy-webapp-remove X` |
| Web apps | Zoom | `zoom` | `omarchy-webapp-remove Zoom` |
| Web apps | Discord | `discord` | `omarchy-webapp-remove Discord` |
| Web apps | ChatGPT | `chatgpt` | not preinstalled |
| Web apps | Grok | `grok` | not preinstalled |
| Web apps | GitHub | `github` | not preinstalled |
| Web apps | Figma | `figma` | not preinstalled |
| Web apps | Google Photos | `google-photos` | `omarchy-webapp-remove "Google Photos"` |
| Web apps | Google Maps | `google-maps` | `omarchy-webapp-remove "Google Maps"` |
| Web apps | Google Messages | `google-messages` | `omarchy-webapp-remove "Google Messages"` |
| Web apps | Google Contacts | `google-contacts` | `omarchy-webapp-remove "Google Contacts"` |
| Web apps | Fizzy | `fizzy` | not preinstalled |
| Developer tools | Docker | `docker` | `omarchy-pkg-drop ufw-docker docker-compose docker-buildx docker` |
| Developer tools | lazydocker | `lazydocker` | not preinstalled |
| Developer tools | Claude Code | `claude-code` | not preinstalled |
| Developer tools | opencode | `opencode` | not preinstalled |

"Not preinstalled" is not an error: the item is reported as already absent and
the command succeeds. This preserves the old behaviour, where the catalogue
listed more than a stock Omarchy ships.

### Browser rule

The browser is **never removed**. Omarchy's web apps run in Chromium, so
removing it breaks them. `borea omarchy trim` rejects a browser item and points
at the default-browser setting instead. A separate `borea omarchy browser <id>`
may install another browser and set it as default; it keeps Chromium installed.
This is the same rule as the old design and is not optional.

## 3. Trim execution

```sh
sudo borea omarchy trim libreoffice docker whatsapp
sudo borea omarchy trim --group web-apps --dry-run
borea omarchy trim --list
```

Execution rules:

- Packages are removed with Omarchy's own `omarchy-pkg-drop`, not `pacman -Rdd`.
  If the helper is missing, the command fails; it does not fall back to a forced
  removal.
- Web apps are removed with `omarchy-webapp-remove <name>` as the target user,
  through Omarchy's own helper.
- Each removal is a separate plan step with a probe: package absent, or launcher
  absent. Re-running is a no-op.
- A removal that would break a dependency is reported, not forced.
- The command prints what it removed, what was already absent, and what failed.
- `--dry-run` prints the exact argv list and changes nothing.

The trimmer is a thin planner over the same `Command` steps as the installer.
It does not mutate pacman's database and does not edit Omarchy's config.

## 4. Default manager

`borea-defaults` is installed into the target (and can be installed later on an
existing Omarchy system) at `/usr/local/bin/borea-defaults`. Policy lives at
`/var/lib/borea/defaults.json`, schema 1, no secrets.

### State machine

| State | Meaning | Next update |
| --- | --- | --- |
| `enforced` | removed and kept removed | if reinstalled by the update → `reintroduced` |
| `reintroduced` | a distro update brought it back | `enforce` removes it again |
| `restored` | the user installed it themselves | left alone forever |
| `blocked { reason }` | removal failed (e.g. a new dependency) | reported, never forced |

An item is only `restored` when the install happened **outside** an update
window. The update window is the discriminator; without it, a user install
during an update could be misclassified.

### Commands

```text
borea-defaults status
borea-defaults trim ID...
borea-defaults restore ID...
borea-defaults enforce
borea-defaults note-install          # package hook entry point
```

- `status` prints each item, its state and the last update time.
- `trim` removes now and records `enforced` (root).
- `restore` stops enforcing; it does not reinstall anything.
- `enforce` removes only `reintroduced` items (root).
- `note-install` reads package names on stdin and records installs; it never
  removes anything.

### Update windows per family

The old implementation only had Omarchy's lock. The schema keeps `update_window`
as an enum so the same policy works on other families later.

| Family | Window | Detection | Status |
| --- | --- | --- | --- |
| Omarchy (pacman) | `omarchy_update_lock` | `omarchy-update` holds a flock on `$XDG_RUNTIME_DIR/omarchy-update.lock`; mtime marks the start | implemented, ported |
| Debian (apt) | `apt_unattended` | log/marker from `unattended-upgrades` | designed, not wired |
| Fedora (dnf) | `dnf_transaction` | dnf history transaction id/time | designed, not wired |
| openSUSE (zypper) | `zypper_history` | zypper history id/time | designed, not wired |

v0.1 ships the Omarchy window and the package-manager query/removal adapters
for all four. The other windows are explicit follow-ups (E04, `O-13`-adjacent)
and are not claimed to work.

### Hooks installed on Omarchy

| Path | Purpose |
| --- | --- |
| `/usr/local/bin/borea-defaults` | the manager |
| `/var/lib/borea/defaults.json` | policy |
| `/etc/pacman.d/hooks/90-borea-defaults.hook` | after each transaction, `note-install` |
| `~/.config/omarchy/hooks/post-update.d/90-borea-defaults` | at the end of `omarchy-update`, `sudo -n borea-defaults enforce` |

The pacman hook only records, because pacman holds its lock while hooks run.
The enforcement runs after the update through Omarchy's own hook runner, which
keeps sudo authorised for the duration.

The manager is installed even when nothing is trimmed, so a later
`borea-defaults trim` works without reinstalling anything.

## 5. Policy file

```json
{
  "schema": 1,
  "distro": "omarchy",
  "upstream": "<OMARCHY_COMMIT>",
  "package_manager": "pacman",
  "update_window": "omarchy_update_lock",
  "user": "luke",
  "last_update_started": 1763000000,
  "items": [
    {
      "id": "libreoffice",
      "label": "LibreOffice",
      "packages": ["libreoffice-fresh"],
      "launchers": [],
      "state": "enforced"
    }
  ]
}
```

This is byte-compatible with the old `DefaultsPolicy` schema 1. The old file can
be read by the new manager without migration. Items not in the catalogue are
kept as data but cannot be trimmed by id until a recipe exists.

## 6. Safety

- `trim`, `restore` and `enforce` require root; `status` does not.
- No `-Rdd`, no `--nodeps`, no forced launcher deletion outside the user's own
  application directory.
- The manager never removes a package that another installed package depends on;
  it reports `blocked`.
- The manager never touches the browser, the kernel, the bootloader, systemd, or
  anything outside the catalogue.
- Removing a package that Omarchy itself needs is a bug in the catalogue; a test
  asserts every mapped package is in Omarchy's preinstall set at the pinned
  commit.
- Secrets: none. The policy holds usernames and package names only.

## 7. Porting notes (from the old repo)

| Old file | New home | Change |
| --- | --- | --- |
| `crates/borea-core/src/catalog.rs` | `crates/borea-core/src/omarchy/trim.rs` | keep ids/labels/groups; add action mapping table test |
| `crates/borea-core/src/omarchy.rs` (`trim_action`, `default_item`, `defaults_policy`) | same module | drop ISO handoff, disk IDs, archinstall config, `Layer` |
| `crates/borea-core/src/defaults.rs` | `crates/borea-core/src/defaults.rs` | keep schema 1; add `UpdateWindow` variants as data |
| `crates/borea-defaults/src/*` | same crate | keep the process-level fake-pacman test; drop live-image copy paths |

Do **not** port: the Arch → Omarchy handoff, archinstall JSON, disk object IDs,
the `Layer`/`LayerStep` machinery, or the live ISO file sources. Those belong to
the old installer and are replaced by the plan/apply steps.

## 8. Acceptance

```sh
cargo test -p borea-core omarchy
cargo test -p borea-defaults --locked
cargo xtask e2e --profile omarchy-trim          # fixture root
```

Fixture assertions:

1. Every catalogue item maps to packages, a web app, or `not_preinstalled`.
2. The browser is refused with the documented remedy.
3. `trim` emits exactly the expected argv and is idempotent.
4. `note-install` inside an update window marks `reintroduced`.
5. `enforce` removes reintroduced items and marks them `enforced`.
6. A user install outside the window marks `restored` and is never removed.
7. A removal with a dependent marks `blocked` and leaves the package installed.
8. The policy file round-trips schema 1 unchanged.

A live Omarchy VM run is recorded once per release if media is available;
otherwise the release notes say the fixture-only limit explicitly.
