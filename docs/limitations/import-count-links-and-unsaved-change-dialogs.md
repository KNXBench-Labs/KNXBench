# Import-count links and unsaved-change dialogs

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-import-count-links-and-unsaved-change-dialogs)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Project-status import links and unsaved-changes dialogs (2026-10-12)

The Overview's **Show in log** links open the existing session log narrowed to
`import:*` entries of the chosen severity; there is no separate stored report
view. That log lives only for the current server run and keeps at most 1000
entries, so after a restart, a native `.knxdb` open (no historical report) or a
very large import it can hold fewer findings than counted. The log then states
both numbers instead of pretending they match. A zero count is deliberately not a
link. The desktop **Quit** dialog shares the component, labels and protection
tested here, but was not exercised in a native WebKitGTK window; browser evidence
covers the identical Open/Import and New-project variants.
<!-- END retained catalogue record -->
