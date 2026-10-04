# ADR 0073: Imported elements keep their own ids; ambiguous references are not guessed

Date: 2026-10-04
Status: Accepted
Session: goal-ui owner, Alpha follow-up UA8 (found while checking ADR-0074)

## Context

`knx_etsproj::map` assigns internal ids in two passes. Pass 1 recorded one id
per ETS `@Id` in a `BTreeMap<String, Id>`. Pass 2 looked each element's own
id and every cross-reference up in that map. Two consequences were hidden
behind the validation error `DuplicateId`:

1. **Collapsed identity.** Two elements sharing an ETS `@Id` (two group
   addresses in `malformed_input.rs`'s fixture, or two devices, lines, …)
   received the **same** internal id. They survived in memory, but the
   native save upserts by id, so one of them vanished on save (now refused
   by ADR-0074). Devices sharing an id would also have collided in the
   `Devices` map; that case was not measured separately.
2. **Cross-installation links.** Schema ≥21 `ComObjectInstanceRef/@Links`
   uses short ids (`GA-1`) without an installation. The mapper derived one
   short-id table for the whole document, where the last full id won. A
   device in installation 0 could therefore be linked to installation 1's
   `GA-1`, silently. That is a link between two separate infrastructures,
   which ADR-0070 forbids for edits. Validation already checks short ids
   per installation (IMPORT_EXPORT).

## Decision

- Pass 1 records every allocation per ETS id in document order
  (`id_table::IdTable`). Pass 2 lets each element claim the next allocation
  for its ETS id, so every element keeps a distinct internal id. Id
  numbering for documents without repeats is unchanged.
- A cross-reference to an ETS id held by several elements resolves to
  nothing and is reported as `MapProblemDetail::AmbiguousReference { kind,
  target }` (report severity `Error`, like a dangling reference). The
  repeated `@Id` itself is still reported by validation as `DuplicateId`.
- Schema ≥21 short ids are resolved against the device's **own
  installation's** group addresses. A short id repeated inside one
  installation is ambiguous.

## Consequences

- An ETS project with a repeated `@Id` imports with every element intact and
  can be saved natively. The references that cannot be attributed are listed
  in the import report instead of being attached to an arbitrary element.
- The corpus has one installation per project and no repeated ids (measured
  2026-10-04 on all three `.knxproj` files). Its import output is unchanged.
  Whether ETS repeats `GA-<n>` across installations in real exports is
  **unverified**: there is no multi-installation schema ≥21 sample.
- Tests: `crates/knx-etsproj/tests/malformed_input.rs`
  (`a_duplicate_group_address_id_keeps_two_internal_ids_and_does_not_guess_links`)
  and `crates/knx-etsproj/tests/links_installation_scope.rs`. Run against the
  previous mapper, the first one fails because both entries share one id, and
  the scope test fails with `[[None], [Some(4000)]]`: the device in
  installation 0 was linked to installation 1's address.
