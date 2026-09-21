/** Compose-and-send form for writing a value to a group address from the bus monitor. */
// apps/knx-web/src/BusComposeForm.tsx
//! Task 5's compose/send form (design spec `docs/superpowers/specs/
//! 2026-09-11-group-monitor-design.md` §6) — the other half of the loop
//! `BusMonitorPanel.tsx` opened: click a row, the destination and (when
//! known) DPT prefill here, edit, `POST /api/bus/write`.
//!
//! A sibling component, not folded into `BusMonitorPanel.tsx` — that file
//! already owns the session lifecycle, the poll loop and the filtered
//! table; this one owns exactly the compose/validate/send state machine,
//! which is a different lifetime (it survives across polls, resets only
//! when the parent hands it a new row to prefill from) and does not need
//! to know anything about sessions, polling or filters to do its job.

import { useState } from "react";
import * as api from "./api";
import type { BusWriteResponse } from "./api";
import { useTranslate } from "./i18n";
import { canonicalGroupAddress, useGroupAddressFormat } from "./gaNotation";

/// What the parent (`BusMonitorPanel.tsx`) knows about this destination's
/// DPT resolution *at the moment it was prefilled* — mirrors
/// `resolve_write_value`/`bus.rs`'s `GroupAddressDpt` three-way outcome,
/// plus a fourth case this component needs that the server-side type does
/// not: `"unknown"`, for a destination with no row behind it at all (typed
/// by hand, or edited away from whatever row it started as — see
/// `onDestinationChange` below). Only `"none"`/`"conflict"` are rejected
/// client-side (design spec §6, "rejected... before the request is even
/// sent"); `"unknown"` is deliberately *not* treated the same as `"none"` —
/// the client has no evidence either way for an address it never saw
/// decoded, so it defers to the server's own resolution/400 rather than
/// guessing "no DPT" for an address that might resolve perfectly well.
export type ComposeResolution =
  | { kind: "single"; dpt: string }
  | { kind: "none" }
  | { kind: "conflict"; names: string }
  | { kind: "unknown" };

interface BusComposeFormProps {
  /// The destination to seed the field with — a clicked row's
  /// `destination`, or `""` for the form's resting state (no row clicked
  /// yet). The parent remounts this component (via a changing `key`) every
  /// time a new row is clicked, so this is read once, as the initial
  /// value, not kept in sync afterwards — see `BusMonitorPanel.tsx`'s
  /// `composeSeed` state.
  destination: string;
  /// The resolution known for `destination` at prefill time. Same
  /// once-at-mount contract as `destination`.
  resolution: ComposeResolution;
  /// Whether a project is currently open, threaded from `App.tsx` (via
  /// `BusMonitorPanel`). With no project open, every resolution the panel
  /// could ever hand this form is `"none"`/`"unknown"` anyway (`bus.rs`'s
  /// `GroupAddressContext::decode` returns `"no project open"` for every
  /// row when `style` is `None`) — this prop exists only so the form can
  /// *say so* up front, per the design's "states this rather than silently
  /// disabling itself" rule, instead of making the user discover it by
  /// trying to send and reading a 400.
  projectOpen: boolean;
  /// Whether the session this form is attached to is already `"closed"` —
  /// either the gateway dropped it mid-session, or `BusMonitorPanel.tsx`
  /// reattached to one that was already closed before the panel ever
  /// mounted (Task 5's own reattach fix: rows from a closed session are
  /// still shown, correctly, as real data). Task 5 review, fix 2: a closed
  /// session has nothing left to send a `GroupValueWrite` through, so
  /// pressing Send here used to reach the server only to bounce off a `409`
  /// for a session everybody already knew was gone. Same "say so rather
  /// than silently disabling itself" rule as `projectOpen` above — the form
  /// disables and explains, it does not just grey out.
  sessionClosed: boolean;
  /// Whether the project state the session froze at `/start` no longer
  /// matches the project as it is now (`busContext.ts`). Same shape as
  /// `sessionClosed` above and for the same reason: the send path must be
  /// shut before the request, not after a puzzling reply. The difference is
  /// what would go wrong — a closed session bounces off a `409`, whereas a
  /// stale context succeeds, on the bus, with the previous project's DPT.
  /// `BusSession::resolve_write_dpt` reads the snapshot taken when the
  /// session started (`bus.rs:1015-1020`, `bus.rs:1135-1141`); nothing
  /// re-resolves it, and nothing tells the server the project moved. A
  /// telegram sent with the wrong DPT is not an error message, it is an
  /// actuator doing the wrong thing, and Undo does not reach the bus.
  contextStale: boolean;
}

