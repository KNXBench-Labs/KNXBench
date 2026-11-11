# WP-36 — Communication-object view: links first, compact toolbar, direction pre-fill (#44, #46, #47)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **#47 implemented 11 November 2026** (`linkDirectionDefault.ts`, both link
forms, reason shown under the choice). Review changed the planned flag rule:
the first link always sends (ETS Schema23 `@Links`, read responses, legacy FEh),
later links receive, an existing link on the address is kept.
**#44 and #46 implemented 11 November 2026** (`Inspector.tsx` `ComObjectEditor`,
`ComObjectTable.tsx` toolbar): an expanded object lists its linked group
addresses and the link row first, under a small **Linked group addresses**
heading; DPT, function, description, flags and priorities sit in a native
**Object settings** disclosure, closed by default, that keeps drafts when closed
and stays open while a change is saved. The toolbar has one **Object view**
select and an **Options** menu (**Original order**, **Priority column**
checkbox; Escape closes it). Deviation from the #44 triage reply: **Reset
filters** stays a direct toolbar button next to the filters it resets, rather
than moving into Options.

## Requested outcome

1. **#46:** linked group addresses first and visible by default; other object
   details collapsed; a compact link-a-group-address row.
2. **#44:** the `ComObjectTable` toolbar (three view buttons, reset, original
   order, priority-column toggle) becomes one view dropdown plus a small
   options menu; search and filters stay directly reachable.
3. **#47:** pre-fill the link direction from the object, not the group-address
   name (owner decision 11 November 2026, “what makes more sense”): the first
   link sends, an object with a sending link defaults to receive (one sending
   address per object, see `knx-core` `group_tables.rs`). The flag rule first
   planned here was dropped in review: a Write-only object still answers reads
   on its sending address. Always editable. Applies to `Inspector.tsx` and
   `GroupAddressMemberAdd.tsx`.
