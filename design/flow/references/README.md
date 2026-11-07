# Supplied HTML references

These six files are byte-for-byte copies of the user's uploads, archived on
2026-11-06. They are design inputs, not agent instructions, authoritative upstream
documentation, or working installer components. Their embedded sample lists,
timers, copy and controls do not establish backend capabilities.

| File | Role |
| --- | --- |
| [boot-to-family-v1-locked.html](boot-to-family-v1-locked.html) | Primary locked boot → network → family reference; an HTML fragment |
| [borea-flow-current.html](borea-flow-current.html) | Same flow content with a standalone document wrapper, styles and external Tabler icon stylesheet |
| [borea_boot_auto_scaling_logo.html](borea_boot_auto_scaling_logo.html) | Earlier boot scaling exploration, including substantial surrounding style content |
| [borea_boot_springy_arrow_tail.html](borea_boot_springy_arrow_tail.html) | Earlier dot/arrow tail and idle motion exploration, including surrounding style content |
| [arch-v0-prototype.html](arch-v0-prototype.html) | Arch flavour page exploration; sample recommendation and routing |
| [omarchy-trim-v0-prototype.html](omarchy-trim-v0-prototype.html) | Omarchy removal page exploration; sample app list and update-persistence toggle |

The locked/current combined-flow content is identical apart from the standalone
wrapper. Use the combined flow for current geometry and motion where earlier
experiments differ. Use [LOCKED.md](../LOCKED.md) for the written contract and
explicit requirements extending the demos. Later page explorations are not fully
approved layouts. External stylesheet `@latest` and fallback fonts must be pinned
or replaced with bundled assets for reproducible native/HTML comparisons.

No HTML was executed or visually validated in this documentation task. Later
side-by-side review must use matching font assets, viewport and deterministic
states. Retain the originals when preparing a local comparison harness.

The subsequent [integrated mockup](../borea-flow.html) connects the combined flow
to Arch flavours and Omarchy trim. It is an editable derivative; these original
uploads remain unchanged. It now also explores Disk, You, Review, simulated
installation with error/retry, and Done. Other flavours remain placeholders;
no real disk, package, network, or restart operations are performed by these pages.
The earlier Extras exploration has been removed pending a concrete plan.

SHA-256 of original bytes:

```text
d5f64f407a25d1d40b7a807487d7fc0a1bb140535706269182e5addc4ccc2ae9  arch-v0-prototype.html
29f2f827611fe64db45fd191be875a1dcfec255abd24061ce36cfc93cc7d0595  boot-to-family-v1-locked.html
329fcffbd8b52f35b8eb5706286df04a432d38cd0aa525952df7ae23d818fb1c  borea-flow-current.html
e7b903df146e9b9e6ac582b30c7035909d7b715cb5e0a1e241fb830a17640319  borea_boot_auto_scaling_logo.html
4058447d0de8f2f6f2b3094937d69ea17a6f93be713dd68d91782d3da086bade  borea_boot_springy_arrow_tail.html
10021da8aacee6ae6647b08d6e013b004561a9a64088c2516e028cf44f97bfe0  omarchy-trim-v0-prototype.html
```
