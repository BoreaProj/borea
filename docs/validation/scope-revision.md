# Scope revision evidence — 2026-11-14

Evidence for [D-039](../DECISIONS.md). Every entry is a packaging *source*, not a
VM result. Nothing here promotes a matrix cell. Tiers follow PLAN §7.

## 1. Session sources per family

`→` marks a correction to `recipes/matrix.toml` as of `cfe489e`.

| Component | Arch | Debian 13 | Fedora | openSUSE Tumbleweed |
| --- | --- | --- | --- | --- |
| KDE | native `plasma` | native | native `@kde-desktop` | native `patterns-kde-kde_plasma` |
| GNOME | native `gnome` | native | native `@gnome-desktop` | native `patterns-gnome-gnome` |
| COSMIC | native `cosmic` | **unsupported** (no Debian 13 package; only OBS home projects for testing/sid) | native (F41+) | native in Tumbleweed `patterns-cosmic-cosmic` → re-verify, currently `third-party` |
| Hyprland | native | native via `trixie-backports` (from 2026-04) | native | native |
| Noctalia v5 | native `noctalia` in `[extra]` → package was `noctalia-shell` (v4) | third-party `pkg.noctalia.dev/apt`, package `noctalia` | native in default repos from F44 → only after the F45 bump (C08); currently mislabelled `native` on F43 | third-party OBS `home:neifua:Noctalia` → currently mislabelled `native` |
| DMS | `dms-shell` (matrix says native; Linutil removes it with an AUR helper, so re-verify) | third-party OBS `home:AvengeMedia:dms/Debian_13` plus `home:AvengeMedia:danklinux` deps | third-party COPR `avengemedia/dms` plus `avengemedia/danklinux` | third-party OBS `home:AvengeMedia:dms/openSUSE_Tumbleweed` plus danklinux deps |
| End4 | `source`, pinned commit (Hyprland only) | unsupported | unsupported | unsupported |

Compositors each shell supports (upstream docs):

| Shell | Compositors upstream supports | Borea default | Borea alternatives (where the family packages them) |
| --- | --- | --- | --- |
| Noctalia v5 | Hyprland, Niri, Sway, Scroll, Mango, Labwc, dwl, Umbriel | Hyprland | Niri, MangoWM |
| DMS | Niri, Hyprland | Hyprland | Niri |
| End4 | Hyprland | Hyprland | none |

Release pin finding: `recipes/families/fedora.toml` pins Fedora 43. Fedora 45
shipped on 2026-11-03, and F43 reaches end of life about six weeks later. F45
also requires a valid RPM signature by default, which matches Borea's pinned-key
rule.

