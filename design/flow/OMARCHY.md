# Omarchy functional baseline

Accepted baseline: the current [HTML flow](borea-flow.html), including Trim
Omarchy, and the shared visual rules in [LOCKED.md](LOCKED.md). Recorded
2026-11-06. This is the behavior to implement, not a working native installer.
The HTML remains an interactive simulation; the live system and worker described
in [SYSTEM.md](../../docs/architecture/SYSTEM.md) do not exist yet.

The accepted HTML snapshot has SHA-256
`427421509c7c0febf09c5765c86a09bfe4609613cdca7be7d30b1291268240db`.
This identifies this baseline; later approved HTML changes should update this
contract and the recorded fingerprint together.

## Scope and flow

The first complete target is **Arch family → Omarchy**. Keep the other family
and flavour choices visible as designed, but do not present their adapters as
implemented. Extras and manual partitioning are deferred. The old manual
partitioner notes are not part of this implementation baseline.

| Step | Working behavior required |
| --- | --- |
| Boot | Run the locked highlight loop until real service readiness. Only then run the dot-to-arrow transformation and enable Continue. Animation alone cannot signal readiness. |
| Get online | Discover connections; distinguish scanning, no devices/networks, cable absent, credentials needed, connecting, authentication failure, address/DNS failure, captive portal and usable internet. Preserve entered choices on retry. Only usable connectivity enables the online gate. |
| Family / flavour | Select Arch and Omarchy. Apply Arch colour to the logo and forward action. Changing family/flavour invalidates incompatible choices; silently carrying an Omarchy removal plan to another target is forbidden. |
| Trim Omarchy | Select extras to remove, individually or by group. Keep everything clears all removals. Preserve selections and the update-persistence preference across normal navigation and overview edits. See the trim contract below. |
| Disk | Offer **Replace** or **Install alongside** only. Show target identity and what will be erased or preserved. Alongside requires a supported existing OS and a verified safe resize/layout plan. Show allocation, mandatory encryption and optional snapshots. Selecting options does not write disks. |
| You | Collect account, hostname, time zone, language and keyboard. The account password is also the Omarchy encryption password. Keep the agreed inline sudo-password control and validation. |
| One last look | Resolve and show the complete plan, including trim and update behavior. Edit routes return directly here. Require confirmation of the current plan before Install. |
| Install | Execute the confirmed plan without routine wizard prompts. The boot-style wordmark uses a fixed family colour and real progress. Tab switches to plain output and back. Errors remain actionable and cannot become success. |
| Complete / restart | Only verified success enables the exact boot dot-to-arrow animation and idle nudge. Use the boot logo's text layers, font and shadows. Print exactly **After restarting, remove the usb**. The arrow means Restart; there is no Done summary or separate USB-removal page. |

Fit each window using the locked reflow rules. Titles/subtitles sweep in after
the logo settles. Keyboard navigation and edits must retain state regardless of
animation, resizing or pagination. Language/keyboard settings live on You;
there is no permanent top-right control in the accepted flow.

## One plan, independent of presentation

The UI edits a draft. The core validates and resolves it into a versioned plan;
the worker executes one confirmed revision. These are proposed contracts, not
existing Rust types or a settled IPC schema.

| Plan section | Required information |
| --- | --- |
| Target | Arch / Omarchy identifiers; pinned upstream installer revision; adapter and trim-catalog revisions |
| Trim | Stable IDs of selected removals; `keep_removed_after_updates`, default true; resolved removal actions and any compatibility failures |
| Storage | Stable disk identity and capacity; Replace/Alongside; existing-OS identity when applicable; resolved allocation, preservation/destruction and EFI/boot changes |
| Protection | Encryption required; account-secret reference for disk unlock; snapshots preference and supported configuration |
| Account | Name, username, hostname; account-secret reference; sudo policy and separate secret reference if selected |
| Locale | Time zone, language and keyboard identifiers |
| Confirmation | Plan revision/digest, validation result and explicit user confirmation bound to that exact revision |

Passwords are runtime secrets, not ordinary serializable plan fields. Never put
them in plan exports, summaries, logs or command-line arguments. A worker request
must reference a validated plan, not carry arbitrary shell commands from the UI.

