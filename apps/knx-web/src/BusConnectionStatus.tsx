/** Status-bar bus connection: state, gateway drop-up and explicit connect/disconnect. */
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import * as api from "./api";
import { appUiScale } from "./appUiScale";
import {
  acquireBusConnectionPolling, connectBusSession, disconnectBusSession, refreshBusConnection, useBusConnection,
  type BusActionResult, type BusConnectionState,
} from "./busConnection";
import { searchBusInterfaces, useBusDiscovery } from "./busDiscovery";
import { splitGatewayEndpoint, validateGatewayFields, type GatewayValidation } from "./gatewayEndpoint";
import { usePreferredGateway } from "./gatewayPreference";
import { describeHolder, gatewayRefusalMessage } from "./gatewayRefusal";
import { useTranslate, type Translate, type TranslatableKey } from "./i18n";
import "./BusConnectionStatus.css";

const INVALID_REASON: Record<Exclude<GatewayValidation, { ok: true }>["reason"], TranslatableKey> = {
  hostRequired: "busMonitor.connectNeedsGateway",
  unsupportedHost: "busMonitor.invalidHost",
  invalidPort: "busMonitor.invalidPort",
};

/** Indicator tone: colour is never the only carrier, the text says the same. */
function tone({ view, action }: BusConnectionState): string {
  if (action !== null) return "pending";
  switch (view.phase) {
    case "connected": return "connected";
    case "lost": case "occupied": return "warning";
    case "disconnected": return "idle";
    default: return "unknown";
  }
}

function stateText({ view, action }: BusConnectionState, t: Translate): string {
  if (action === "connecting") return t("busConnection.state.connecting");
  if (action === "disconnecting") return t("busConnection.state.disconnecting");
  switch (view.phase) {
    case "connected":
      return view.gateway === undefined
        ? t("busConnection.state.connectedUnnamed", { id: view.sessionId })
        : t("busConnection.state.connected", { gateway: view.gateway });
    case "lost":
      return view.gateway === undefined
        ? t("busConnection.state.lostUnnamed", { id: view.sessionId })
        : t("busConnection.state.lost", { gateway: view.gateway });
    case "disconnected": return t("busConnection.state.disconnected");
    case "occupied": return t("busConnection.state.occupied");
    case "indeterminate": return t("busConnection.state.indeterminate");
    case "unknown": return t("busConnection.state.unknown");
  }
}

function detailText({ view }: BusConnectionState, t: Translate): string {
  switch (view.phase) {
    case "connected": return t("busConnection.disclosure");
    case "lost": return t("busConnection.detail.lost");
    case "disconnected": return t("busConnection.detail.disconnected");
    case "occupied": return describeHolder(view.holder, t);
    case "indeterminate": return t("busConnection.detail.indeterminate");
    case "unknown": return t(`busConnection.detail.${view.reason}`);
  }
}

/** Worded refusal, else a localized generic text; never raw server text or JSON. */
function actionError(result: BusActionResult<unknown>, fallback: TranslatableKey, t: Translate): string | null {
  if (result.ok) return null;
  switch (result.reason) {
    case "inFlight": return null;
    case "stale":
      return fallback === "busConnection.error.disconnect"
        ? t("busConnection.error.staleStop") : t("busConnection.error.stale");
    case "invalidIdentity": return t(fallback);
    case "failed": break;
  }
  const status = api.errorStatus(result.error);
  if (fallback === "busConnection.error.disconnect" && (status === 404 || status === 405)) {
    return t("busConnection.error.unsupported");
  }
  return gatewayRefusalMessage(result.error) ?? t(fallback);
}

