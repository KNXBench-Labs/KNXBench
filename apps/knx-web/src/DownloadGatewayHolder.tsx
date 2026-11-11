/** Issue #10: the download page says up front what holds the gateway, and offers to stop it. */
import { useCallback, useEffect, useState } from "react";
import * as api from "./api";
import { describeHolder, holderFromActivity, type TunnelHolder } from "./gatewayRefusal";
import { useTranslate } from "./i18n";

/** How often the volatile activity snapshot is read while the page is open. */
export const HOLDER_POLL_MS = 3000;

interface DownloadGatewayHolderProps {
  /** The gateway field's current value. */
  gateway: string;
  /** No stop or suggestion while this page's own plan or download is busy. */
  disabled: boolean;
  /** Puts an endpoint into the gateway field, as if the user picked it. */
  onUseGateway: (endpoint: string) => void;
}

/**
 * Reads `GET /api/bus/activity` (ADR-0055: partial, volatile) and names the
 * KNXBench session holding the gateway's one tunnel before anything is
 * prepared. A failed read shows nothing: this is a hint, and the start's own
 * structured refusal stays authoritative. Another client (ETS) is never
 * visible here; the gateway names it when a start is refused.
 */
export default function DownloadGatewayHolder({ gateway, disabled, onUseGateway }: DownloadGatewayHolderProps) {
  const t = useTranslate();
  const [snapshot, setSnapshot] = useState<{ holder: TunnelHolder | null; serverIncarnation: string } | null>(null);
  const holder = snapshot?.holder ?? null;
  const [stopping, setStopping] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [scanCleanupError, setScanCleanupError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const activity = await api.busActivity();
      setSnapshot({ holder: holderFromActivity(activity), serverIncarnation: activity.serverIncarnation });
    } catch {
      setSnapshot(null);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => void refresh(), HOLDER_POLL_MS);
    return () => window.clearInterval(interval);
  }, [refresh]);

  // A terminal activity snapshot has no holder; that must not erase failed cleanup.
  const errorNotice = error === null ? null : <p className="form-error" role="alert">{error}</p>;
  const cleanupNotice = scanCleanupError === null ? null
    : <p className="form-error" role="alert">{t("lineScan.cleanupUnconfirmed", { error: scanCleanupError })}</p>;
  if (holder === null) return <>{errorNotice}{cleanupNotice}</>;
  const monitorGateway = holder.kind === "busMonitor" ? holder.gateway : undefined;

  async function stop(current: TunnelHolder, serverIncarnation: string) {
    setError(null);
    setStopping(true);
    try {
      // WP-26: only the session this notice shows; a changed session answers
      // `busSessionChanged` and stays running. Never the legacy global stop.
      if (current.kind === "busMonitor") await api.stopBoundBusMonitor({ sessionId: current.sessionId, serverIncarnation });
      else if (current.kind === "lineScan") {
        const acknowledgement = await api.cancelLineScan({ sessionId: current.sessionId, serverIncarnation });
        if (acknowledgement.disconnectConfirmed === false) setScanCleanupError(acknowledgement.cleanupError);
      }
      else if (current.kind === "addressProgramming" && current.sessionId !== undefined) {
        await api.stopAddressProgramming(current.sessionId);
      }
      // The monitor's gateway worked a moment ago: keep it for the download
      // when no gateway was entered yet.
      if (current.kind === "busMonitor" && current.gateway !== undefined && gateway.trim() === "") {
        onUseGateway(current.gateway);
      }
    } catch (reason) {
      setError(api.errorMessage(reason));
    } finally {
      setStopping(false);
      await refresh();
    }
  }

  const stopLabel = holder.kind === "busMonitor" ? t("deviceDownload.holder.stopMonitor")
    : holder.kind === "lineScan" ? t("deviceDownload.holder.cancelScan")
      : holder.kind === "addressProgramming" && holder.sessionId !== undefined ? t("deviceDownload.holder.stopProgramming")
        : null;

  return (
    <div className="device-download-holder" data-holder={holder.kind}>
      <p role="status">{describeHolder(holder, t)}</p>
      <div className="device-download-holder-actions">
        {stopLabel && (
          <button type="button" disabled={disabled || stopping} onClick={() => snapshot && void stop(holder, snapshot.serverIncarnation)}>{stopLabel}</button>
        )}
        {monitorGateway !== undefined && monitorGateway !== gateway.trim() && (
          <button type="button" disabled={disabled} onClick={() => onUseGateway(monitorGateway)}>
            {t("deviceDownload.holder.useMonitorGateway", { gateway: monitorGateway })}
          </button>
        )}
      </div>
      {errorNotice}
      {cleanupNotice}
    </div>
  );
}
