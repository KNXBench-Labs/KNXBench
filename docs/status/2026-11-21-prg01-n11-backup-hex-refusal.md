# PRG-01-N11 acceptance — 21 November 2026

Bounded refusal hardening of the device-backup file reader
(`crates/knx-app/src/device_backup.rs`). Integer parsing accepts a leading `+`,
so a damaged backup field such as region octets `+5`, address `+400` or mask
`+701` was read as a number instead of refused. The reader now requires every
character of octet, address, mask and manufacturer text to be an ASCII hex
digit; such files are `Malformed`, never guessed, and are not rewritten.
The write side already emits canonical uppercase hex, so valid files are
unchanged (all existing backup tests pass).

Pin: `signed_or_padded_hex_is_malformed_never_read_as_a_number` — seen RED
against the previous parser, GREEN with the fix. File format, version numbers,
CLI/HTTP/history/report/web consumers are unchanged (the refusal reuses the
existing `Malformed` error path). Simulator/offline only; no bus, device, VM or
hardware contact; not hardware acceptance. PRG-01 stays open.
