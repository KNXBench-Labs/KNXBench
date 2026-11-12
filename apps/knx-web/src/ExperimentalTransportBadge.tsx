/** Mark TCP tunnelling and USB/serial TP-UART as experimental wherever they are shown. */
import { useTranslate } from "./i18n";
import type { IpTunnelTransport } from "./serialTransport";
import "./ExperimentalTransportBadge.css";

/**
 * UDP tunnelling is the supported transport; TCP and the serial TP-UART have no
 * real-interface test yet (REL-01 B-14), so every place that names them says so.
 */
export function isExperimentalTransport(serial: boolean, tunnel: IpTunnelTransport | undefined): boolean {
  return serial || tunnel === "tcp";
}

export default function ExperimentalTransportBadge() {
  const t = useTranslate();
  // The leading space keeps the tag a separate word in text and accessible names.
  return <>{" "}<span className="transport-experimental-badge" title={t("transport.experimentalNote")}>{t("transport.experimental")}</span></>;
}