export default function BusConnectionStatus({ onOpenBusSettings }: { onOpenBusSettings: () => void }) {
  const t = useTranslate();
  const connection = useBusConnection();
  const discovery = useBusDiscovery();
  const [selected, select] = usePreferredGateway();
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelId = useId();

  useEffect(() => acquireBusConnectionPolling(), []);

  useEffect(() => {
    if (!open) return;
    void refreshBusConnection();
    function onPointerDown(event: PointerEvent) {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    }
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  // The bar wraps on narrow or zoomed windows, so its height is not known in
  // CSS: bound the drop-up to the space above the bar, in layout pixels.
  useLayoutEffect(() => {
    if (!open) return;
    function fit() {
      const panel = document.getElementById(panelId);
      const bar = rootRef.current?.closest(".workbench-status");
      if (!panel || !bar) return;
      const above = bar.getBoundingClientRect().top / appUiScale() - 8;
      panel.style.maxHeight = `${Math.max(96, Math.floor(above))}px`;
    }
    fit();
    window.addEventListener("resize", fit);
    return () => window.removeEventListener("resize", fit);
  }, [open, panelId]);

  const validation = validateGatewayFields(splitGatewayEndpoint(selected));
  const { view, action } = connection;
  const bound = view.phase === "connected" || view.phase === "lost" ? view : null;
  const connectReason = !validation.ok
    ? t(INVALID_REASON[validation.reason])
    : view.phase === "disconnected" ? null : t("busConnection.connectBlocked");

  async function connect() {
    if (!validation.ok) return;
    setError(null);
    const result = await connectBusSession(validation.endpoint);
    setError(actionError(result, "busConnection.error.connect", t));
  }

  async function disconnect() {
    if (bound === null) return;
    setError(null);
    const result = await disconnectBusSession({ sessionId: bound.sessionId, serverIncarnation: bound.serverIncarnation });
    setError(actionError(result, "busConnection.error.disconnect", t));
  }

  function close(returnFocus: boolean) {
    setOpen(false);
    if (returnFocus) triggerRef.current?.focus();
  }

  const listed = discovery.interfaces.map((entry) => entry.controlEndpoint);
  const savedOnly = selected !== "" && !listed.includes(selected);
  const selectedLabel = selected === "" ? t("busConnection.noneSelected") : selected;

  return (
    <div
      className="bus-connection"
      ref={rootRef}
      onKeyDown={(event) => {
        if (event.key === "Escape" && open) {
          event.stopPropagation();
          close(true);
        }
      }}
      onBlur={(event) => {
        if (open && !rootRef.current?.contains(event.relatedTarget as Node | null)) setOpen(false);
      }}
    >
      <span className="bus-connection-state" title={detailText(connection, t)}>
        <span className="bus-connection-indicator" data-tone={tone(connection)} aria-hidden="true" />
        {stateText(connection, t)}
      </span>
      <button
        ref={triggerRef}
        type="button"
        className="bus-connection-trigger"
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        onClick={() => setOpen((value) => !value)}
      >
        {t("busConnection.trigger", { gateway: selectedLabel })}
        <span aria-hidden="true"> ▴</span>
      </button>
      {bound ? (
        <button type="button" className="bus-connection-action" onClick={() => void disconnect()} disabled={action !== null}>
          {t("busConnection.disconnect")}
        </button>
      ) : (
        <button
          type="button"
          className="bus-connection-action"
          onClick={() => void connect()}
          disabled={action !== null || connectReason !== null}
          title={connectReason ?? t("busConnection.disclosure")}
        >
          {t("busConnection.connect")}
        </button>
      )}
      {error && <span className="bus-connection-error" role="status">{error}</span>}
      {open && (
        <div id={panelId} className="bus-connection-panel" role="group" aria-label={t("busConnection.label")}>
          <p className="bus-connection-detail">{stateText(connection, t)}: {detailText(connection, t)}</p>
          {connection.restarted && <p className="bus-connection-detail">{t("busConnection.detail.restarted")}</p>}
          {bound?.gateway !== undefined && selected !== "" && bound.gateway !== selected && (
            <p className="bus-connection-detail">
              {t("busConnection.selectionDiffers", { connected: bound.gateway, selected })}
            </p>
          )}
          {!bound && connectReason && <p className="bus-connection-detail">{connectReason}</p>}
          {!bound && <p className="bus-connection-detail">{t("busConnection.disclosure")}</p>}
          <p className="bus-connection-heading">{t("busConnection.interfaces")}</p>
          <ul className="bus-connection-list">
            {savedOnly && (
              <li>
                <button type="button" className="bus-discovery-option" aria-pressed="true" onClick={() => select(selected)}>
                  <span className="bus-discovery-endpoint">{selected}</span>
                  <span className="bus-discovery-tag">{t("busConnection.saved")}</span>
                </button>
              </li>
            )}
            {discovery.interfaces.map((entry) => (
              <li key={entry.controlEndpoint}>
                <button
                  type="button"
                  className="bus-discovery-option"
                  aria-pressed={entry.controlEndpoint === selected}
                  disabled={!entry.supportsTunnelling}
                  onClick={() => select(entry.controlEndpoint)}
                >
                  <span className="bus-discovery-name">{entry.friendlyName}</span>
                  <span className="bus-discovery-endpoint">{entry.controlEndpoint} · {entry.individualAddress}</span>
                  {entry.supportsTunnelling
                    ? entry.controlEndpoint === selected && <span className="bus-discovery-tag">{t("busConnection.selected")}</span>
                    : <span className="bus-connection-reason">{t("busConnection.notTunnelling")}</span>}
                </button>
              </li>
            ))}
          </ul>
          {discovery.phase === "done" && discovery.interfaces.length === 0 && (
            <p className="bus-connection-detail">{t("busConnection.noInterfaces")}</p>
          )}
          {discovery.phase === "failed" && <p className="bus-connection-detail">{t("busConnection.searchFailed")}</p>}
          <div className="bus-connection-tools">
            <button type="button" onClick={() => void refreshBusConnection()}>{t("busConnection.checkStatus")}</button>
            <button type="button" onClick={() => void searchBusInterfaces()} disabled={discovery.phase === "searching"}>
              {discovery.phase === "searching" ? t("busDiscovery.searching") : t("busConnection.searchAgain")}
            </button>
            <button type="button" onClick={() => { close(false); onOpenBusSettings(); }}>{t("busConnection.settings")}</button>
          </div>
        </div>
      )}
    </div>
  );
}
