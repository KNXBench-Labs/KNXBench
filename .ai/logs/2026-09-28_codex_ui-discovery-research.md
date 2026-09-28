# U2 — AppImage discovery diagnosis and address-editor research (2026-09-28)

## Discovery: measured, not guessed

- Isolated worktree `ui-discovery-research` at `48cc48e`. `npm ci`, `npm run build`; `APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=1 cargo tauri build --bundles appimage --ci` succeeded. Artifact: `KNXBench_0.1.0-alpha.1_amd64.AppImage`, 106,936,824 bytes, executable ELF x86_64. A bounded graphical launch reached `POST /api/bus/discover` from the bundled WebView. GBM warnings occurred, but the route was reached.
- Built the unpackaged debug `knx-server` from the same worktree. The route returned HTTP 200, `interfaces: []` after 10.003 s.
- Downloaded a `strace` package to this track's private scratch (no system install or privilege change). Process syscall traces: each path made one successful 14-byte UDP `SEARCH_REQUEST` sendto to `224.0.23.12:3671`, same host LAN HPAI IPv4, different nonzero ephemeral ports; no UDP response was seen. Route to multicast and to the configured unicast gateway both select `eno1`.
- This is syscall evidence, **not** a wire capture: unprivileged `AF_PACKET` and `nft list ruleset` were denied. AppImage-only missing route/send is ruled out here; multicast path/firewall/gateway response remains unresolved. No package workaround/retry/sleep was invented. The manually entered endpoint remains available. RESEARCH §20.1 and KNOWN_LIMITATIONS §79 hold the durable, address-redacted evidence.
- No device write, tunnel, bus monitor or individual-address read. No contact with `1.1.220`.

## Line-relative address: direct PDFs

- Read Architecture v03.00.02 §3.1 p. 10 and Project Schema23 v01.00.00 pp. 40–43 directly; verified page footers. Area and line contain a `DeviceInstance` whose optional `Address` is only [0…255]. RESEARCH §20.2 separates documented facts, current-command behavior and unverified UX choice.
- `SetIndividualAddress` currently only checks duplicate addresses and `MoveDeviceToLine` leaves addresses untouched. U11 must add core validation and test it before exposing a device-octet-only field; do not silently re-address on a line move.

## Package boundary

- Documentation only in this package. ISSUE-12's live comparison and diagnosis rows have named evidence; implementation rows stay open for U10. ISSUE-09's address-editor implementation stays open for U11. Web lock belongs to commissioning K6 UI at this point; no web source was edited here.
