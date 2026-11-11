/** Sidebar list of devices with unprogrammed parts: select one or open its download (issue #42). */
// Read-only: nothing here plans, starts or touches the bus. "Download…" is
// the same hand-off as the device editor's "Download to device…" button.
import { useId, useMemo, useState } from "react";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { createProgrammingChips } from "./ProgrammingChips";
import { useOpenDownload } from "./OpenDownloadButton";
import type { PendingDevicesSummary } from "./pendingDevices";

/** Rows rendered before "Show all": a sidebar list, not the Devices table. */
export const PENDING_PANEL_LIMIT = 50;

export default function PendingProgrammingPanel({ summary, selectedDeviceId, onSelect }: {
  summary: PendingDevicesSummary;
  selectedDeviceId: number | null;
  onSelect: (deviceId: number) => void;
}) {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const openDownload = useOpenDownload();
  const chips = useMemo(() => createProgrammingChips(t, language), [t, language]);
  const [showAll, setShowAll] = useState(false);
  const reasonId = useId();
  const { devices, unknown } = summary;
  const shown = showAll ? devices : devices.slice(0, PENDING_PANEL_LIMIT);
  return <div className="pending-programming-panel">
    {devices.length === 0 && <p className="pending-programming-empty">{t("pendingPanel.empty")}</p>}
    {devices.length > 0 && <ul className="pending-programming-list" aria-label={t("pendingPanel.listLabel")}>
      {shown.map((device) => {
        const label = device.address ? `${device.address} ${device.name}` : device.name;
        return <li key={device.id} className="pending-programming-row" data-device-id={device.id}>
          <button type="button" className="pending-programming-open" onClick={() => onSelect(device.id)}
            aria-current={device.id === selectedDeviceId ? "true" : undefined}>
            <span className="pending-programming-name">
              {device.address && <span className="pending-programming-address">{device.address}</span>}
              <span title={device.name}>{device.name}</span>
            </span>
            {chips(device)}
          </button>
          {openDownload && (device.address
            ? <button type="button" className="pending-programming-download" onClick={() => openDownload(device.id)}
              aria-label={t("pendingPanel.downloadDevice", { device: label })} title={t("pendingPanel.downloadDevice", { device: label })}>
              {t("pendingPanel.download")}</button>
            : <>
              <button type="button" className="pending-programming-download" disabled aria-describedby={`${reasonId}-${device.id}`}
                aria-label={t("pendingPanel.downloadDevice", { device: label })}>{t("pendingPanel.download")}</button>
              <small id={`${reasonId}-${device.id}`} className="pending-programming-reason">{t("devices.downloadNeedsAddress")}</small>
            </>)}
        </li>;
      })}
    </ul>}
    {shown.length < devices.length && <p className="pending-programming-more">
      <span>{t("pendingPanel.showing", { shown: shown.length, count: devices.length })}</span>
      <button type="button" onClick={() => setShowAll(true)}>{t("pendingPanel.showAll")}</button>
    </p>}
    {unknown > 0 && <p className="pending-programming-unknown">{t("pendingPanel.unknown", { count: unknown })}</p>}
  </div>;
}
