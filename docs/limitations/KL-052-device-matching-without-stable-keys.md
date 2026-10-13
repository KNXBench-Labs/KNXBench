# Device matching without stable keys

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-052)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 52. Project diff cannot correlate a device with no individual address and no matching `ets_id`

**Update 2026-10-10 (case observed).** The zero corpus count no longer holds.
The privately supplied schema-23 project has a device in
`Topology/UnassignedDevices` with no individual address and no serial number,
so only its ETS id can correlate it. Whether `Puid` or another attribute stays
stable across re-exports is unverified (§54), so no fallback key is
introduced. The ranking argument below is withdrawn; behaviour is unchanged.

**Limitation.** A device's natural key
(`docs/superpowers/specs/2026-09-10-project-diff-design.md` §3.4) is its
individual `address`, and only when `Some`. A device with no individual
address relies entirely on an `ets_id` match; if that also fails to line
up between the two projects being compared, `diff_projects` cannot
correlate the two at all — the device surfaces as an unrelated `removed`
on one side and `added` on the other, never as a match with field
changes.

**Cause.** A deliberate scope decision (design spec §3.4, §9): there is
no stronger identity to fall back on. Guessing would risk a false match
between two genuinely different devices, which CLAUDE.md's
never-silently-discard/never-guess posture rules out.

**Impact.** Two saves that differ only in, say, a description edit on an
address-less device can be reported as one device removed and a
different device added, obscuring what was actually a single edit.

**Measured 2026-09-23 (T15).** All three local corpus projects contain
zero devices lacking both an address and an ETS id. The matcher regression
also establishes that two empty ETS ids are *not* an id match: they remain
unrelated unless a real natural key resolves them.

The aggregate-only local measurement is intentionally ignored in ordinary CI.
With the gitignored `OriginalData/` corpus present, reproduce it with
`cargo test -p knx-app --test diff_correlation_measurement -- --ignored --nocapture`.

**Lifted when.** Open. No stronger per-device identity exists in the
domain model today. The zero corpus count ranks this below observed gaps;
it does not justify inventing a fallback or claiming the case impossible.
<!-- END retained catalogue record -->
