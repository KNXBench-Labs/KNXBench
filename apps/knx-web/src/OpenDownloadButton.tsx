/** "Download to device…" from the device editor: navigation only, by device id. */
import { createContext, useContext, useId, type ReactNode } from "react";
import { useTranslate } from "./i18n";

type OpenDownload = (deviceId: number) => void;
const DownloadNavigation = createContext<OpenDownload | null>(null);

/** Mounted by the main workbench only; the diagnostics companion has no bus workspace. */
export function DownloadNavigationProvider({ onOpen, children }: { onOpen: OpenDownload; children: ReactNode }) {
  return <DownloadNavigation.Provider value={onOpen}>{children}</DownloadNavigation.Provider>;
}

/**
 * Opens the bus workspace's download tab with this device preselected. It
 * never prepares a plan, starts a download or opens a connection; a device
 * without an individual address is explained instead of offered.
 */
export default function OpenDownloadButton({ deviceId, address }: { deviceId: number; address: string | null }) {
  const open = useContext(DownloadNavigation);
  const t = useTranslate();
  const reason = useId();
  if (!open) return null;
  if (address === null) {
    return <span className="device-download-entry">
      <button type="button" disabled aria-describedby={reason}>{t("devices.downloadToDevice")}</button>
      <small id={reason}>{t("devices.downloadNeedsAddress")}</small>
    </span>;
  }
  return <span className="device-download-entry">
    <button type="button" onClick={() => open(deviceId)}>{t("devices.downloadToDevice")}</button>
  </span>;
}
