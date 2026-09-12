// apps/knx-web/src/BusMonitorPanel.tsx
import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import type { BusMonitorStopResponse, BusTelegramRow } from "./api";
import BusComposeForm, { type ComposeResolution } from "./BusComposeForm";
import { useTranslate } from "./i18n";

// The identity of the one session this panel can ever be attached to
// (`AppState.bus_session` holds at most one — D3/D6). Deliberately not
// `BusMonitorStartResponse`: that DTO carries `assignedAddress`, which a
// *reattached* session (see the mount effect below) does not have — a
// `/telegrams` poll response never repeats the address `/start` returned
// once, so there is nothing honest to put there. `assignedAddress: null`
// means exactly "unknown, because this panel did not start the session
// itself," not "the tunnel has no address."
interface AttachedSession {
  sessionId: number;
  assignedAddress: string | null;
}

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

/// `DecodedValue::Conflict`'s wire text is `"conflicting DPTs: <names>"`
/// (`bus.rs`'s `format_dpt_list`, comma-joined `DptRef::to_string()`s) —
/// there is no separate names array on the wire (`DecodedValueDto` carries
/// only `kind`/`text` for the `conflict` case), so this strips the known
/// prefix to recover exactly the `{names}` the compose form's verbatim
/// message needs. Falls back to the whole text unchanged if the prefix
/// ever stops matching, rather than silently emitting an empty name list —
/// a fallback, not a case this is expected to hit.
const CONFLICT_TEXT_PREFIX = "conflicting DPTs: ";

function conflictNames(text: string): string {
  return text.startsWith(CONFLICT_TEXT_PREFIX) ? text.slice(CONFLICT_TEXT_PREFIX.length) : text;
}

/// Task 5, item 1/2: what the compose form should prefill from a clicked
/// row, mirroring `resolve_write_value`'s three-way outcome (design §6).
/// Only a `"value"`-decoded row hands the form a `dpt` at all; every other
/// `decoded.kind` (`"unresolved"`, `"conflict"`, `"error"`, or no `decoded`
/// at all) prefills a blank DPT field — `"error"` collapses into the same
/// `"none"` bucket as `"unresolved"` because `DecodedValueDto`'s `error`
/// case does not carry a `dpt` either (`bus_routes.rs`'s `DecodedValueDto`
/// `From` impl), so there is nothing more specific to tell the form.
function resolutionFromRow(row: BusTelegramRow): ComposeResolution {
  if (row.decoded?.kind === "value" && row.decoded.dpt) {
    return { kind: "single", dpt: row.decoded.dpt };
  }
  if (row.decoded?.kind === "conflict") {
    return { kind: "conflict", names: conflictNames(row.decoded.text) };
  }
  return { kind: "none" };
}

