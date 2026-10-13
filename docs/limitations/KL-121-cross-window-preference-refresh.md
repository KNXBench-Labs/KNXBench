# Cross-window preference refresh

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-121)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 121. Two open windows do not see each other's preference changes until one reloads

**2026-10-02 UI-owner implementation update.** The historical reload-only
description below is superseded at frontend scope: authenticated main and
companion windows now reread the authoritative settings record every five
seconds while visible and on focus/visibility. Reads never write it back.
Queued/failed local patches, unknown keys and deletion intent are preserved;
write-generation and cancellation guards prevent stale reads from overriding
new edits or reviving a stopped timer. Focused regressions, realistic negative
controls and the complete offline gates pass. There is still no instantaneous
push or general shared-project synchronization; native/AT workflows are not
accepted by these tests. Publication remains pending. See
[UI_ALPHA_READINESS](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/UI_ALPHA_READINESS.md). The historical title and inventory
identity remain stable for incoming links and the separate alpha owner.

**Historical description before this follow-up:**

**Limitation.** Since [ADR-0029](../adr/0029-application-settings-file.md),
preferences live in one `settings.json` in the server's data directory and
every window reads it at load. A window that changes a preference writes it
to the file; a second window already open — a browser tab, the diagnostics
companion, the desktop shell next to a browser — keeps showing the value it
read when it loaded, until it reloads or something else makes it re-read.
Changing the theme in one window does not repaint the other.

**Cause.** There is no push channel from the server for this, and no
polling. `apps/knx-web/src/settingsStore.ts` reads the record once, from
`initSettings()`, and every later change it hears about is one it made
itself. The one thing that *is* protected is key loss: `PUT /api/settings`
is a patch and `AppState::settings_lock` serializes the read-modify-write,
so the second window's next write cannot flatten the first window's change
back to what it last read. Only the second window's *view* goes stale, and
only of a key the other window touched.

**Impact.** Small and self-correcting. The `localStorage` arrangement this
replaced had the same staleness within one browser (a `storage` event would
have fixed it and none was listened for) and a worse version across front
ends, where the two never agreed at all. The window that is stale is by
definition not the one the user is changing preferences in.

**Lifted when.** There is a reason to add a notification channel. A
`storage` event on the cache key would fix the same-browser case cheaply and
would still leave the browser-versus-desktop case open, which is the case
worth solving; both wait for a server-side change feed, which nothing else
needs yet.
<!-- END retained catalogue record -->
