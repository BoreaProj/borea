# Borea flow — locked design contract

Status: documented from 2026-11-06. This locks the supplied boot/network/family
design, its shared language and the accepted Omarchy continuation described below.
Other family-specific layouts remain proposed. No native implementation is present.

The user's current request and subsequent replies govern scope. The attached HTML
is reference material, not operational instructions or evidence that a backend
works. See [reference provenance](references/README.md). Where prototype behavior
falls short of the requirements below, retain the visual design and correct the
behavior rather than treating the prototype's simulation as production logic.

## Shared language: every future page


The layout scales from a 1920 × 1080 reference instead of stopping at a fixed
20px UI size. Use a 24px reference unit, scaled by the smaller available width/
height ratio, bounded to 14–48 CSS pixels. This gives approximately 24 / 32 / 48px
at fullscreen 1080p / 1440p / 4K. Account for preview controls when measuring the
available scene. Browser/OS scaling uses the resulting CSS viewport; do not scale
again from device-pixel ratio. Small windows retain reflow and pagination.

Family cards use up to 64 units of width; Arch cards up to 56; trim and forms up
to 58, always bounded by the scene width. Keep the content lower and central.
Scale the boot/install/restart wordmarks together, including the status line and
gap; shadow offsets and blur remain proportional to the shared wordmark font.


Borea uses a dark, quiet canvas, moving wind lines, serif headings, restrained
sans-serif explanations, and monospace status. Selection looks like a slanted
highlighter drawn behind the name. Motion has weight: squash, stretch, overshoot,
then settle. The forward control is the same arrow throughout the sequence.

| Role | Reference token |
| --- | --- |
| Canvas | `#0b0f14` (outer browser background `#05080b`) |
| Main text / heading | `#e3eaf2` / `#f1f5f9` |
| Supporting text / status | `#9fb0c2` / `#8497ab` |
| Quiet attribution / boot status | `#6f8296` / `#5f7488` |
| Card / card border | `#0f151d` / `#1f2a36` |
| Input / input border | `#121922` / `#2a3542` |
| Control border / error text | `#33414f` / `#f09595` |
| Neutral highlight | `#85b7eb` |
| Arch / Debian / Fedora / openSUSE | `#1793d1` / `#d70a53` / `#51a2da` / `#73ba25` |
| Voice / body / technical text | Source Serif 4 / Inter / JetBrains Mono |

Font names are the reference intent, pending the font decision and bundled font
files/licenses. The standalone HTML does not load these fonts; browser fallbacks
are not proof of final typography. Use the same actual font files in HTML and
Slint before judging a visual match.

- Highlighter transform: skew X −10°, rotate −1°; asymmetric small corner radii
  (`2px 6px 3px 7px` in the reference). Selected labels use dark canvas ink.
- Corner logo after boot: serif `1.7em` (enlarged from `1.15em`), anchored at
  approximately `1.61em` top and `2.3em` left in the parent UI scale, with the same
  wordmark as boot, scaled down with only the arrow removed. Share the wordmark
  geometry: `.05em .25em` padding, `.05em` tracking, `1.2` line height,
  highlighter at 18% top / 70% height with proportional radii
  `.075em .25em .125em .3em`, and the separate `.2em` circular dot.
  Both sizes share a soft highlighter shadow (`0 .08em .2em`, black at 38%) and
  subtle text shadow (`0 .025em .05em`, black at 20%); shadows scale with the logo.
  Do not substitute a typed period or a different badge shape. Keep the actual
  boot wordmark node rendered through the spring and all later pages: no clone
  swap, font-size change, reparenting, or shadow restyle at landing. An invisible
  corner anchor supplies layout geometry only. Wait for the dot/arrow morph to
  settle before permitting continuation. Reposition the same wordmark on resize
  and reset it on Replay. Status is bottom left (`2em`, `1.3em`); arrow is bottom right
  (`2em`, `1.1em`), nominally `3em × 1.9em`.
