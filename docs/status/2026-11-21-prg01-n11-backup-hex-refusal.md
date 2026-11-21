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

Gate: change-scoped package gate, all stages green except rust-tests. Failures
there: the known pre-existing `c49_partial_group_addresses` and five
`http_parameter_grid`/`http_parameter_captions` tests (untouched code), plus
`tcp_connection_refusal_never_falls_back_and_udp_controls_are_handshake_only`,
which panicked with an I/O `WouldBlock` in its test harness under full-suite
load and passes alone (`cargo test -p knx-cli --test it`, 1 passed). Classified
as a load flake in untouched code; no claim that the gate is fully green.
