// apps/knx-web/src/BusMonitorPanel.tsx
import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import type { BusMonitorStartResponse, BusMonitorStopResponse, BusTelegramRow } from "./api";

// The four `ApplicationService` variant names design spec §5's checkbox
// filter enumerates. `bus.rs`'s synthetic `"SessionClosed"` marker (pushed
// when the gateway drops mid-session, §4.1 — see `push_closed_marker`'s
// own doc comment: "cannot be mistaken for real bus traffic by anything
// that later renders this row") is deliberately *not* one of these: a
// status marker explaining why telegrams stopped must stay visible
// regardless of which service checkboxes are ticked, or a user who has
// unchecked "Other" would silently lose the one row that tells them the
// session died.
const KNOWN_SERVICES = ["GroupValueRead", "GroupValueResponse", "GroupValueWrite", "Other"] as const;
type KnownService = (typeof KNOWN_SERVICES)[number];

function isKnownService(service: string): service is KnownService {
  return (KNOWN_SERVICES as readonly string[]).includes(service);
}

// Poll cadence for `GET /api/bus/monitor/telegrams`. Design spec §4.2's
// `Lagged(n)` accounting already guarantees a slow poller never loses a
// telegram *silently* — a lag just turns into a bigger `droppedBefore`
// jump the client is told about (§4.3) — so this constant is purely a
// UX/load trade-off, not a correctness one. 1 second is fast enough that a
// live session still feels live to someone watching the table, and slow
// enough that the one browser tab talking to the one session that can
// exist at a time (D3/D6) never amounts to meaningful request volume;
// nothing is gained by polling faster than a human notices a new row.
const POLL_INTERVAL_MS = 1000;

type ServiceFilters = Record<KnownService, boolean>;

function defaultServiceFilters(): ServiceFilters {
  return { GroupValueRead: true, GroupValueResponse: true, GroupValueWrite: true, Other: true };
}

function decodedSummary(row: BusTelegramRow): string {
  if (!row.decoded) return "—";
  return row.decoded.dpt ? `${row.decoded.dpt}: ${row.decoded.text}` : row.decoded.text;
}

