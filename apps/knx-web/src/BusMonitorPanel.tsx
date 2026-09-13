/** Bus monitor panel: starts or attaches a session, polls telegrams, and filters/renders them. */
// apps/knx-web/src/BusMonitorPanel.tsx
import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import type { BusMonitorStopResponse, BusTelegramRow } from "./api";
import BusComposeForm, { type ComposeResolution } from "./BusComposeForm";
import {
  type ContextLock,
  forgetSessionContext,
  readContextLock,
  recordSessionContext,
  subscribeContextChanges,
} from "./busContext";
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

  // Whether the project state behind the attached session still matches the
  // project as it is now — see `busContext.ts` for why this cannot be
  // answered by asking the server. Re-read on every poll tick, on every
  // cross-window signal and whenever this window regains focus, so a stale
  // verdict never waits for a remount.
  const [contextLock, setContextLock] = useState<ContextLock>("synced");
  // The id of a session that replaced the one this panel was attached to
  // (someone disconnected and reconnected, in this window or another).
  // Connection state moving under the panel is one of the four things the
  // lock must be explicit about, so it gets its own notice rather than
  // being folded quietly into the session line.
  const [replacedBy, setReplacedBy] = useState<number | null>(null);
  // Set when a poll comes back `404`: the session this panel was attached
  // to was stopped somewhere else. Without this the panel would show a
  // permanent poll error for a session that ended perfectly normally.
  const [endedElsewhere, setEndedElsewhere] = useState(false);

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

  const [selectedSequence, setSelectedSequence] = useState<number | null>(null);
  const selectedTelegram = rows.find((row) => row.seq === selectedSequence);

  function selectRow(row: BusTelegramRow) {
    setSelectedSequence(row.seq);
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

  // Mirrors `session` for the listeners registered once on mount below,
  // which fire long after the render that created their closure and must
  // see the current value rather than the one captured at mount.
  // Mirrors `session` for the listeners registered once on mount below,
  // which fire long after the render that created their closure and must
  // see the current value rather than the one captured at mount. Updated
  // synchronously by `attachTo` rather than by an effect: `connect()`
  // publishes a session record in the same tick it adopts the session, and
  // that publication notifies this window's own listener immediately — an
  // effect-updated mirror would still read `null` there and fire a
  // redundant reattach request against the session just started.
  const sessionRef = useRef<AttachedSession | null>(null);

  function attachTo(next: AttachedSession | null) {
    sessionRef.current = next;
    setSession(next);
  }

  /// Attach to whatever session the server already has, if any. Shared by
  /// the mount effect and the signal effect below; `isCancelled` lets the
  /// mount effect discard a reply that lands after unmount.
  async function reattach(isCancelled: () => boolean): Promise<void> {
    try {
      const response = await api.pollBusTelegrams(0);
      if (isCancelled()) return;
      sinceRef.current = response.nextSince;
      setRows(response.telegrams);
      setDroppedBefore(response.droppedBefore);
      setStatus(response.status);
      setEndedElsewhere(false);
      setReplacedBy(null);
      setContextLock(readContextLock(response.sessionId));
      skipNextImmediatePollRef.current = true;
      attachTo({ sessionId: response.sessionId, assignedAddress: null });
    } catch (e) {
      if (isCancelled()) return;
      if (api.errorStatus(e) === 404) return; // no session — Connect form, as before.
      // Anything else (network error, 500, …) is not silently
      // swallowed either, even though it leaves the same Connect-form
      // state a 404 would: the user can still see what went wrong.
      setConnectError(api.errorMessage(e));
    }
  }

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
    void reattach(() => cancelled);
    return () => {
      cancelled = true;
    };
    // Deliberately empty deps — this runs once per mount, matching the
    // brief's "on mount, ask GET /telegrams" (not on every session change;
    // `connect()`/`disconnect()` manage `session` themselves afterwards).
  }, []);

  // The same question, asked again when something suggests the answer may
  // have changed: another window published a session record, or this window
  // regained focus after time spent elsewhere. Only ever asked when this
  // panel holds no session — a panel that already has one learns about
  // changes from its own poll loop, and must not fire a second request per
  // signal.
  //
  // Why this exists at all: a companion window showing the Connect form
  // while the main window is already connected would invite exactly the
  // second session the one-session model forbids. The server would refuse
  // it with a `409` (`bus_routes.rs:95-109`), so nothing breaks — but an
  // error message is a worse answer than the running session's telegrams.
  //
  // Cross-window `storage` delivery is a platform courtesy, not a
  // guarantee, which is why the `focus` listener is here too: a user who
  // clicks into the companion has already given it the one signal no
  // platform withholds.
  useEffect(() => {
    function onSignal() {
      const current = sessionRef.current;
      if (current === null) {
        void reattach(() => false);
        return;
      }
      setContextLock(readContextLock(current.sessionId));
    }
    const unsubscribe = subscribeContextChanges(onSignal);
    window.addEventListener("focus", onSignal);
    return () => {
      unsubscribe();
      window.removeEventListener("focus", onSignal);
    };
  }, []);

  useEffect(() => {
    if (!session) return;
    let cancelled = false;

    const attached = session;

    async function poll() {
      try {
        const response = await api.pollBusTelegrams(sinceRef.current);
        if (cancelled) return;
        // Connection state moved under this panel: `/telegrams` is
        // answering for a *different* session than the one these rows and
        // this cursor belong to (`bus_routes.rs:272-311` returns the live
        // session's `sessionId` on every poll, and `AppState` holds at most
        // one). Somebody disconnected and reconnected — in this window's
        // sibling, or in another client entirely. Keeping the old rows
        // would mix two sessions' traffic in one table under one sequence
        // column, and keeping the old cursor would index the new session's
        // buffer with the old one's position. Both are dropped, the change
        // is announced, and the effect re-runs against the new identity —
        // which re-reads the lock, because the new session froze its own
        // `GroupAddressContext` at its own moment.
        if (response.sessionId !== attached.sessionId) {
          sinceRef.current = 0;
          setRows([]);
          setSelectedSequence(null);
          setNewRowThreshold(null);
          setDroppedBefore(0);
          setPollError(null);
          setReplacedBy(response.sessionId);
          setContextLock(readContextLock(response.sessionId));
          setStatus(response.status);
          attachTo({ sessionId: response.sessionId, assignedAddress: null });
          return;
        }
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
        // Cheap (one synchronous `localStorage` read) and unconditional, so
        // the verdict never depends on a cross-window event this platform
        // may or may not deliver.
        setContextLock(readContextLock(response.sessionId));
      } catch (e) {
        if (cancelled) return;
        if (api.errorStatus(e) === 404) {
          // The session was stopped somewhere else — the other window's
          // Disconnect, or another client's. That is an ordinary end, not
          // a failure, so this returns to the Connect form and says what
          // happened instead of showing a poll error forever.
          attachTo(null);
          setStatus(null);
          setEndedElsewhere(true);
          setPollError(null);
          setContextLock("synced");
          forgetSessionContext();
          return;
        }
        setPollError(api.errorMessage(e));
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
      setSelectedSequence(null);
      setNewRowThreshold(null);
      setDroppedBefore(0);
      setStatus("active");
      setStopSummary(null);
      setPollError(null);
      setEndedElsewhere(false);
      setReplacedBy(null);
      // Recorded *before* the session is adopted, so the first poll tick
      // already has something to compare against. This is the only moment
      // at which the project fingerprint the server froze can be captured
      // — `bus_routes.rs:120-123` takes its snapshot inside this very
      // request, and never mentions it again.
      const attached = { sessionId: started.sessionId, assignedAddress: started.assignedAddress };
      // The ref first, and before the record is published. Publishing
      // notifies this window's own listener synchronously; a listener that
      // still read `null` here would conclude this panel holds no session
      // and fire a reattach request against the session just started.
      sessionRef.current = attached;
      recordSessionContext(started.sessionId);
      setContextLock(readContextLock(started.sessionId));
      attachTo(attached);
    } catch (e) {
      setConnectError(api.errorMessage(e));
    }
  }

  async function disconnect() {
    try {
      const summary = await api.stopBusMonitor();
      attachTo(null);
      setStatus(null);
      setStopSummary(summary);
      setReplacedBy(null);
      setEndedElsewhere(false);
      setContextLock("synced");
      // The session is gone; the record describing it must go too, or the
      // next session would briefly look verified against its predecessor's
      // fingerprint.
      forgetSessionContext();
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
      {/* `.workspace-heading` like every other centre-pane view; the
          connect controls are this view's action cluster. The eyebrow
          states the only transport this panel has: `knx-server`'s bus
          layer is tunnelling-only, no discovery and no routing (see
          `apps/knx-server/src/bus.rs`'s module comment, D7).
          The placeholder is an RFC 5737 documentation address, not
          anybody's gateway. */}
      <header className="workspace-heading">
        <div>
          <p className="eyebrow">{t("busMonitor.eyebrow")}</p>
          <h1>{t("busMonitor.title")}</h1>
        </div>
        <div className="bus-monitor-connect">
          {/* The placeholder is the only visible hint at what this field is,
              and a placeholder is not a label; `aria-label` gives it a name
              that survives being filled in. The `title` on the disabled
              states is the reason, not decoration: a disabled control is out
              of the tab order, so the prose elsewhere on the page never
              reaches anyone arriving at it. */}
          <input
            type="text"
            placeholder="192.0.2.1:3671"
            aria-label={t("busMonitor.gatewayLabel")}
            value={gatewayInput}
            onChange={(e) => setGatewayInput(e.target.value)}
            disabled={!!session}
            title={session ? t("busMonitor.gatewayLocked") : undefined}
          />
          {session ? (
            <button onClick={disconnect}>{t("busMonitor.disconnect")}</button>
          ) : (
            <button
              onClick={connect}
              disabled={!gatewayInput}
              title={gatewayInput ? undefined : t("busMonitor.connectNeedsGateway")}
            >
              {t("busMonitor.connect")}
            </button>
          )}
        </div>
      </header>
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
      {endedElsewhere && (
        <p className="bus-monitor-ended-elsewhere" role="alert">
          {t("busMonitor.endedElsewhere")}
        </p>
      )}
      {session && replacedBy !== null && (
        <p className="bus-monitor-replaced-notice" role="alert">
          {t("busMonitor.sessionReplaced", { id: replacedBy })}
        </p>
      )}
      {/* The explicit stale lock. Not a hint, not a tooltip: a banner that
          names what moved and says plainly that the decoded column below is
          the old snapshot's answer. `role="alert"` because a user reading
          telegrams is looking at the table, not at the chrome. */}
      {session && contextLock === "stale" && (
        <p className="bus-monitor-stale-lock" role="alert">
          {t("busMonitor.contextStale")}
        </p>
      )}
      {session && contextLock === "unverified" && (
        <p className="bus-monitor-unverified-lock" role="note">
          {t("busMonitor.contextUnverified")}
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
          // Task 4: a write resolves its DPT from the same frozen snapshot
          // the decoded column is read through (`bus.rs:1135-1141`). If
          // that snapshot no longer describes the project, the DPT the
          // server would pick is the old project's answer — so the send
          // path locks on exactly the same condition the table does.
          contextStale={contextLock === "stale"}
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
          <div className="monitor-data">
          <div className="monitor-table-scroll">
          {/* The second half of the stale lock: the banner says it, and the
              table carries it, so a decoded value read out of context on a
              screenshot still shows it was not current. */}
          <table
            className={contextLock === "stale" ? "bus-monitor-table bus-monitor-stale-table" : "bus-monitor-table"}
          >
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
                    tabIndex={0}
                    aria-selected={selectedSequence === row.seq}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        selectRow(row);
                      } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                        e.preventDefault();
                        const sibling = e.key === "ArrowDown" ? e.currentTarget.nextElementSibling : e.currentTarget.previousElementSibling;
                        if (sibling instanceof HTMLElement) sibling.focus();
                      }
                    }}
                    onClick={() => selectRow(row)}
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
          </div>
          <aside className="telegram-details" aria-label={t("workbench.telegram")}>
            <h3>{t("workbench.telegram")}</h3>
            {selectedTelegram ? <dl>
              <dt>{t("busMonitor.column.time")}</dt><dd>{selectedTelegram.timestamp}</dd>
              <dt>{t("busMonitor.column.source")}</dt><dd className="mono">{selectedTelegram.source}</dd>
              <dt>{t("busMonitor.column.destination")}</dt><dd><span className="mono">{selectedTelegram.destination}</span>{selectedTelegram.destinationName && <p>{selectedTelegram.destinationName}</p>}</dd>
              <dt>{t("busMonitor.column.service")}</dt><dd>{selectedTelegram.service}</dd>
              <dt>{t("busMonitor.column.decoded")}</dt><dd>{decodedSummary(selectedTelegram)}{selectedTelegram.decoded?.error && <p className="field-error">{selectedTelegram.decoded.error}</p>}</dd>
              <dt>{t("busMonitor.column.payload")}</dt><dd className="mono">{selectedTelegram.rawPayload ?? "—"}</dd>
            </dl> : <p>{t("workbench.selectTelegram")}</p>}
          </aside>
          </div>
        ))}
    </div>
  );
}