Any installation-affecting edit invalidates confirmation and resolution as
needed. Before the first write, recheck target disk identity, current partition
state, available space, required connectivity and adapter compatibility. Changes
to those preconditions return to an actionable plan error, not silent execution
of a stale plan.

## Trim contract

A marked item means **remove this extra after Omarchy installs it**. Unmarked
means keep it. Do not reinterpret this as a custom Arch install or silently skip
parts of Omarchy's own installation. If upstream later supports a verified
selection interface, adopting it requires an explicit adapter/contract revision.

The mockup catalogue contains 32 display labels:

| Group | Baseline labels |
| --- | --- |
| Apps | Chromium, LibreOffice, Obsidian, Kdenlive, OBS Studio, Pinta, Xournal++, Signal, Spotify, Typora, LocalSend, 1Password |
| Web apps | HEY, Basecamp, WhatsApp, YouTube, X, Zoom, Discord, ChatGPT, Grok, GitHub, Figma, Google Photos, Google Maps, Google Messages, Google Contacts, Fizzy |
| Developer tools | Docker, lazydocker, Claude Code, opencode |

These labels are design fixtures, not verified upstream package names or proof
that an Omarchy release includes each item. The native catalogue must be pinned
to a supported upstream revision and map stable item IDs to verified actions.
Display names must never become package-manager arguments.

Each catalogue entry needs its group/label, upstream availability, owned package
IDs and/or launcher/config assets, removal method, dependencies and protected
resources, verification method, and supported update-persistence mechanism.
Treat web apps as their actual upstream objects; removing a launcher must not
accidentally uninstall the shared browser. Do not assume Docker and lazydocker,
or an AI tool and its runtime, can be removed independently without checking.

Selection behavior:

- Individual toggles update the selected set and all counts immediately.
- Remove all selects every removable entry in that group. Keep all clears it.
  Mixed groups retain correct item/count state. Other groups are unaffected.
- Keep everything clears the selected set. It does not silently change the
  separate keep-removed-after-updates preference.
- Group operations are atomic model updates. Staggered strike animations only
  display the result; they cannot schedule future changes to the install plan.
- Rapid clicks, navigation, resize and overview edits cannot leave pending model
  mutations. Leaving Trim reads the final selected set immediately.
- The summary shows the complete selection or a count with an edit path to it.
  Keep-everything and keep-removed-after-updates have distinct meanings.

Execution behavior:

1. Before Review, resolve every selected ID against the pinned catalogue and
   Omarchy revision. Identify conflicts, shared dependencies and protected
   components. Never expand a removal to unselected apps without explaining and
   re-reviewing the resulting plan.
2. Run Omarchy through a verified unattended integration that respects Borea's
   single review, encryption/account choices and target layout.
3. Apply the resolved trim actions inside the installed target. Do not remove
   apps from the live environment or another existing OS/user installation.
4. Verify the selected extras are absent and retained/protected components still
   satisfy the resolved plan. Record nonsecret per-item results.
5. When keep-removed-after-updates is on, configure a supported, narrowly scoped
   mechanism and verify it. Off means remove now without promising to prevent
   reintroduction by future Omarchy updates.
6. A requested removal or persistence failure blocks success. Show the item,
   reason, current installation state and supported recovery. Never silently
   ignore selected items or install an unreviewed policy change.

Already-absent resources should be a verified no-op. Retry must use per-action
state and idempotent operations; rerunning a removal must not broaden its scope.
Do not use an unrestricted dependency purge or edit upstream update scripts by
assumption. If upstream has no supported way to honour a selected behavior,
report it before confirmation and resolve the integration decision.

## Storage, account and execution details

Replace erases the reviewed target. Alongside preserves the supported existing OS
and its boot path while allocating space for Omarchy. The fixture's NVMe/Windows
and empty SATA disks, 80–500 GB slider bounds and capacity figures are not hardware
rules. Real limits come from detected geometry, filesystem/BitLocker state,
verified shrink support and actual installation requirements. Never offer
Alongside solely because a disk has the same display name as the sample.

