# Decisions and unresolved inputs

Recorded 2026-11-06. The current scope remains documentation and the interactive HTML mockup.
Earlier conversational suggestions are not final choices unless accepted below.

## Confirmed

| Subject | Decision |
| --- | --- |
| Current scope | Design/architecture documentation and HTML mockup; native workspace, Slint and worker remain deferred |
| Project license | MIT; see the repository's [LICENSE](../../LICENSE). Third-party components retain their own licensing terms |
| Omarchy baseline | Current HTML is the implementation reference, including trim; [OMARCHY.md](../../design/flow/OMARCHY.md) defines working behavior and acceptance checks |
| Builder | Fully custom Rust orchestration, not Buildroot |
| Speed | Aim for ≤2 minutes via cached assembly; full-source builds cannot have that guarantee |
| Repository write access | A push to `BoreaProj/borea` main was verified on 2026-11-07; publication still follows the user’s request for each task |
| Component CI/cache | GitHub Actions and release-hosted component artifacts are acceptable |
| Source build | Retain an optional local full-source rebuild |
| UI direction | Slint port of the locked HTML flow is the next implementation milestone |
| Live base | Independent of distro root filesystems; minimal init, Windpad in initramfs |
| Graphics direction | Open drivers, attached-screen GPU default, software fallback |
| Design | Every later page follows [LOCKED.md](../../design/flow/LOCKED.md); title/subtitle sweep after logo landing, central body content, locale settings on You |
| Extras | Removed from the current flow until a concrete page plan exists |
| Omarchy encryption | Required and on; reuse the account password for disk unlock |
| Administrator password | “Use as sudo password” defaults on; off requires a separate administrator password, as confirmed by the user |
| Password UX | No arbitrary length/complexity/punctuation restrictions; require nonempty matching entries and reject NUL. Native PAM/credential transport still needs verification |

## Open — no choice inferred

| Decision | Why it matters / next evidence |
| --- | --- |
| glibc or musl | Evaluate upstream component support, linking, Slint backend and runtime closure; static binaries are not a given |
| Custom Rust init or s6/dinit | Establish service supervision, device management, recovery and maintenance cost |
| Secure Boot: own keys or off | Own-key signing also needs a user enrollment/trust story; signing alone does not make retail machines trust Borea. Decide boot modes/bootloader and key custody |
| Slint GPLv3 or royalty-free terms | Select applicable Slint terms and distribution obligations before shipping; Borea's MIT license does not replace third-party licensing terms |
| Fonts | Confirm Source Serif 4, Inter, JetBrains Mono versus alternatives; pin and bundle chosen fonts and license notices |
| GitHub Actions/release publishing | Repository pushes are verified; Actions permissions, release publication, signing and artifact trust still need verification. This document does not authorize publication |
| Multi-monitor behavior | Choose primary screen, other-screen content, hotplug/focus behavior and scaling policy |
| Minimum RAM and tiers | Measure initramfs, graphics, UI and install working sets before fixing numeric limits |
| GPU override | Decide boot/recovery/settings location, persistence and fallback if selection fails |
| Laptop details | Need CPU model, integrated GPU, MUX availability/modes and video ports; observe which GPU owns each display. These details are unavailable in the cloud machine |
| Drop AMD OpenGL / LLVM | Confirm a compatible Vulkan UI path, actual Mesa dependency closure, and independent software fallback before removing features |
| Binary firmware | Decide whether redistributable vendor firmware blobs are acceptable. Strict all-source firmware conflicts with the named hardware targets |
| Target architecture / boot packaging | x86-64 is suggested by initial hardware; select bootloader, UEFI/legacy scope, initramfs format, compression and image layout |
| Debian/openSUSE flavours | Specify supported variants and install adapters; do not invent a catalogue |
| Custom shell/compositor catalogue | Confirm Noctalia dependencies and supported pairings; choose additional entries only with target support |
| Omarchy integration | Resolve installer handoff versus one Borea review/silent install, protected package removals and keeping removals across updates |
| Separate sudo authentication | Verify a supported explicit administrator/root-password policy for the selected target; sudo normally authenticates the invoking user. No passwordless interpretation |
| Bazzite and other flavour adapters | Confirm supported image/package installation methods and customization boundaries |
| Networking services | Select address/DNS management, connectivity probes, portal browser and privacy behavior |
| Cache trust and CI capacity | Pin source/toolchain versions, artifact signing/provenance trust, publication policy, storage and runner budgets |

These decisions do not block reviewing the documentation. They become explicit
prerequisites at the affected implementation milestone; no credentials or secret
values should be supplied in chat or committed to the repository.