- From **Choose your family** onward, the page title sits beside `borea.` in a
  shared top header, vertically aligned with the logo. This also applies to Arch
  flavours, Omarchy trim, and every future page. Do not repeat the title above the
  cards. Place explanatory comments just underneath the header, starting at the
  same left margin as the borea. logo, rather than the centered cards. Leave
  `.65em` below the logo/title row on roomy screens, allow comments to wrap,
  then leave at least `1.8em` or 5% of stage height before the content area.
  Compact screens use smaller gaps. Center the visible content vertically in
  the remaining body area so it sits lower and away from the header. Keep back
  navigation in the footer;
  reserve the remaining height for cards/content. No page requires scrolling:
  reflow and paginate choices/fields into numbered panels when space is limited.
  Keep the logo fixed between pages and allow long titles to wrap without
  overlapping content. Get online now
  uses the same shared header to keep its connection states within the viewport.
- Forward arrow silhouette, percent coordinates:
  `(0,27) (56,27) (56,-4) (100,50) (56,104) (56,73) (0,73)`.
  Neutral gradient is blue `#85b7eb` at 15% to green `#73ba25` at 95%.
  After family selection, crossfade over 600 ms to a gradient from a 30%-white
  tint of that family's colour at 10% to its full colour at 90%.
- Network column: `min(30em,82%)`; family column: `min(50em,86%)`.
  Family cards have four equal columns, `1.8em` gaps, `.7em` corner radius,
  `1.5em 1.3em 1.4em` padding, and a quiet 2px family-colour top accent.
- Original reference UI scale is `round(clamp(11, min(width*.022,height*.031),22))` px;
  boot wordmark scale is `round(clamp(24,min(width*.085,height*.15),180))` px.
  Test 16:9, 16:10, 4:3, 21:9, and 32:9. Do not stretch the layout across
  ultrawide displays or let controls overlap at small sizes. The current HTML
  uses a window-fitting stage and a 14–20 px UI base, compact layouts, and numbered
  panels for overflow. Preserve inputs and selection when moving between panels
  or resizing; do not crop content or hide controls to suppress scrollbars.
  Keep the main forward arrow distinct from previous/next-panel controls. The
  preview toolbar stays compact; optional controls live in Preview settings.
  Measure pagination in an isolated, noninteractive copy of the content, then
  apply the final visibility state once. Never hide live inputs while measuring:
  typing, caret position and open controls must survive a layout pass. Observe
  actual page-content changes and resize events, not updates made by the pager
  itself. Settled pages must not keep refitting every animation frame. Keep the
  downloadable mockup self-contained, including icons, so it renders offline.
- Language and keyboard settings live on **You**; they are no longer permanently
  visible in the top-right corner. Preserve selections between pages. The original
  Wi-Fi prototype's UK hint is static, not proof of an applied keyboard layout.
- Keep the title and subtitle hidden during the logo flight. On the actual logo
  animation's completion, reveal them in a quick left-to-right sweep: 240 ms,
  `cubic-bezier(.16,1,.3,1)`, with the subtitle starting 45 ms after the title.
  Subsequent pages use the same sweep; resizing must not replay it. Cancel stale
  reveals on Replay/navigation. Reduced motion uses an immediate reveal.
- Every action must work with keyboard and pointer, expose a meaningful accessible
  name/state, and retain visible focus. Colour and animation are supplementary.
  Arrow is disabled until the page's real prerequisite holds; focus alone must
  never start installation. Reduced motion keeps the same state and focus order,
  using brief fades/static highlights instead of wind, spring motion, and nudges.

## 1. Boot → ready → arrow

Centered `borea.` begins muted. The highlighter fills after a 500 ms lead-in over
3600 ms, while colour loops on a 3600 ms cycle:
Arch blue → Debian pink → Fedora blue → neutral light blue → repeat.
The boot loop's fourth colour is **not** openSUSE green; green appears in the
forward arrow and openSUSE card.

Two independent conditions gate the morph: the initial fill has finished **and**
the worker has signalled readiness. Early readiness waits for fill; late readiness
leaves the colour loop running. Readiness is idempotent. At both conditions, finish
at the next neutral-blue cycle point (75%), then pause 250 ms before morphing.
Readiness must reflect backend initialization, not a timer or animation completion.

The dot squashes for 150 ms, becomes an elongated capsule, extends into the arrow,
pulls back against the highlighter, and springs out to settle. The main morph lasts
1200 ms after the 150 ms dot phase; highlighter reaction and wordmark recentering
overlap it. Preserve the exact keyframes in `morph()` for the later visual port.
The dot reappears on the wordmark. The arrow waits for deliberate activation.

