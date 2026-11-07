# Borea

A cross-distro Linux installer, designed around a quiet setup flow.

**Design preview:** this repository contains the interactive HTML mockup, design
specification and architecture plan. The native installer and live ISO are not
implemented yet. Installation and networking in the preview are simulated.

[![Borea design preview: boot, family selection, Omarchy trim and restart](design/promo/borea-design-preview.gif)](design/promo/borea-design-preview.mp4)

[Watch the 1080p / 60 fps video](design/promo/borea-design-preview.mp4) ·
[View the looping GIF](design/promo/borea-design-preview.gif) ·
[Browse the design](design/flow/LOCKED.md)

## Try the preview

Download [borea-flow.html](design/flow/borea-flow.html) and open it in a browser.
It is self-contained and needs no build step. Use the controls below the scene
to replay the animation or jump to a step.

To preview a local checkout instead:

```sh
git clone https://github.com/BoreaProj/borea.git
cd borea
python3 -m http.server 8000 --bind 127.0.0.1
```

Open <http://127.0.0.1:8000/design/flow/borea-flow.html>.

## The Omarchy flow

Boot → Get online → Choose your family → Arch flavours → Trim Omarchy → Disk →
You → Review → Install → Restart.

The current prototype includes:

- The boot highlighter loop, dot-to-arrow transformation and logo transition.
- Arch, Debian, Fedora and openSUSE family cards with matching colour changes.
- The Omarchy route, including individual and group trim selections for apps,
  web apps and developer tools, plus the preference to keep them removed after updates.
- Replace or Install alongside, mandatory Omarchy encryption, optional snapshots,
  account and locale settings, and review with direct edit routes.
- A silent installation preview with highlight progress, Tab to view output,
  and the same boot-style logo and arrow for restart.

Content scales with the window at 1080p, 1440p and 4K; smaller windows reflow
without page scrolling.

The complete prototype route is Omarchy. Other distro adapters, extra app picks
and manual partitioning remain future work. The trim interface records choices;
real package removal and update persistence still need upstream integration.

## Implementation direction

Borea is planned around a Slint interface and a fully custom Rust build system.
The live environment is intended to be independent of a distro base, with pinned
upstream components and verified caches. The warm ISO-assembly goal is under two
minutes; this is not a measured result or a promise for full source builds.

The next implementation milestone is a native workspace and Slint port of the
accepted flow, followed by the Omarchy worker integration. Open runtime,
licensing and hardware decisions are recorded explicitly.

## Design and plan

| Document | Purpose |
| --- | --- |
| [Locked design language](design/flow/LOCKED.md) | Typography, layout, colour, motion and shared page rules |
| [Omarchy functional baseline](design/flow/OMARCHY.md) | Required logic, trim execution contract and acceptance checks |
| [System architecture](docs/architecture/SYSTEM.md) | Live runtime, UI/worker boundaries, custom builder and resource policy |
| [Decisions](docs/architecture/DECISIONS.md) | Confirmed choices and inputs still needed |
| [Promo assets](design/promo/README.md) | Video, GIF, posters and recording instructions |
| [Original references](design/flow/references/README.md) | The six source HTML references and their provenance |

For contributions, start with the functional baseline and open decisions. New
pages should follow the locked design language, and implementation should keep
mockup behavior distinct from verified hardware and upstream support.

## License

[MIT](LICENSE) · Copyright © 2026 Borea contributors.
Third-party components retain their own licensing terms.
