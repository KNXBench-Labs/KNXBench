# KNXBench v0.1.0-alpha.13

Alpha 13 packages the work since Alpha 12: a more orderly workbench, real
manufacturer-declared operating modes, and object lists that follow supported
parameter changes. The offline download foundations also grow stricter and more
capable. A better toolbox is not, by itself, a bus boarding pass.

## Workbench highlights

- **Fold navigation independently.** Project, engineering and diagnostics sections
  can be opened and closed separately; the explorer keeps its own scroll area,
  including on narrow screens.
- **A grouped File menu.** Project/file actions, imports, exports and support tools
  have clear groups instead of one long queue of commands.
- **Actual device operating modes.** Supported manufacturer-declared enum
  parameters appear beside the relevant communication objects. This edits real
  parameter values, not a free-text Function label. The shared catalogue path is
  manufacturer-independent; it is not universal manufacturer-script support.
- **Parameter groups from the source.** Declared blocks and optional captions make
  the parameter list easier to navigate; unsupported or absent captions do not
  become write authority.
- **Missing objects follow supported edits.** Positively active, unscoped objects
  can be added in the same durable, undoable command as a parameter change.
  Existing IDs, links, overrides and order remain intact. Ambiguous identities or
  required new module-instance mappings refuse the whole edit rather than leave
  a half-updated project. [Editing contract](../contracts/parameter-object-refresh.md).

## Integrity fixes and offline engineering

- **No silent partial source images.** Absolute image readers refuse relative
  segments rather than omit them. Uninterpreted direct FixupList declarations,
  even empty ones, refuse across the existing image entries. Strict offline
  System-1/System-2 entries also refuse unresolved LegacyPatchAlways; established
  Easy3 source policy is unchanged, not newly verified.
- **Explicit parameter bits and checksum polarity.** Bounded direct-Memory numeric
  fields within one octet use the existing writer without requiring equality to
  template defaults; bounds and corroboration guards remain. The absolute-property
  checksum-disable bit now follows its documented polarity. Neither correction
  admits a new device or completes a real-product download.
- **More offline building blocks.** Explicit legacy/System-B object and link-table
  codecs, a System-300 property-table adapter, retained relative-source metadata,
  bounded System-1/System-2 source-image APIs and normalized property-plan
  simulation have focused regression coverage. Source mapping, complete procedure
  translation and durable whole-mutation recovery remain separate work.
  [Expansion boundary](../contracts/offline-download-expansion.md).
- **Public UP258 evidence, not a programming fix**
  ([#22](https://github.com/KNXBench-Labs/KNXBench/issues/22),
  [PR #24](https://github.com/KNXBench-Labs/KNXBench/pull/24)). Digest-pinned public
  package tests and a documented production No-Go identify exact missing rules.
  `MV-0025` is still refused. [Decision](../research/2026-10-16-up258-production-decision.md).
- **Nothing new for ETS 5.7/schema 22 since Alpha 12**
  ([#7](https://github.com/KNXBench-Labs/KNXBench/issues/7)). Existing bounded
  schema-20 evidence and nested-submodule/schema-22 limitations are unchanged.

## Community and documentation

The public Hall of Fame now counts pull requests as well as issues, with explicit
XP sources, earned medals and one maintained credit comment per eligible PR.
Bonuses do not stack across reopen/close/merge. The bilingual website has a new
Phosphor Atlas presentation. Existing application stills remain labelled Alpha 12;
older recordings and the approved Evolution Story retain their provenance.

## Upgrade: independent backups first

**No schema bump relative to Alpha 12:** native/model schema remains **16** and
the dedicated product database remains **23**. This statement follows comparison
of the actual schema/model changes, not just the version numbers. New object
creation reuses the existing object model and durable command/history storage.
**Opening Alpha-13-saved files in Alpha 12 has not been verified.** Alpha 11 and
earlier refuse native16; retain independent pre-upgrade copies when returning.

Close KNXBench before copying `.knxdb` files and all Docker data, including the
product database. Keep original `.knxproj`/`.knxprod` sources. Update standalone
`knx-mcp` together with the application. In-file history is not an external backup.
The Docker port stays **8484**; review firewall/LAN/VPN exposure when using host
networking. Publishing an image does not replace any existing container.

## Still alpha — explicit boundaries

- **Not verified:** no new physical device/gateway operation, broad hardware
  support, native WebKitGTK/Orca feature acceptance or universal ETS compatibility
  follows from this release. Browser/native-evidence checks use isolated software
  servers and synthetic projects, not a live installation.
- Siemens System-2 / `MV-0025` programming remains refused. Offline codecs,
  simulator results and public source evidence are not a complete source image,
  executable download plan, whole-mutation recovery or target verification.
- New module-instance/repeat/number-offset construction and unsupported manufacturer
  scripts remain outside parameter-driven object creation. Whole-edit refusal
  preserves the project when these are required.
- No new `.knxproj` export, KNX Secure, individual-address programming, firmware
  replacement or little-endian download support is added. Colour/picture value
  codecs and specialized module-scope widgets remain outside this release.

## Distribution

Published on **16 October 2026** from frozen commit `6105976f`. All 59 ordered
source-gate stages pass with unchanged inputs: 4,724 Rust tests, 3,632 component
tests, 947 browser cases, 24 software-server native-evidence/edit cases,
152/152 conventional corpus tests and 16 explicit product-corpus tests.
The original Firefox cache setup failure and same-source continuation provenance
remain explicit in the [acceptance record](alpha13-acceptance.md).

The actual public AppImage/MCP downloads pass `SHA256SUMS` and provider digest
checks. The AppImage's headless Broadway embedded API/frontend starts with the
right version; this is not FUSE/EGL, native visual or Orca acceptance. Namespace
portal/FUSE warnings remain recorded; no picker/portal success is inferred.
The published Docker image passes isolated HTTPS/authentication, save/reopen and
an authentic synthetic native13 → 16 migration with retained device name and
three undo/one redo counts. Version/latest registry indexes match for amd64 and
arm64. All 36 served website/media files match the accepted release-mode build;
no new deployment was needed. [Publication record](../evidence/release-alpha13-2026-10-16.json).

Linux x86-64 AppImage and standalone `knx-mcp` with `SHA256SUMS`; Docker image
`knxbench/knxbench-server:0.1.0-alpha.13` for `linux/amd64` and `linux/arm64`.
`latest` follows published releases, including alphas, not moving source.
[Installation](../manual/getting-started/04-installation.md) ·
[Docker update](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md).
