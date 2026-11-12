# Last-download object evidence

The [ADR-0174](../adr/0174-last-downloaded-links-and-flags.md) backend (WP-37f)
records effective object flags with Parameters and links with Group addresses,
with independent usability and settlement times. The backend is on main; the
communication-object UI remains pending.

- Evidence describes what a completed regular/batch request built its image
  from, not a device readback. Per-part pending flags remain authoritative.
- Native20 parameter values and recorded-empty baselines survive migration;
  their flags remain unknown and their links absent until their own part is
  successfully downloaded. Read-on-init is null, not a written false.
- Modules remain outside these baselines. Current comparison refusals do not
  fix the full-download duplicate-lookup overwrite or the missing `070n`
  object-table-outside-application-segment guard.
- Synthetic storage, settlement and comparison tests do not establish physical
  device behavior, hardware compatibility or UI/browser acceptance.