// Task 5 review round 2: `SESSION_CLOSED_MESSAGE`/`NO_DPT_RESOLVED_MESSAGE`/
// `conflictingDptsMessage()` used to be module-level, English-only — the
// same freeze-at-import trap `commandRegistry.ts`'s `COMMANDS` had, except
// these never even called `t()` at all. All three are now built inside the
// component from `messages/en.ts`/`messages/de.ts` via `t()`, resolved
// fresh on every render.

export default function BusComposeForm({
  destination: initialDestination,
  resolution: initialResolution,
  projectOpen,
  sessionClosed,
  contextStale,
}: BusComposeFormProps) {
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  const [destination, setDestination] = useState(initialDestination);
  const [dpt, setDpt] = useState(initialResolution.kind === "single" ? initialResolution.dpt : "");
  const [value, setValue] = useState("");
  // The resolution this form still trusts for `destination` — starts as
  // whatever the parent prefilled, but is invalidated the moment the user
  // edits `destination` themselves (see `onDestinationChange`).
  const [resolution, setResolution] = useState<ComposeResolution>(initialResolution);
  const [sendError, setSendError] = useState<string | null>(null);
  const [sent, setSent] = useState<BusWriteResponse | null>(null);
  const [sending, setSending] = useState(false);

  function onDestinationChange(next: string) {
    setDestination(next);
    // A resolution only describes the address it was computed for. Once
    // the user types over the prefilled destination, that resolution no
    // longer applies to whatever is in the field now — there is no local
    // way to tell if the new address is `Single`/`None`/`Conflict` short of
    // asking the server, so this falls back to `"unknown"` rather than
    // keeping a stale answer for a different address.
    setResolution({ kind: "unknown" });
  }

  async function send() {
    setSendError(null);
    setSent(null);

    if (sessionClosed) {
      // Belt and braces: the Send button (and every field) is already
      // `disabled` below whenever `sessionClosed` is true, but guarding
      // here too means a session that closes between renders — the
      // gateway drops it, the next poll notices — can never reach
      // `api.writeBusValue` through a click that raced the re-render.
      setSendError(t("busCompose.sessionClosedMessage"));
      return; // Rejected client-side — `fetch` is never called.
    }

    if (contextStale) {
      // Same belt-and-braces reasoning as `sessionClosed` above, with more
      // at stake: the project can move between the render that disabled
      // this button and the click that raced it, and the request that
      // slipped through would be a real telegram on a real bus, resolved
      // against a project that no longer exists.
      setSendError(t("busCompose.contextStaleMessage"));
      return; // Rejected client-side — `fetch` is never called.
    }

    const explicitDpt = dpt.trim();
    let dptToSend: string | null;
    if (explicitDpt) {
      // An explicit DPT always wins (design §6, mirrors
      // `resolve_write_value`) — whatever the resolution says is moot.
      dptToSend = explicitDpt;
    } else {
      switch (resolution.kind) {
        case "single":
          dptToSend = resolution.dpt;
          break;
        case "none":
          setSendError(t("busCompose.noDptResolvedMessage"));
          return; // Rejected client-side — `fetch` is never called.
        case "conflict":
          setSendError(t("busCompose.conflictingDptsMessage", { names: resolution.names }));
          return; // Rejected client-side — `fetch` is never called.
        case "unknown":
          // No local basis to accept or reject — defer to the server's own
          // resolution against its session-cached DPT map; a `400` comes
          // back the same inline way as any other server error below.
          dptToSend = null;
          break;
      }
    }

    setSending(true);
    try {
      // The field accepts either notation; the wire only ever carries the
      // canonical `/` form (`gaNotation.ts`). Anything that is not a
      // level-style address goes through untouched, so the server's own
      // validation names what the user actually typed.
      const response = await api.writeBusValue(
        canonicalGroupAddress(destination),
        dptToSend,
        value,
        dptToSend === null ? null : api.defaultDptInputFormat(dptToSend),
      );
      setSent(response);
    } catch (e) {
      // `400`/`409`/`502` all land here — shown inline on the form, not
      // only as a toast (design §6: a `502` in particular is exactly the
      // kind of thing a KNX engineer needs to read in full).
      setSendError(api.errorMessage(e));
    } finally {
      setSending(false);
    }
  }

  /// The four controls below go `disabled` for reasons that are only
  /// rendered as prose a few lines above them. A disabled control is not in
  /// the tab order, so a screen reader only meets it while browsing the
  /// document, and then it needs to carry its own reason: hence
  /// `aria-describedby` pointing at whichever hint is currently mounted.
  /// Both can be mounted at once (a closed session whose project also moved),
  /// so this is a list, and it must never name an id that is not in the DOM.
  const disabledReason =
    [
      sessionClosed ? "bus-compose-closed-hint" : null,
      contextStale ? "bus-compose-stale-hint" : null,
    ]
      .filter((id): id is string => id !== null)
      .join(" ") || undefined;

  return (
    <div className="bus-compose-form">
      <h3>{t("busCompose.heading")}</h3>
      <p className="bus-compose-live-action">{t("busCompose.liveAction")}</p>
      {!projectOpen && <p className="bus-compose-hint">{t("busCompose.noProjectHint")}</p>}
      {sessionClosed && (
        <p id="bus-compose-closed-hint" className="bus-compose-hint bus-compose-closed-hint">
          {t("busCompose.sessionClosedMessage")}
        </p>
      )}
      {contextStale && (
        <p
          id="bus-compose-stale-hint"
          className="bus-compose-hint bus-compose-stale-hint"
          role="alert"
        >
          {t("busCompose.contextStaleMessage")}
        </p>
      )}
      <div className="bus-compose-fields">
        <label>
          {t("busCompose.destinationLabel")}
          <input
            type="text"
            className="bus-compose-destination ga-address"
            value={destination}
            placeholder={formatGa("1/1/1")}
            onChange={(e) => onDestinationChange(e.target.value)}
            disabled={sessionClosed || contextStale}
            aria-describedby={disabledReason}
          />
        </label>
        <label>
          {t("busCompose.dptLabel")}
          <input
            type="text"
            className="bus-compose-dpt"
            placeholder="DPST-1-1"
            value={dpt}
            onChange={(e) => setDpt(e.target.value)}
            disabled={sessionClosed || contextStale}
            aria-describedby={disabledReason}
          />
        </label>
        <label>
          {t("busCompose.valueLabel")}
          <input
            type="text"
            className="bus-compose-value"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            disabled={sessionClosed || contextStale}
            aria-describedby={disabledReason}
          />
        </label>
        <button
          onClick={() => void send()}
          disabled={sending || !destination || !value || sessionClosed || contextStale}
          aria-describedby={disabledReason}
        >
          {t("busCompose.send")}
        </button>
      </div>
      {sendError && <span className="field-error bus-compose-error">{sendError}</span>}
      {sent && (
        <p className="bus-compose-sent">
          {t("busCompose.sent", { service: sent.service, payload: sent.encodedPayload })}
          {" — "}
          {t("busCompose.sentDecoded", { text: sent.decodedEcho.text })}
        </p>
      )}
    </div>
  );
}
