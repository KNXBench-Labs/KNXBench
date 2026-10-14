# Stored bus access and IP configuration — Goal B3

ETS projects can store how ETS reached the bus (`BusAccess`, per line) and a
device's IP settings (`IPConfig`). KNXBench keeps both verbatim on import and,
since Goal B3, shows them **read-only**. This contract says what is shown, what
stays hidden, and what KNXBench never does with them. Evidence:
[research](../research/project-format.md#update-2026-10-14-busaccess-and-ipconfig-goal-b3).

## Never

- No connection, discovery, scan or bus request to a stored address.
- No stored endpoint becomes a gateway, a default or a suggestion; the gateway
  settings are not read or written by this view.
- No write to the project, its stored bytes or the product database.
- No value in the session log, a debug report or an error message.

## What is shown

`GET /api/project/stored-bus-access` lists every stored record: the opaque rows
of kind `RetainedElement` named `BusAccess` or `IPConfig`, in stored order.

- **Owner.** The last `Line[@Id='…']` or `DeviceInstance[@Id='…']` step of the
  stored xpath names the source element. It is shown as that line or device only
  when exactly one line or device of the project has that source id. Otherwise
  the record is listed as not assigned, with the reason: the stored row names no
  id (`notKeyed`), no element has it (`noMatch`), or several do
  (`severalMatches`). The importer keys `BusAccess` by its line; `IPConfig` is an
  element it does not model and keys by element names only, so an `IPConfig` is
  never assigned to a device.
- **Provenance.** Source file, stored xpath, byte size and SHA-256 of every record.
- **Fields.** Every attribute in document order, marked when the schema does not
  name it. `BusAccess/@Parameter` is split into its `key=value` parts only when
  every non-empty `;`-separated part has exactly one `=` and an identifier key;
  otherwise it stays one value and the record says `parameterNotSplit`. Child
  elements are kept and named, not read.
- **Refusals.** A record over 64 KiB (`tooLarge`) or not one well-formed element
  (`notWellFormed`) is listed with its provenance and no fields.

## Hidden until asked

Every value is hidden, with its class (empty, IPv4 address, MAC address, GUID,
number, text) and length, except enumeration-like settings that cannot name an
endpoint: `BusAccess` parameters `Type`, `ProtocolType`, `UseNat`, `NAT` and
`IPConfig/@Assign`, and only while the value is a single word of letters
(optionally single-quoted). Names (`@Name`, `Name=`) are hidden: they can be host
names.

`POST /api/project/stored-bus-access/reveal` with `{ "recordId" }` answers that
one record with all of its values. A record id is `<position>:<first 16 hex digits
of the stored SHA-256>`; when it no longer matches (another project, changed
rows), the answer is `404` with kind `storedBusAccessChanged` and the web reads
the list again. The web keeps revealed values only in the open record; **Hide
values** forgets them without a request.

## Surfaces

The command palette's **Stored bus access…** opens every record. The line
inspector shows that line's records and nothing at all when there are none.

## Not covered

- Assigning an `IPConfig` to its device: the stored row does not say which.
- Interpreting `Edi` drivers or the connection string beyond the split above, or
  turning any of it into a KNXBench connection.
- Editing, exporting or deleting stored records.
