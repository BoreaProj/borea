# Capability matrix

This is the source of truth for what `borea plan` may offer. It is data:
`recipes/matrix.toml`. The tables below are generated from it by
`cargo xtask matrix render`; if they disagree, the TOML wins and CI fails.

**Revision:** `matrix-2026-11-13`
**Evidence last checked:** 2026-11-13
**Rule:** a `native` or `third-party` cell needs a passing VM artifact for that
family before release. No artifact, no green.

---

## 1. Tiers

| Tier | Meaning | What the user sees |
| --- | --- | --- |
| `native` | package in the family's official repositories | offered |
| `third-party` | documented external repo/COPR/OBS/APT with a pinned key | offered with a notice |
| `source` | built in the target from a pinned upstream commit | offered only with `--allow-source`, experimental |
| `unsupported` | no documented path as of the evidence date | refused with reason + remedy |

`native` includes official backports (Debian `trixie-backports` is a Debian
repository, not a third party).

## 2. Components

### 2.1 Base and desktops

| Component | Arch | Debian | Fedora | openSUSE |
| --- | --- | --- | --- | --- |
| minimal base | native | native | native | native |
| KDE Plasma | native | native | native | native |
| GNOME | native | native | native | native |
| COSMIC | native | **unsupported** | native | third-party |

### 2.2 Compositors

| Component | Arch | Debian | Fedora | openSUSE |
| --- | --- | --- | --- | --- |
| Hyprland | native | native (backports) | native | native |
| Niri | native | **unsupported** | native | native |
| MangoWM | native | third-party (ButterRepo) | third-party (Terra) | **unsupported** |

### 2.3 Shells

| Component | Arch | Debian | Fedora | openSUSE |
| --- | --- | --- | --- | --- |
| Noctalia (v5) | native | third-party (Noctalia APT) | native (F44+) | native |
| DUX / Dank Material Shell | native | third-party (OBS + backports quickshell) | third-party (COPR) | third-party (OBS) |
| end-4 (illogical-impulse) | source (pinned installer) | **unsupported** | **unsupported** (community only) | **unsupported** |

### 2.4 Supporting pieces

| Component | Arch | Debian | Fedora | openSUSE |
| --- | --- | --- | --- | --- |
| greetd + tuigreet | native | native | native | native |
| SDDM | native | native | native | native |
| GDM | native | native | native | native |
| NetworkManager | native | native | native | native |
| Flatpak | native | native | native | native |
| codecs (GStreamer/FFmpeg) | native | native (non-free where needed) | third-party (RPM Fusion) | third-party (Packman) |

SDDM, GDM and NetworkManager are part of the base/desktop recipes rather than
standalone matrix components; the table records the packaging assumption behind
those recipes. Only `greeter.greetd`, `apps.flatpak` and `system.codecs` have
cells in `recipes/matrix.toml`.

## 3. Session pairs promised in v0.1

A pair is the compositor + shell combination. The matrix gates components; a
pair is only promised when its row has an artifact. v0.1 promises exactly these
pairs, on every family:

| Pair | Arch | Debian | Fedora | openSUSE |
| --- | --- | --- | --- | --- |
| Hyprland only | required | required | required | required |
| Hyprland + Noctalia | required | required | required | required |
| Hyprland + DMS | required | required | required | required |

Other combinations are **planned**, not refused: the resolver allows them when
both components are supported and the shell's `compatible_compositors` list
includes the compositor, but `borea catalogue` marks them `untested` until a VM
artifact exists. They become promised one cell at a time.

This is deliberate. Claiming "every shell on every compositor on every distro"
without artifacts is how the previous repository lost credibility.

## 4. Gated cells and why

Every `unsupported` cell must carry a reason, a remedy and an evidence link.
This table is the current set; `cargo xtask matrix check` enforces it.

| Cell | Reason (as of 2026-11-13) | Remedy |
| --- | --- | --- |
| Debian + COSMIC | No COSMIC packages or documented Debian repository found; upstream packaging is not distro-ready | Use KDE/GNOME; track `pop-os/cosmic-epoch` packaging; source build is a post-v1 decision |
| Debian + Niri | No `niri` package in Debian 13 or backports; no documented third-party repo | Use Hyprland; source build tracked as D-011 |
| openSUSE + MangoWM | No package in Tumbleweed or a documented OBS project; upstream targets Arch/Fedora | Use Hyprland or Niri; source build tracked as D-012 |
| Debian + end-4 | Upstream documents Arch (and Nix); Debian path is WIP only | Use Noctalia or DMS, which are supported on Debian |
| Fedora + end-4 | Community packages only; no maintained COPR verified for F44 | Use Noctalia or DMS; revisit if a pinned COPR appears |
| openSUSE + end-4 | Upstream fork/WIP only; no maintained package | Use Noctalia or DMS |

