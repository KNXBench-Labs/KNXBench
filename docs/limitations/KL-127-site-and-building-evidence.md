# Site and building evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-127)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 127. A site over several buildings rests on schema text and synthetic tests, not on an ETS sample

**Boundary.** [ADR-0038](../adr/0038-site-is-a-ground-root-space.md) represents a
site/property that groups several buildings on one KNX infrastructure as a
`Ground` space at the root of one installation's building structure, with
`Building` children. The only authority is *Project Schema23 v01.00.00*
§1.2.6.3 ("Space elements directly below Locations_t will nromally have Type
"Area" or "Building" or “Ground”"). None of the three reference projects
contains a `Ground` space, more than one root space, or more than one
installation (ADR-0038 E3). The import, native round-trip and command
behaviour is covered only by synthetic tests (`knx-etsproj/tests/site_hierarchy.rs`,
`knx-store` and `knx-core` unit tests). How ETS 5/6 displays or restricts a
`Ground` root is unverified.

An undocumented `Space/@Type` such as `Site` is **not** read as `Ground`. It
stays an `UnknownEnumValue` map problem with the existing reported
`BuildingPart` fallback (§89), and a test pins that.

The Buildings workspace now offers **Add site / property**, a fixed `Ground`
root in the first installation; existing buildings can move beneath it.
Mocked UI tests cover two buildings under one root without duplicated
devices. This does not supply independent ETS `Ground` export evidence or
make later installations editable.

Found on the way, and since addressed in the core and server by MODEL-01
(2026-10-04): `Command::RenameInstallation` renames an installation, and
commands act in the installation that owns the addressed entity instead of
`installations[0]`; root creates accept an explicit installation. The web UI
does not yet expose installation rename or later-installation creation.

**Alpha disposition (2026-10-04).** A renewed search found no `Ground`
sample in the corpus or in eight public xknxproject fixtures (RESEARCH §25).
On the user's instruction this stays a known gap and is closed for the Alpha
(`KL-127` → `ACCEPTED_BOUNDARY`); it is not evidence that ETS accepts the
ADR-0038 shape.

**Lifted when.** A real ETS export containing a `Ground` root (ideally with
several `Building` children, or several installations) is added to the
corpus and imports with the ADR-0038 shape. The installation-rename and
first-installation-only gaps are tracked separately, for ISSUE-05/B10
triage.
<!-- END retained catalogue record -->