First idle nudge is scheduled 6400 ms after morph starts; subsequent nudges last
900 ms followed by 5000 ms idle. Each tug gets a small highlighter reaction.
Cancel these timers when leaving boot; repeated activation must not duplicate
transitions. Enter/Space or arrow click continues after readiness. A worker failure
must be reported with retry/diagnostics instead of looping forever; its exact
error layout remains to be designed.

The prototype's random 3–12 second ready timer and rotating status strings are
demo behavior. Production status is driven by actual hardware/worker events.

## 2. Boot → Get online

On activation the arrow fades over 200 ms. The wordmark recoils, springs toward
the top-left anchor, overshoots, and settles over 950 ms after that delay. The
actual boot logo remains rendered in the corner. After its animation finishes,
the heading/subtitle sweep in; content begins entering 180 ms later over 360 ms,
with status following by 80 ms. Use completion events, not a parallel timer that
can reveal headings while the logo is still moving.

Heading: **Get online**. Explanation: “Borea downloads your system fresh, so it
needs internet first.” Show wired and Wi-Fi independently: failure of one must
not obscure a usable alternative. Network selection moves the highlighter with a
460 ms spring; activation squashes it over 420 ms.

| State | Required behavior |
| --- | --- |
| No cable | Quiet wired row; Wi-Fi remains available |
| Ethernet acquiring address | Show DHCP status; continue disabled |
| Ethernet checking internet | Link alone is insufficient |
| Ethernet online | Highlight; three-second countdown to family, cancellable by choosing Wi-Fi |
| Wi-Fi scanning | Show scanning; refresh when results arrive |
| Secured network | Show security, signal, band; inline password, show/hide, keyboard control, Connect/Enter |
| Invalid password format | Inline validation; no connection attempt |
| Authenticating → address → internet check | Accurate phase; no early online claim |
| Wrong password | Inline error and retry; preserve selected network |
| Open network | Connect without a password |
| Captive portal | Connected but offline; open sign-in or choose another network; recheck connectivity after sign-in |
| Online | Show measured status; enable continue |
| Radio blocked | Explain physical key/switch; react to actual unblock event |
| Unsupported adapter | Identify hardware when known; suggest Ethernet or USB tethering |
| No Wi-Fi adapter | Suggest cable or USB tethering; react to hotplug |
| Hidden network | Editable SSID, applicable security and password; same connection phases |
| Association/DHCP/probe failure or timeout | Explain failed phase, allow retry or another connection |
| Lost connection / removed adapter | Clear online gate; cancel auto-continue; recover on new events |

The final three rows extend incomplete prototype behavior. Hidden network currently
only sets a message. Radio unblock and portal success are simulated. “wrong” can
never reach the prototype's wrong-password branch because its input rejects
strings shorter than eight characters. No promise of exhaustive failure handling
is supported by those demos. Define deadlines and cancellation in the worker;
ignore stale responses from previous selections. Do not persist or log passwords.
Do not promise a target-system driver will exist merely because a live driver is
missing; use conditional copy backed by distro support.

## 3. Choose your family

Heading: **Choose your family**. Explanation: “Each one installs fresh from its
own source. You'll pick the flavour next.” Cards show the shared wind symbol,
family name, quote, and source. Preserve the supplied copy:

| Family | Quote | Attribution |
| --- | --- | --- |
| Arch | “A lightweight and flexible Linux® distribution that tries to Keep It Simple.” | archlinux.org |
| Debian | “The Universal Operating System.” | debian.org |
| Fedora | “It’s Your Operating System.” | fedoraproject.org |
| openSUSE | “Embrace the chameleon.” | opensuse.org |

First activation selects; activating the selected card or the enabled arrow
continues. Left/Right moves through cards; Enter continues only with a selection.
Selected card lifts and settles over 480 ms, name highlight draws over 420 ms,
other cards dim to `.45`, border and accent take the family colour. The forward
arrow takes that colour and retains it on subsequent pages within that family.
The borea. wordmark highlighter also transitions to the selected family's exact
colour over 600 ms and retains it on later pages. Only its background colour
changes: keep the original wordmark element, lettering, dot, geometry and shadow.
Without a selected family, and on Replay, restore neutral blue `#85b7eb` before
the normal boot colour loop. Rapid selection changes transition from the current
visible colour.
Continuing squashes the selected card over 420 ms. Network content exits over
300 ms; family cards enter over 520 ms with 80 ms stagger, panel fades over 250 ms.