`source` cells are not silently enabled. They require `--allow-source` and are
marked experimental in the plan notes. v0.1 ships:

| Source cell | Pin | Acceptance |
| --- | --- | --- |
| Arch + end-4 | upstream installer commit + Quickshell/Hyprland versions | VM boot + shell start, D7 |
| Arch + COSMIC | not source; native | n/a |

## 5. Apps

Apps are resolved per app, not per matrix cell. Each `recipes/apps/*.toml`
declares a package for a family, a Flatpak fallback, or neither. `borea` prefers
the family package and falls back to Flatpak when `apps.flatpak` is supported on
that family. A community repo (AUR, ButterRepo) is opt-in per app.

The table below is a design reference for the first app recipes. The source of
truth for an app is its recipe's `packages` map plus the `apps.flatpak` cell.
An app with neither a package nor a Flatpak is refused with
`E_UNSUPPORTED` when named in the config.

| App | Arch | Debian | Fedora | openSUSE |
| --- | --- | --- | --- | --- |
| Firefox | native | native | native | native |
| Chromium | native | native | native | native |
| Helium | third-party (AUR) | third-party (ButterRepo) | unsupported | unsupported |
| Zen | native/Flatpak | Flatpak | Flatpak | Flatpak |
| LibreOffice | native | native | native | native |
| ONLYOFFICE | Flatpak | Flatpak | Flatpak | Flatpak |
| Steam | native | native | native (RPM Fusion) | native |
| Lutris | native | native | native | native |
| Heroic | Flatpak | Flatpak | Flatpak | Flatpak |
| Vesktop | third-party (AUR) | Flatpak | Flatpak | Flatpak |
| Signal | native | native | native | native |
| Telegram | native | native | native | native |
| Amethyst Mod Manager | upstream Flatpak | upstream Flatpak | upstream Flatpak | upstream Flatpak |

The app list is not a promise that every app is installed by default. Nothing
is selected unless the config names it. The matrix decides whether a named app
is offered.

## 6. Family bootstrap evidence

These are the native tools each family's environment must provide. `doctor`
checks them; the adapter uses them.

| Family | Bootstrap | Base packages | Bootloader |
| --- | --- | --- | --- |
| Arch | `pacstrap -K -P /mnt` from the Arch ISO | `base linux linux-firmware` + microcode | GRUB |
| Debian | `debootstrap --variant=minbase trixie /mnt` | `apt` base + `linux-image-amd64` | GRUB |
| Fedora | `dnf5 --installroot=/mnt --releasever=<n>` | `fedora-release glibc systemd kernel` | GRUB |
| openSUSE | `zypper --root /mnt` (Tumbleweed) | `openSUSE-release systemd kernel-default` | GRUB |

Source pages and exact package names are recorded per recipe in
`recipes/families/*.toml` with an `evidence.checked` date. Package names must be
resolved against a live repository during D2–D4, not trusted from this table.

## 7. Known fragility

| Area | Risk | Handling |
| --- | --- | --- |
| Fedora release number | changes every 6 months | `release` in config or family default file; never hard-code in code |
| openSUSE Tumbleweed | rolling snapshot drift | pin a snapshot URL in the family recipe; refresh with a reviewed change |
| Third-party keys | rotation/expiry | fingerprint pinned; `matrix check` validates presence; fail closed |
| Noctalia APT | new repo, packaging may move | keep it `third-party`; verify in D6 |
| DMS on Debian | needs `quickshell` from backports | adapter enables backports explicitly; if unavailable, gate |
| end-4 | installer is a moving target | pin commit; source tier; VM gate |
| MangoWM on Debian | ButterRepo pins wlroots 0.19 beside Debian 0.18 | install only in the target, never on the host; verify no conflict |

## 8. How to change the matrix

1. Edit `recipes/matrix.toml`.
2. Add or update the evidence URL and `checked` date.
3. Run `cargo xtask matrix check`.
4. If promoting to `native`/`third-party`, add the recipe packages and run the
   e2e cell. Attach the artifact path in the PR/commit message.
5. If demoting, update the reason/remedy and any docs that promise the cell.
6. Never promote a cell based on a web search alone. The VM run is the evidence.