Encryption stays on for Omarchy. Snapshots are optional and do not replace
backups. Reject an incompatible resolved layout before Review. The supported
filesystem, EFI layout and upstream encryption handoff must be established by
the adapter rather than copied from the deferred manual-partitioner fixture.

Require a nonempty matching account password and reject NUL, without arbitrary
length, complexity or punctuation rules. Apply supported credential transport
without shell interpolation. The sudo checkbox defaults on. Turning it off
replaces its words with the separate sudo-password input in the same row;
turning it back on clears that separate secret. A separate password must use a
verified authentication policy: ordinary sudo normally authenticates the invoking
user, so storing another password alone cannot implement this option.

Suggested worker phase boundaries, corresponding to the current presentation:

1. Validate the confirmed plan and preflight the target.
2. Prepare storage and encryption.
3. Install the Arch foundation and Omarchy using the selected adapter.
4. Apply account/locale, trim, update-persistence and snapshot choices in the
   adapter's verified dependency order.
5. Verify storage/boot configuration, required target components and all selected
   customization results; finalize and flush the target before success.

The real order must follow the supported upstream installer; these boundaries
are not evidence that arbitrary hooks already exist. Emit phase, progress
availability, per-action status and recoverable/fatal errors. Unknown-duration
work must remain truthful; the HTML's timer is not a real progress algorithm.

Retry a failed download without repeating completed destructive storage work.
Cancellation before writes can return to the draft; after writes it must explain
what completed and which recovery is supported. Do not promise generic rollback.
No second upstream wizard may replace or bypass Borea's reviewed plan.

## Acceptance checks for the working implementation

- Real readiness/network events gate Continue; success and failure states are
  distinguishable and retry does not invent a connection.
- Selecting Arch → Omarchy reaches Trim and carries the selected family colour.
- Individual, group and keep-everything trim actions produce exact selected sets
  and counts, including rapid group clicks followed immediately by Next/Review.
- All 32 baseline labels have reviewed mappings or explicit unsupported status;
  protected/shared resources cannot be removed by incidental dependency cleanup.
- Review edits preserve choices, return directly to One last look and invalidate
  confirmation. No animation can mutate an already confirmed plan.
- Replace and Alongside resolve against real disks; absent/changed targets or
  unsupported shrink conditions prevent writes.
- Encryption uses the account password; snapshot and supported sudo choices are
  applied and verified. Secrets never appear in exported plans or output.
- Trim changes only the target and removes exactly the reviewed extras. Repeating
  a completed action is safe; an absent item is handled explicitly.
- Persistence on survives a representative supported Omarchy update. Persistence
  off adds no Borea removal-enforcement policy. Unsupported persistence is caught
  before install, not silently downgraded.
- Failure injection at download, partitioning, Omarchy, trim, persistence and final
  verification produces actionable errors and never enables the restart arrow.
- Cover/output toggling and resizing preserve execution; real progress and the
  final verified state come from worker events, not animation timers.
- Success reuses the exact boot wordmark/arrow and prints
  **After restarting, remove the usb**. There is no separate USB page. The mockup
  only replays boot; native restart must use the verified shutdown/reboot path.

Validate state transitions in core tests, adapter behavior in disposable target
images/VMs, and boot, encryption unlock and OS preservation on supported hardware.
Alongside preservation requires an actual reboot into both systems, not only a
successful partitioning command.

## Known gaps before native implementation

The HTML already demonstrates selection/navigation, draft validation, review,
progress presentation and restart animation. Its group trim currently staggers
model changes with animation timers; the atomic-update requirement above must
replace that in the native model. Its catalogue, disks, progress and retry are
fixtures. It does not install Omarchy, remove software, enforce update policy,
apply locale settings, provision credentials or reboot hardware.

The upstream Omarchy revision, unattended handoff, package/asset ownership,
protected dependency set, update-persistence integration, Alongside support and
separate sudo-password policy still need verification. See
[DECISIONS.md](../../docs/architecture/DECISIONS.md). These gaps do not change the
accepted UI baseline and must not be represented as already working features.
