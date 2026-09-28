# U1 — independent ADR-0038 review (2026-09-28)

## Evidence and verdict

- Reviewed the primary source PDFs directly: Project Schema23 v01.00.00 §1.1.2.3 enumerates `Ground`, §1.2.6.3 names it as a normal root `Space`, and §1.2.6.4 permits nested `Space`s. KNX IoT Information Model §1.2.3.5 defines `loc:Site`; KNX IoT 3rd Party API Table 10 maps it to the ETS Buildings root, *not* to a `Ground` token. ADR-0038 keeps this distinction explicit.
- Cross-checked `knx-etsproj/tests/site_hierarchy.rs`: a synthetic schema-23 `Ground` with two child buildings and one shared line imports, and undocumented `Site` is reported rather than aliased. `knx-store/src/project.rs::a_ground_site_over_two_buildings_on_one_line_round_trips` tests persistence; `knx-core/src/command.rs::a_ground_site_holds_two_buildings_and_a_device_moves_between_them` tests command-path references.
- KNOWN_LIMITATIONS §127 accurately identifies the absence of a real ETS export with `Ground`, and first-installation-only command editing. This decision does not claim ETS UI semantics.
- Verdict: **accept with the stated evidence bounds; no blocking contradiction**. The user explicitly accepted ADR-0038 on 2026-09-28. Status changed to Accepted; no domain/schema/UI change.
- ISSUE-06 plan: first two research/decision rows covered by ADR and named tests, conditional migration/type row is not applicable for the accepted no-new-type decision. Site UI row remains open for U12/ISSUE-05.

No KNX traffic and no hardware write. The web lock remains released.