Sources:
[Noctalia v5 install](https://docs.noctalia.dev/noctalia/v5.4.1/getting-started/installation/),
[Arch noctalia](https://archlinux.org/packages/extra/x86_64/noctalia/),
[DMS install](https://danklinux.com/docs/1.6/dankmaterialshell/installation),
[danklinux packages](https://github.com/AvengeMedia/danklinux),
[Debian Hyprland backports](https://linuxiac.com/debian-13-stable-users-can-now-install-hyprland-from-backports/),
[openSUSE COSMIC pattern](https://software.opensuse.org/package/patterns-cosmic-cosmic),
[COSMIC Epoch availability](https://www.linuxcompatible.org/story/cosmic-epoch-120-lands-with-rustnative-desktop-polish-avif-support-and-wayland-fixes/),
[Fedora 45 release](https://phoronix.com/news/Fedora-45-Released),
[Fedora 43 EOL window](https://computingforgeeks.com/what-is-new-in-fedora-45/).

Open risk: one Debian user reports that `trixie-backports` Hyprland fails on a
`libxkbcommon0` version floor. Only the Debian Hyprland VM cell can settle it.

## 2. Standalone flavours

A flavour is composed with the base family's native bootstrap tool, pointed at
the flavour's own repositories and keys. Borea does not run the flavour's ISO,
live session or installer (Calamares, Anaconda, `pika-installer-gtk4`).

| Flavour | Base | Native bootstrap | Repositories and keys | Constraints |
| --- | --- | --- | --- | --- |
| Omarchy | Arch | `pacstrap` | Arch plus Omarchy's pinned upstream (E06, D-033) | Upstream `install.sh` is interactive and user-context; E06 blocker stands |
| PikaOS | Debian **Sid** | `mmdebstrap`/`debootstrap` against PikaOS's mirror | `ppa.pika-os.com`, key `ppa.pika-os.com/key.gpg`; curated mix of Sid, experimental, deb-multimedia and PikaOS builds | Packages built for x86-64-v3: needs a CPU preflight and a v3-capable QEMU CPU model. Rolling Sid, not stable. Meta packages: `pika-gaming-meta`, `pika-multimedia-meta` |
| Nobara | Fedora | `dnf5 --installroot` with `nobara-repos` and `nobara-gpg-keys` | Nobara replaces Fedora's repos with monthly snapshot mirrors, plus `nobara-appstream`; RPM Fusion present but unused at install | Upstream does not support converting Fedora to Nobara; composing from Nobara's own repo files is the closest native path. AppArmor, not SELinux. Wayland-only sessions |

Each flavour cell is `third-party` with pinned repo URLs and key fingerprints in
`recipes/flavours/`. A flavour cell passes when the VM boots to the flavour's
default greeter, `/etc/os-release` names the flavour, and its kernel and repo
files are present.

Sources:
[PikaOS base](https://pika-os.com/),
[PikaOS repository mix](https://wiki.pika-os.com/en/faqs/pika-modifications),
[PikaOS repo key](https://git.pika-os.com/kernel-packages/linux-pikaos-template/src/commit/7377130b6743fe83bd302c9dfd6d154226890d40),
[Nobara repos](https://wiki.nobaraproject.org/modifications/fedora-repos),
[Nobara package list](https://wiki.nobaraproject.org/modifications/packages),
[Nobara build system](https://wiki.nobaraproject.org/modifications/source),
[Nobara conversion warning](https://copr.fedorainfracloud.org/coprs/man2dev/nobara/).

## 3. Linutil (ChrisTitusTech/linutil @ `e272190`, 2026-11-14)

Linutil is a reference for package names, repositories and customisations. It
is **not** run by Borea. Its scripts run on a live host with `sudo`, prompt
interactively and some pipe remote scripts to a shell. All three are forbidden
by AGENTS §4–5.

What Borea adopts, as recipe data:

| Linutil script | Borea use | Family data |
| --- | --- | --- |
| `system-setup/desktop-environments.sh` | Desktop package sets | Arch `plasma`/`gnome`; dnf groups `@kde-desktop`/`@gnome-desktop`; zypper `patterns-kde-kde_plasma`/`patterns-gnome-gnome`. Debian entries are Ubuntu metapackages (`ubuntu-gnome-desktop`); use Debian `task-*` instead |
| `system-setup/gaming-setup.sh` | Gaming customisation recipe (D14) | Arch `multilib`; Debian `dpkg --add-architecture i386` plus i386 libs; Fedora RPM Fusion free/nonfree plus `fedora-cisco-openh264`; Wine and Steam dependency sets |
| `system-setup/fedora/rpm-fusion-setup.sh`, `multimedia-codecs.sh` | Fedora codecs (already `third-party`) | `dnf swap ffmpeg-free ffmpeg --allowerasing`; tainted repos stay opt-in |
| `applications-setup/setup-flatpak.sh` | Flatpak recipe | Flathub plus `gnome-software-plugin-flatpak` or `plasma-discover-backend-flatpak` per desktop |
| `system-setup/zram-setup.sh` | Base zram | `/etc/systemd/zram-generator.conf` `[zram0] zram-size = ram / 2`, `swap-priority = 100`; package names per family |
| `applications-setup/*`, `gaming/*` | Optional app catalogue (D14) | Package and repository names only, each re-verified with evidence |

What Borea rejects:

| Linutil behaviour | Why |
| --- | --- |
| `dank-shell.sh`: `curl … install.danklinux.com \| sh` | Unpinned remote script. Use DMS's documented OBS/COPR repos |
| `system-setup/arch/omarchy.sh`: `curl … omarchy.org/install \| bash` | Unpinned remote script. E06 pins upstream |
| `noctalia-shell.sh` on dnf: Terra added with `--nogpgcheck` | Disables signature checking. Use F45 default repos |
| `noctalia-shell.sh`: installs v4 (`noctalia-shell`, `noctalia-qs`, `noctalia-legacy` repo) | Superseded by v5 (`noctalia`) |
| `noctalia-shell.sh` on apt: imports the key but writes no sources entry | Incomplete; Borea writes a `signed-by` source file |
| AUR helpers (`paru`/`yay`) for repo packages | Not needed: Noctalia, DMS and Hyprland are in Arch repos. AUR stays out of scope |
| Interactive `read` prompts | `apply` never prompts mid-plan |
