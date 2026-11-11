# WP-36 — Communication-object view: links first, compact toolbar, direction pre-fill (#44, #46, #47)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **open.**

## Requested outcome

1. **#46:** linked group addresses first and visible by default; other object
   details collapsed; a compact link-a-group-address row.
2. **#44:** the `ComObjectTable` toolbar (three view buttons, reset, original
   order, priority-column toggle) becomes one view dropdown plus a small
   options menu; search and filters stay directly reachable.
3. **#47:** pre-fill the link direction from the object, not the group-address
   name (owner decision 11 November 2026, “what makes more sense”): an object
   with an existing sending link defaults to receive (one sending address per
   object, see `knx-core` `group_tables.rs`); Transmit without Write defaults
   to send; Write without Transmit defaults to receive; otherwise send. Always
   editable. Applies to `Inspector.tsx` and `GroupAddressMemberAdd.tsx`.