let nextComposeSeedKey = 1;

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
/// `projectOpen` is threaded from `App.tsx` (same `tree !== null` fact
/// `LogPanel` already receives as `tree`) purely so the compose form can
/// state up front that no project means no automatic DPT resolution,
/// rather than the user discovering that from a failed send.
export default function BusMonitorPanel({ projectOpen }: { projectOpen: boolean }) {
  const t = useTranslate();
  const [gatewayInput, setGatewayInput] = useState("");
  const [session, setSession] = useState<AttachedSession | null>(null);
  const [connectError, setConnectError] = useState<string | null>(null);
  const [pollError, setPollError] = useState<string | null>(null);
  const [stopSummary, setStopSummary] = useState<BusMonitorStopResponse | null>(null);

  const [rows, setRows] = useState<BusTelegramRow[]>([]);
  const [status, setStatus] = useState<"active" | "closed" | null>(null);
  const [droppedBefore, setDroppedBefore] = useState(0);

  // The lowest `seq` that counts as "arrived in the most recent incremental
  // poll" (design D34: an entry highlight the stylesheet renders, driven by
  // this threshold rather than by any per-row state — see `styles.css`'s
  // `.bus-monitor-row-new`). `null` means "nothing is new right now."
  //
  // Deliberately reset on *every* poll tick, including one that comes back
  // empty: the highlight is this row batch's for exactly one poll interval,
  // never longer. Marking nothing on an empty poll means a quiet bus does
  // not leave last poll's rows lit up forever, and it does not need a timer
  // to say so — the next tick already is the clock.
  //
  // Deliberately *not* touched by the mount-time reattach effect or by
  // `connect()`'s reset: adopting a running session's backlog, or starting
  // a fresh one, is not "these rows just arrived" — see both call sites.
  const [newRowThreshold, setNewRowThreshold] = useState<number | null>(null);

  const [textFilter, setTextFilter] = useState("");
  const [serviceFilters, setServiceFilters] = useState<ServiceFilters>(defaultServiceFilters);

  // What `BusComposeForm` is seeded with — `key` changes every time a row
  // is clicked so the form (a child component, task 5) fully remounts and
  // picks up the new `destination`/`resolution` as fresh initial state,
  // rather than this panel reaching into that component's internals to
  // overwrite fields the user may already be mid-edit on for an unrelated
  // send. Starts at a resting "nothing clicked yet" seed — the form still
  // renders and works from here (design §6: "the form still works" with no
  // row selected at all, not only with no project open).
  const [composeSeed, setComposeSeed] = useState<{
    key: number;
    destination: string;
    resolution: ComposeResolution;
  }>({ key: 0, destination: "", resolution: { kind: "unknown" } });

  function selectRow(row: BusTelegramRow) {
    setComposeSeed({
      key: nextComposeSeedKey++,
      destination: row.destination,
      resolution: resolutionFromRow(row),
    });
  }

  // Advanced by every poll response's `nextSince`. A `ref`, not state: the
  // next tick's poll must read the cursor synchronously as it fires, not
  // wait for a render that may not have happened yet (design spec §4.3:
  // "cursor advanced by nextSince").
  const sinceRef = useRef(0);

  // Set by the mount-time reattach effect right before it adopts an
  // existing session, so the *next* run of the polling effect below skips
  // its own "first poll immediately" call — the reattach poll below already
  // *was* that first poll (its rows/droppedBefore/cursor are already
  // applied), so polling again immediately would just be a wasted request
  // a beat before the interval would have fired anyway.
  const skipNextImmediatePollRef = useRef(false);

  // Task 5's second inherited fix: on mount, ask whether a session already
  // exists instead of assuming there is none. Before this, navigating away
  // from the panel (Log button, selecting an entity) left the server-side
  // session running — correct, the session is not owned by a React
  // component — but reopening the panel always showed the Connect form,
  // and pressing Connect hit a `409` with no way to clear it short of a
  // process restart. A `404` here means what it already means everywhere
  // else on this API (`bus_routes.rs`'s `poll_telegrams`): no session
  // exists, so the Connect form is exactly right, same as today. A `200`
  // means one does, and its `droppedBefore` is real — it must reach the
  // same gap notice a mid-session poll would trigger, not be discarded
  // just because this panel was not mounted when the gap happened (the
  // whole point of D3, applied here to the panel's own absence, not only
  // to a slow poller). A session already `"closed"` (the gateway dropped it
  // while nothing was mounted) still reattaches: its rows are worth
  // showing and `stopBusMonitor()` still ends it, same as if the panel had
  // stayed mounted the whole time — `status === "closed"` alone does not
  // gate anything in the render below, so this falls out for free.
  useEffect(() => {
    let cancelled = false;
    async function reattach() {
      try {
        const response = await api.pollBusTelegrams(0);
        if (cancelled) return;
        sinceRef.current = response.nextSince;
        setRows(response.telegrams);
        setDroppedBefore(response.droppedBefore);
        setStatus(response.status);
        skipNextImmediatePollRef.current = true;
        setSession({ sessionId: response.sessionId, assignedAddress: null });
      } catch (e) {
        if (cancelled) return;
        if (api.errorStatus(e) === 404) return; // no session — Connect form, as before.
        // Anything else (network error, 500, …) is not silently
        // swallowed either, even though it leaves the same Connect-form
        // state a 404 would: the user can still see what went wrong.
        setConnectError(api.errorMessage(e));
      }
    }
    void reattach();
    return () => {
      cancelled = true;
    };
    // Deliberately empty deps — this runs once per mount, matching the
    // brief's "on mount, ask GET /telegrams" (not on every session change;
    // `connect()`/`disconnect()` manage `session` themselves afterwards).
  }, []);

  useEffect(() => {
    if (!session) return;
    let cancelled = false;

    async function poll() {
      try {
        const response = await api.pollBusTelegrams(sinceRef.current);
        if (cancelled) return;
        sinceRef.current = response.nextSince;
        setRows((previous) => [...previous, ...response.telegrams]);
        // This tick's own batch only — never a running minimum kept across
        // ticks, or the marker would accumulate exactly the way it must not.
        setNewRowThreshold(
          response.telegrams.length > 0
            ? Math.min(...response.telegrams.map((t) => t.seq))
            : null,
        );
        setDroppedBefore(response.droppedBefore);
        setStatus(response.status);
        setPollError(null);
      } catch (e) {
        if (!cancelled) setPollError(api.errorMessage(e));
      }
    }

    if (skipNextImmediatePollRef.current) {
      skipNextImmediatePollRef.current = false;
    } else {
      void poll(); // first poll immediately, not after the first interval tick
    }
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
      setNewRowThreshold(null);
      setDroppedBefore(0);
      setStatus("active");
      setStopSummary(null);
      setPollError(null);
      setSession({ sessionId: started.sessionId, assignedAddress: started.assignedAddress });
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
      <h2>{t("busMonitor.title")}</h2>
      <div className="bus-monitor-connect">
        <input
          type="text"
          placeholder="192.168.1.10:3671"
          value={gatewayInput}
          onChange={(e) => setGatewayInput(e.target.value)}
          disabled={!!session}
        />
        {session ? (
          <button onClick={disconnect}>{t("busMonitor.disconnect")}</button>
        ) : (
          <button onClick={connect} disabled={!gatewayInput}>
            {t("busMonitor.connect")}
          </button>
        )}
      </div>
      {connectError && <span className="field-error">{connectError}</span>}
      {session && (
        <p className="bus-monitor-session">
          {t("busMonitor.session", { id: session.sessionId })}
          {session.assignedAddress && (
            <>{t("busMonitor.assignedAddress", { address: session.assignedAddress })}</>
          )}
          {status === "closed" && t("busMonitor.closedByGateway")}
        </p>
      )}
      {stopSummary && (
        <p className="bus-monitor-stop-summary">
          {t("busMonitor.stopSummary", {
            id: stopSummary.sessionId,
            count: stopSummary.telegramCount,
            dropped: stopSummary.droppedCount,
          })}
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
          {t("busMonitor.gapNotice", { count: droppedBefore })}
        </p>
      )}
      {session && (
        <BusComposeForm
          key={composeSeed.key}
          destination={composeSeed.destination}
          resolution={composeSeed.resolution}
          projectOpen={projectOpen}
          // Task 5 review, fix 2: `status` already covers both ways a
          // session can be closed while this panel shows it — the gateway
          // dropping it mid-poll, and the mount-time reattach effect above
          // adopting one that was already closed — so no separate tracking
          // is needed here.
          sessionClosed={status === "closed"}
        />
      )}
      {session && (
        <div className="bus-monitor-filters">
          <input
            type="text"
            placeholder={t("busMonitor.filterPlaceholder")}
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
          <p className="bus-monitor-empty">{t("busMonitor.emptyNoTelegrams")}</p>
        ) : visibleRows.length === 0 ? (
          <p className="bus-monitor-empty">{t("busMonitor.emptyFiltered")}</p>
        ) : (
          <table className="bus-monitor-table">
            <thead>
              <tr>
                <th>{t("busMonitor.column.seq")}</th>
                <th>{t("busMonitor.column.time")}</th>
                <th>{t("busMonitor.column.source")}</th>
                <th>{t("busMonitor.column.destination")}</th>
                <th>{t("busMonitor.column.service")}</th>
                <th>{t("busMonitor.column.payload")}</th>
                <th>{t("busMonitor.column.decoded")}</th>
              </tr>
            </thead>
            <tbody>
              {visibleRows.map((row) => {
                // Combined, not replaced (design D34): a `SessionClosed`
                // marker row that just arrived is both at once.
                const rowClasses = [
                  row.service === "SessionClosed" ? "bus-monitor-row-marker" : null,
                  newRowThreshold !== null && row.seq >= newRowThreshold ? "bus-monitor-row-new" : null,
                ]
                  .filter((c): c is string => c !== null)
                  .join(" ");
                return (
                  <tr
                    key={row.seq}
                    className={rowClasses || undefined}
                    onClick={() => selectRow(row)}
                    style={{ cursor: "pointer" }}
                    title={t("busMonitor.rowTitle")}
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
                );
              })}
            </tbody>
          </table>
        ))}
    </div>
  );
}
