# WP-11: current-source native capture export — 9 November 2026

Scope: one bounded slice of the 8 November acceptance target (bus-monitor capture export) on the **current-source native build**, in an actual isolated Wayland/WebKitGTK window at 80/100/150% app zoom. A disposable loopback KNXnet/IP peer delivers one fictional frame to the real desktop backend. Not whole-WP-11 or KL-130 acceptance; no ledger status changes. The [measurement](../evidence/wp11-current-native-capture-2026-11-09.json) holds full digests, exact fields and exceptions.

## What was bound

- Built source `0b182054a`, UI `0.1.0-alpha.20`; admission revision `4c427fc0d`. The tracked diff between them is documentation only (status documents, evidence records, one open-work edit); source, lockfiles, configuration and assets are identical. The retained build (149 files, full digests in the JSON) was authenticated from its archive and reused. **No fresh build, no new UI-stamp claim.**
- Peer: the 27 October disposable peer, unchanged bytes, served the current TunnelClient unmodified. Fixture literal cEMI `29 00 BC E0 11 01 08 01 01 00 81` matches the `group_write_indication` test helper (`cemi.rs`, identical at both revisions). An independent hand decode gives fictional `1.1.1` → raw `2049` (1/0/1), GroupValueWrite, short value 1, low priority, hop count 6, not repeated. Software witness only, not normative KNX or hardware evidence.
- Isolation: desktop and peer shared one fresh loopback-only network namespace (only `lo`, loopback routes only), zero effective capabilities, nonzero UID, masked user-data/mount trees, private HOME/XDG/results. Both processes were inspected live **before** any connect. Exceptions: direct GTK chooser (`GTK_USE_PORTAL=0`), in-memory GSettings, shared host Wayland/session-bus/AT-SPI (the per-application AT-SPI socket was reported unreachable, yet the tree was readable over the shared bus), software-render flags. GVFS also fell back to the shared host session bus; direct filesystem masking does not establish host-service isolation. No portal/GPU/full-sandbox claim. Initial interface discovery was refused under the fence and left alone; the gateway was entered by hand as `127.0.0.1`.

## Session and fixture delivery

Native Connect reached the peer (one CONNECT exchange, session 1, UDP, assigned address 1.1.1). Before emission the UI showed "Connected to 127.0.0.1:3671" and "No telegrams yet." (the no-emission control). Only then the peer was asked to emit; it sent one tunnelling request, received ACK `06100421000a04210000`, answered four heartbeat roundtrips and the ordinary Disconnect. The native row shows source `1.1.1`, destination `2049`, GroupValueWrite, low, hop count 6, not repeated, `0x01 (6-bit)`; its backend row-insertion timestamp falls about 27 ms after the trigger wall-clock record. This is not packet-receive latency, an echo or RTT measurement; the peer monotonic clock was not calibrated to that clock. The peer log, UI row and saved file are three separate observations.

## Executed per zoom

Zoom observed by persisted `uiScale` (default/0.8/1.5) and AT-SPI geometry of Export capture (width 119 / 94 / 177 px). Export capture was offscreen at 150% and brought into view with AT-SPI scroll-to before a fresh SHOWING guard. Per zoom, with the native chooser/alert scope and an independent file oracle:

1. Export → Save File chooser → **Cancel**: chooser gone, results directory unchanged, no capture file.
2. Export → name `cap-<zoom>` set through AT-SPI EditableText (not physical typing) → **Save**: file parses as `knxbench-bus-monitor` v1, session 1, active, losses 0/0, exactly the one row above.
3. Export → same name → Save → overwrite question (names the file and its folder) → **Cancel**; then chooser **Cancel**: bytes and mtime preserved.
4. Export → same name → Save → **Replace**: only `exportedAt` differs, rows and provenance identical, mtime advanced. Original and replaced bytes are retained separately.

Across all six writes only `exportedAt` differs, and it strictly increases. One native session served all three zooms.

## Teardown

Ordinary Disconnect answered by the peer, ordinary window Close; desktop, peer and launcher exited 0, no owned process or window remained, private runtime directory removed. Active-window (none), per-output workspace/focus and cursor position matched the pre-run state; this is a limited predicate, not a full inventory or other-application restoration claim.

## Counts

81 recorded native step receipts (setup, trees and settlement included), all exit 0; 4 owned-window crops as corroboration. These are steps, not independent tests. No failed native-step receipt occurred. Three reader-development failures (alias/index, nested settings, offscreen geometry) and missing-file inspections remain retained; the corrected reader consumed unchanged observations without native replay. No product bug was observed and no product source changed.

## Review and scope deviation

Independent read-only review confirmed the native cells and file/protocol chain.
The worker also wrote a diagnostic timestamp outside its authorized task scratch.
Prior existence and content were not recorded. The controller retained a hashed
copy of the current regular file and left the original untouched; deletion or
restoration that requires unknown prior state is deferred. This is an execution
scope deviation, not expanded consent or a successful workspace-confinement claim.
The initial review BLOCK and the controller disposition remain private evidence.
The historical handoff also miscounted four added docs as three; source comparison
establishes four additions plus one modified open-work page.

## Not claimed

Whole WP-11, hardware/firmware/ETS/normative compatibility, NAT/RouteBack, reconnect or general protocol conformance, portal/GPU, Orca/Firefox, other layouts and keyboard cells, name-field suffix behavior (extensionless names). Raw receipts, trees, saved bytes and the independent reader stay in private evidence.
