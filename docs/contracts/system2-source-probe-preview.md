# Offline source-field preview for a System-2 comparison

18 October 2026, issue #22; source-only work after Alpha 14. This prepares the
small field comparison in the [UP 258D12 goal](../goals/up258-programming-2026-10-18.md).
It does not read or program a device and is not the missing complete download.
[ADR-0132](../adr/0132-offline-source-images-do-not-grant-download-admission.md)
continues to separate source interpretation from executable admission.

## Existing path reused

ProductDB's byte-order probe reuses the production value resolver, Dynamic
activation, numeric validation, placement, overlap checks and deterministic
ordering. Its new `preview_system2_probe` accepts one to eight explicit
parameter references and returns a distinct `SourceProbePreview`, not a live
`ProbePlan`. Existing System-2 tool-metadata cross-checks must succeed. Unknown,
inactive, ambiguous, symmetric multioctet, out-of-segment or out-of-16-bit-address
fields refuse the requested preview instead of silently disappearing. The last
one-octet address `FFFFh` remains representable. Unselected candidates remain
listed; evaluation diagnostics and uninterpreted legacy-patch presence survive.
No product-specific implementation branch, dependency or database migration.

App prepares the request for exactly one project device using the same mapping
as a download. CLI previews return before constructing an async runtime or a
network client. `--preview` also works on the existing live-probe family; it
makes the gateway optional and never sends even when one is supplied.

```sh
knx device byte-order-probe <area.line.device> \
  --project <project.knxdb> --product-db <products.sqlite> \
  --preview --system2-source \
  --parameter-ref M-0001_A-9205-04-9270_P-903_R-903 \
  --parameter-ref M-0001_A-9205-04-9270_P-900_R-900
```

The project supplies the values; this command does not alter them. For the
pinned public Siemens package and the previously defined A/B requests, the
preview places P903/R903 at `3CF8h` (16 bits, values 300 or 600) and P900/R900
at `3CFAh` (control 0). R904 is not selected/active. Eleven existing evaluation
diagnostics remain visible. These are **source-derived candidate locations**,
not observed runtime locations or proof of every parameter's activation.

The output labels big/little-endian byte sequences as candidates, never device
bytes. No recovery file, image or support certificate is produced. The ZIP
identity export remains format 1, identity-only, with memory `not-requested`.

## Safety boundary

`--system2-source` requires `--preview`; it cannot become a live override.
`--confirm-read`, key, force and restart flags are not accepted. The existing
live probe still uses `check_program_kind` and refuses `MV-0025`. The System-2
image still refuses `LegacyPatchAlways`. No source-to-property executor or
hardware scope is enabled by this change.

A proposed first write-back of identical bytes is **not** a no-op proof or
proof of the complete download path. A read-only session cannot write; an
ordinary authorised connection can write Verify Mode before the requested
memory operation, including on reconnection. Any narrower test therefore needs
its own admitted target/range, defined setup lifecycle and durable recovery
before that first setup mutation. It must not inherit download authorisation or
call memory-only backup complete allocator recovery. The reporter must approve
any actual read or write separately.

## Verification

Synthetic source tests cover explicit selection, eight-field acceptance and
nine-field refusal, missing/duplicate fields, metadata mismatch, symmetric
values, address boundaries and unchanged image/live-probe refusal. The existing
digest-pinned public-package test now checks both real A/B previews and retained
evaluation diagnostics. Built-CLI tests use synthetic project/product files:
preview succeeds without sending to a supplied loopback gateway; a live source
override refuses before opening the project or gateway. No physical validation
is claimed.

## Remaining dependencies

The new preview makes the exact comparison reproducible locally; it does not
resolve the [production decision](../research/2026-10-16-up258-production-decision.md).
Full image construction still needs legacy patch/invisible-parameter and encoding
semantics; full download additionally needs source control/resource transforms
and complete recoverability. No memory acquisition or write-test release can be
claimed from the identity-only ZIP.
