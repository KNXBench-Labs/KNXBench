# English offline community demos

**Published on `main` (2026-10-09); not a release, not linked from the website yet.** The 1.0.0 package READMEs still say "local review candidate": the packages are frozen and checksummed, so their text was not changed.

| Project | Devices | Group addresses | Building-space nodes | Lines (including main) |
| --- | ---: | ---: | ---: | ---: |
| Single-Family Home | 32 | 105 | 22 | 2 |
| Multi-Unit Residential Building | 101 | 339 | 60 | 8 |
| Office Building | 157 | 507 | 62 | 4 |

The home has two living floors, a basement/technical area and garage/outside. The residential project has six apartments on three living floors plus shared areas. The office has three floors with offices, meeting rooms, corridors and auxiliary rooms. Device totals include illustrative couplers and the IP interface, not power supplies or an electrical design.

## Downloads

- [All three demos, shared catalogue and guides](1.0.0/knxbench-community-demos-1.0.0.zip)
- [Single-Family Home](1.0.0/single-family-home-1.0.0.zip)
- [Multi-Unit Residential Building](1.0.0/multi-unit-residential-1.0.0.zip)
- [Office Building](1.0.0/office-building-1.0.0.zip)
- [Checksums](1.0.0/SHA256SUMS) and [manifest](1.0.0/manifest.json)

Each individual ZIP includes its native `.knxdb`, the same original fictional `.knxprod`, an English tour/five exercises, source descriptors/generator and the unchanged project licence. The combined ZIP includes all three. No original vendor data, firmware or ETS project export.

## First use

1. Extract a package. Retain an untouched copy and work on a duplicate: native projects can auto-save edits.
2. **Product catalog → Install product database**: install `fictional-demo-devices.knxprod` once. Do not replace your product database. A byte-identical re-import is deduplicated.
3. **File → Open (.knxdb)…**: open the selected native file; the web picker can upload a file into its own server data area.
4. Follow the project's English guide: trace a connection, edit a device description, edit a supported parameter, link a reserve channel to a new group, save as/reopen.

Device names are not editable in the current UI, so the exercise edits Description. A newly created practice group has no declared DPT and infers 1.001 from its linked object. Its intentionally unfinished receiver-only state belongs to the exercise, not to the clean baseline.

## Verified boundary

Real production frontend + real isolated server in Chromium; clean product catalogue; 15 UI exercises and all 290 devices' programme/parameter resolution. Full original-model save/load and full edited-model equality are separately verified. Tested application source: `33db32e94b7281787331c7bb0ebf3c8a1c4043ab`; native schema 10, product database 22, synthetic product input scheme 11. Server reports `0.1.0-alpha.2+g33db32e9`; frontend identifies as `0.1.0-alpha.5`. These differing existing labels are not a compatibility claim about a released AppImage.

No ETS admission/XSD validation, native WebKitGTK/Orca, real-device download, simulation, Secure or electrical/filter-table/isolation proof. Never connect these fictitious applications to a real device download. Existing startup discovery failed in the loopback-only namespace (expected HTTP 502); it could not reach hardware.

[Engineering contract and reproducible verification](../docs/COMMUNITY_DEMO_PROJECTS.md). Public repository/knxbench.com linking awaits the owner's separate publication go.