/// Live view of `/api/bus/monitor/*` (design spec `docs/superpowers/specs/
/// 2026-09-11-group-monitor-design.md` §4). Needs no open project — same
/// reasoning as `LogPanel` (`KNOWN_LIMITATIONS.md` #36, part A):
/// monitoring reads straight off the gateway, so with no project open the
/// table still works, it just shows raw group addresses and no names
/// (`destinationName` comes back `null` — see `bus.rs`'s
/// `GroupAddressContext`).
///
/// This is not a claim of ETS Group Monitor parity, nor of anything
/// verified against real hardware — see the design spec's §5/§7.
///
/// Composing and sending a value (`POST /api/bus/write`) is a different
/// task's form; this panel only connects, watches, filters, and
/// disconnects.
export default function BusMonitorPanel() {
  const [gatewayInput, setGatewayInput] = useState("");
  const [session, setSession] = useState<BusMonitorStartResponse | null>(null);
  const [connectError, setConnectError] = useState<string | null>(null);
  const [pollError, setPollError] = useState<string | null>(null);
  const [stopSummary, setStopSummary] = useState<BusMonitorStopResponse | null>(null);

  const [rows, setRows] = useState<BusTelegramRow[]>([]);
  const [status, setStatus] = useState<"active" | "closed" | null>(null);
  const [droppedBefore, setDroppedBefore] = useState(0);

  const [textFilter, setTextFilter] = useState("");
  const [serviceFilters, setServiceFilters] = useState<ServiceFilters>(defaultServiceFilters);

  // Advanced by every poll response's `nextSince`. A `ref`, not state: the
  // next tick's poll must read the cursor synchronously as it fires, not
  // wait for a render that may not have happened yet (design spec §4.3:
  // "cursor advanced by nextSince").
  const sinceRef = useRef(0);

  useEffect(() => {
    if (!session) return;
    let cancelled = false;

    async function poll() {
      try {
        const response = await api.pollBusTelegrams(sinceRef.current);
        if (cancelled) return;
        sinceRef.current = response.nextSince;
        setRows((previous) => [...previous, ...response.telegrams]);
        setDroppedBefore(response.droppedBefore);
        setStatus(response.status);
        setPollError(null);
      } catch (e) {
        if (!cancelled) setPollError(api.errorMessage(e));
      }
    }

    void poll(); // first poll immediately, not after the first interval tick
    const id = setInterval(() => void poll(), POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [session]);

  async function connect() {
    setConnectError(null);
    try {
      const started = await api.startBusMonitor(gatewayInput);
      sinceRef.current = 0;
      setRows([]);
      setDroppedBefore(0);
      setStatus("active");
      setStopSummary(null);
      setPollError(null);
      setSession(started);
    } catch (e) {
      setConnectError(api.errorMessage(e));
    }
  }

  async function disconnect() {
    try {
      const summary = await api.stopBusMonitor();
      setSession(null);
      setStatus(null);
      setStopSummary(summary);
    } catch (e) {
      setConnectError(api.errorMessage(e));
    }
  }

  function toggleServiceFilter(service: KnownService) {
    setServiceFilters((filters) => ({ ...filters, [service]: !filters[service] }));
  }

  // Client-side only, over already-fetched rows (design spec §5, `[R]`
  // ruling) — this never re-fetches and never touches `sinceRef`/the
  // server cursor, so a filtered-out row still advances the buffer
  // position correctly; switching a filter back on reveals rows already
  // held, not a permanently narrowed view.
  const visibleRows = useMemo(() => {
    const query = textFilter.trim().toLowerCase();
    return rows.filter((row) => {
      if (isKnownService(row.service) && !serviceFilters[row.service]) return false;
      if (!query) return true;
      const destination = row.destination.toLowerCase();
      const name = (row.destinationName ?? "").toLowerCase();
      return destination.includes(query) || name.includes(query);
    });
  }, [rows, textFilter, serviceFilters]);

  return (
    <div className="bus-monitor-panel">
      <h2>Bus monitor</h2>
      <div className="bus-monitor-connect">
        <input
          type="text"
          placeholder="192.168.1.10:3671"
          value={gatewayInput}
          onChange={(e) => setGatewayInput(e.target.value)}
          disabled={!!session}
        />
        {session ? (
          <button onClick={disconnect}>Disconnect</button>
        ) : (
          <button onClick={connect} disabled={!gatewayInput}>
            Connect
          </button>
        )}
      </div>
      {connectError && <span className="field-error">{connectError}</span>}
      {session && (
        <p className="bus-monitor-session">
          Session {session.sessionId} — assigned address {session.assignedAddress}
          {status === "closed" && " — closed by gateway"}
        </p>
      )}
      {stopSummary && (
        <p className="bus-monitor-stop-summary">
          Stopped session {stopSummary.sessionId}: {stopSummary.telegramCount} telegram(s) seen,{" "}
          {stopSummary.droppedCount} dropped.
          {stopSummary.warning && <span className="bus-monitor-warning"> {stopSummary.warning}</span>}
        </p>
      )}
      {pollError && <span className="field-error">{pollError}</span>}
      {/* The user-facing half of "never lose a telegram silently" (§4.2's
          `Lagged(n)`/ring-buffer eviction accounting): whenever the
          buffer's running `droppedBefore` counter is above zero, this
          banner says so and by how many, and it updates every time the
          counter grows on a later poll — it is never folded quietly into
          the row count. */}
      {droppedBefore > 0 && (
        <p className="bus-monitor-gap-notice" role="alert">
          {droppedBefore} telegram(s) could not be kept (buffer capacity or a slow poller) and are
          missing from this view.
        </p>
      )}
      {session && (
        <div className="bus-monitor-filters">
          <input
            type="text"
            placeholder="Filter by destination or name…"
            value={textFilter}
            onChange={(e) => setTextFilter(e.target.value)}
          />
          {KNOWN_SERVICES.map((service) => (
            <label key={service} className="bus-monitor-filter">
              <input
                type="checkbox"
                checked={serviceFilters[service]}
                onChange={() => toggleServiceFilter(service)}
              />
              {service}
            </label>
          ))}
        </div>
      )}
      {session &&
        (rows.length === 0 ? (
          <p className="bus-monitor-empty">No telegrams yet.</p>
        ) : visibleRows.length === 0 ? (
          <p className="bus-monitor-empty">No telegrams match the current filters.</p>
        ) : (
          <table className="bus-monitor-table">
            <thead>
              <tr>
                <th>Seq</th>
                <th>Time</th>
                <th>Source</th>
                <th>Destination</th>
                <th>Service</th>
                <th>Payload</th>
                <th>Decoded</th>
              </tr>
            </thead>
            <tbody>
              {visibleRows.map((row) => (
                <tr
                  key={row.seq}
                  className={row.service === "SessionClosed" ? "bus-monitor-row-marker" : undefined}
                >
                  <td>{row.seq}</td>
                  <td>{row.timestamp}</td>
                  <td>{row.source}</td>
                  <td>
                    {row.destination}
                    {row.destinationName && (
                      <span className="bus-monitor-dest-name"> ({row.destinationName})</span>
                    )}
                  </td>
                  <td>{row.service}</td>
                  <td>{row.rawPayload ?? "—"}</td>
                  <td className={row.decoded ? `bus-monitor-decoded-${row.decoded.kind}` : undefined}>
                    {decodedSummary(row)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ))}
    </div>
  );
}