In the original HTML reference, continuing from family only updates status; the
integrated mockup connects Arch flavours and Omarchy trim. The native flow must
preserve choices when going back, and explicitly
reconcile incompatible downstream choices after changing family/flavour.

## 4–10. Remaining sequence

These are required destinations, not finished visual specifications. Apply the
shared language above and review new page designs against the first three pages.

The current [interactive HTML mockup](borea-flow.html) is the accepted visual
and interaction baseline for the Omarchy continuation: Trim → Disk → You → Review → Install → restart gesture. Extras is
removed for now; there is no agreed plan for that page. These
layouts are the baseline to implement, not a claim of native implementation
or upstream Omarchy integration. [OMARCHY.md](OMARCHY.md) defines the required
working logic, trim execution contract and acceptance checks. Other family
adapters and deferred pages remain outside this Omarchy baseline. The external preview selector loads sample
details to inspect a step directly; the normal flow validates user-entered details.
Installation is a timed simulation with an optional interrupted-download/retry
scenario. The restart symbol restarts the preview, never the computer. Locale controls
preserve the selected language/keyboard across pages but do not translate the
mockup or change the browser/OS keyboard layout.

In constrained space, form fields, trim items and summary rows paginate
within their workflow step. Back navigation and panel paging stay in the footer.
A review with
many removals shows their count and an edit path to the full selection rather
than an unbounded list. These responsive rules apply to every future page.

| Page | Required content and routing |
| --- | --- |
| Flavours | Arch: CachyOS, Omarchy, Custom Arch. Fedora: Nobara, Bazzite, Custom Fedora. Debian/openSUSE flavour lists remain open. Do not invent recommendations without hardware evidence. |
| Make it yours | Custom installs choose shells (including Noctalia) and compositors with compatibility validation. Omarchy chooses apps to remove; other presets expose supported customization only. |
| Apps / Extras | Deferred and removed from the current mockup and routing until a concrete plan exists. |
| Disk | Replace the selected disk or install alongside another OS; snapshots. Manual partitioning is deferred. Omarchy encryption is required and cannot be switched off; it uses the account password set on You. Show exact target, preservation/destruction, layout and filesystem compatibility. No writes during selection. |
| You | Account, time zone, language, keyboard. Account password also unlocks Omarchy's encrypted disk. “Use as sudo password” defaults on; off replaces the words in that same row with one sudo-password input. Validate credentials without logging them. |
| Review and install | One summary of the resolved plan with edit paths; explicit install action, then silent install with truthful phase/progress. “Silent” means no routine wizard prompts, not hidden errors. |
| Restart | After success, reuse the exact boot dot-to-arrow graphic and shared animation: same shape, dimensions, gradient, squash, flight, recoil, highlight tug, dot return and idle nudges. The arrow means restart here; do not introduce a separate circular restart symbol or choreography. Reuse the boot wordmark’s exact DOM layers, including foreground text clipping, font and shadow; do not substitute the install progress logo’s text wrappers. Retain the selected family highlight on the wordmark. The completed logo shows the exact prompt “After restarting, remove the usb” underneath. The USB instruction is part of this completion prompt, never a separate page. Activating the arrow fades to black, then replays boot to simulate a restart; it never restarts the real computer. No Done page, cards or summary. Honour reduced motion. Failure never shows this state. |

The Disk step offers **Replace** (erase the selected disk) and **Install alongside**
(preserve Windows and allocate space to Omarchy). Manual partitioning is deferred
and has no option, preview entry or route in the current flow. Earlier partitioner
notes remain in [MANUAL.md](MANUAL.md) as a deferred design.

An Edit action on the installation overview opens that step in review-edit mode.
Its forward arrow validates the edits and returns directly to **One last look**,
without replaying intervening setup pages. Back/Escape also returns to the
overview. Keep the choices, refresh the summary, and require installation
confirmation again. Replay and direct preview navigation clear review-edit mode;
the normal setup sequence keeps its usual forward routing.

