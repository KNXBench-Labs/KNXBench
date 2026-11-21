# PRG-01-N12 acceptance — 21 November 2026

Consumer-consistency pin for the delivered customer-adjustable refusal and every
other download refusal category ([ADR-0187](../adr/0187-download-refusal-categories-have-a-drift-pin.md)).
`UnsupportedCategory::ALL` plus a knx-server test comparing the web reason and
action tables with the server's category codes. No runtime, history or report
change; no bus, device, VM or hardware contact. Negative check: removing one web
table entry made the test fail. PRG-01 stays open for the items in the N4–N6
scoping records.