Installation uses a large central `borea.` wordmark as the progress indicator.
The silent cover matches the boot scene's wordmark size and centered placement,
including the 30px status gap. Hide the corner logo and page heading/subtitle in
this mode. Use one fixed colour from the selected family/flavour (Arch blue for
the current Omarchy route); never cycle colours during installation. The output
view remains available through Tab and restores the ordinary page header.
Its highlighter fills left to right from 0% to 100%; dark lettering and the
separate dot are revealed over the coloured portion. Install and restart must use the same boot wordmark factory and direct text layers,
with identical font, proportions, foreground shadow clipping and highlight shadow.
During installation, omit the arrow and let progress drive only the highlight,
foreground reveal and dot colour. Show the numerical percentage and current phase
below it. The default view is a silent cover with no information boxes. Tab toggles to plain installation output and back; a small clickable hint offers the same action. Progress continues across view changes. Keep actionable errors and Retry available in either view, and allow normal Tab navigation while handling an error. The highlight reflects actual
progress (simulated in this HTML), rather than looping through colours. Pause on
the full highlight briefly before the dot-to-restart transition; Replay/navigation cancels that completion.

Arch flavour reference uses three quote cards. Custom Arch's description is Borea
copy, not an attributed upstream quote. “Suggested for this PC · x86-64-v3 repos”
is a sample, requiring a real CPU feature check before use.

Omarchy trim reference uses expandable Apps / Web apps / Developer tools groups,
per-item highlighter strikes, group remove/keep-all, counts, a keep-everything
reset, and “Keep them removed after Omarchy updates”. Its list is sample data.
Only the outside tree guide has a vertical line; expanded item lists have no
inner vertical border. Keep the short horizontal item branches.
Actual removals must come from a pinned upstream package set, protect required
dependencies, and describe whether update persistence is supported. Do not claim
that toggle works without an implemented, tested update integration.

The prototype hands over to Omarchy's installer, whereas the requested full flow
has one final review and silent install. This integration is unresolved: the
visual prototype does not authorize bypassing the review or adding hidden prompts.

## Omarchy passwords and administrator access

The current mockup follows the user's selected Omarchy policy: encryption is
always on, with the account password reused for disk unlock. There is no separate
disk passphrase. Turning “Use as sudo password” off requires a separate
administrator password in that same checkbox row, with no extra confirmation field; it must never silently mean passwordless sudo. Turning
it back on clears the separate administrator-password inputs. Review describes
the relationship without showing any password value.

Account and administrator passwords have no Borea-imposed minimum/maximum length,
complexity rule, or punctuation/Unicode allowlist. Require a nonempty value and
matching confirmation for the account password (the inline sudo password has no repeat field), preserve whitespace exactly, and reject NUL because the
intended Linux authentication interfaces use C strings. The HTML inputs are
single-line controls; that transport behavior is not a universal Linux character
policy. Wi-Fi's protocol-specific rules are separate from account passwords.

Native integration still needs per-target verification: there is no universal
PAM policy across all families, and no backend may silently truncate a password.
The upstream [passwd manual](https://github.com/shadow-maint/shadow/blob/master/man/passwd.1.xml)
documents PAM/complexity checks. The upstream
[sudoers manual](https://github.com/sudo-project/sudo/blob/main/docs/sudoers.mdoc.in)
states that sudo normally authenticates the invoking user. A separate admin
password therefore requires an explicit supported administrator-authentication
configuration (for example, a deliberately configured root-password policy),
not just a second password field. This HTML implements the interaction only.

## Port acceptance, for the next implementation task

- Render Slint and the HTML at equal viewport size, scale, and bundled fonts;
  compare side by side at boot fill, readiness, dot/arrow morph, idle tug, corner
  landing, network states, and each selected family. Capture deterministic animation
  phases rather than comparing random wind positions or arbitrary timer samples.
- Exercise early/late/duplicate readiness, retry, double activation, resize,
  reduced motion, keyboard-only navigation, and long translated labels.
- Drive every network row above with deterministic fixtures, including disconnect
  during the Ethernet countdown and late events after switching networks.
- Test 16:9, 16:10, 4:3, 21:9, 32:9 and display scaling; verify corner controls,
  focus, text contrast and layout. Low-opacity nonselected cards must stay legible;
  adjust accessibility shortcomings explicitly rather than silently copying them.
- Label fixtures/dry runs as simulations. Visual agreement does not establish
  networking, GPU support, disk safety, installation, or ISO boot readiness.
