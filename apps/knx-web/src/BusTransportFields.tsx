/** Render transport preferences and disclose serial prerequisites before an explicit connection. */
import { useBusConnection } from "./busConnection";
import { useBusTransport, useIpTunnelTransport, useSerialTarget } from "./serialPreference";
import { useTranslate } from "./i18n";
import { validSerialPath, validSerialSource } from "./serialTransport";
import SerialCandidateFields from "./SerialCandidateFields";
import { isExperimentalTransport } from "./ExperimentalTransportBadge";
import "./BusTransportFields.css";

export default function BusTransportFields({ disabled = false }: { disabled?: boolean }) {
  const t = useTranslate();
  const [transport, select] = useBusTransport();
  const [tunnel, selectTunnel] = useIpTunnelTransport();
  const pending = useBusConnection().action !== null;
  const [target, setTarget] = useSerialTarget();
  return <div className="bus-transport-fields">
    <label>{t("busTransport.label")}
      <select value={transport} disabled={disabled} onChange={event => select(event.target.value === "serial" ? "serial" : "ip")}>
        <option value="ip">KNXnet/IP</option><option value="serial">{t("busTransport.serialOption")}</option>
      </select>
    </label>
    {isExperimentalTransport(transport === "serial", transport === "ip" ? tunnel : undefined)
      && <p role="note" className="bus-transport-experimental">{t("transport.experimentalNote")}</p>}
    {transport === "ip" && <>
      <label>{t("busTransport.ipTunnel.label")}
        <select value={tunnel} disabled={disabled || pending} onChange={event => {
          if (!disabled && !pending) selectTunnel(event.target.value === "tcp" ? "tcp" : "udp");
        }}>
          <option value="udp">{t("busTransport.ipTunnel.udp")}</option><option value="tcp">{t("busTransport.ipTunnel.tcp")}</option>
        </select>
      </label>
      <p role="note">{t("busTransport.ipTunnel.note")}</p>
    </>}
    {transport === "serial" && <>
      <label>{t("busTransport.path")}<input value={target.path} disabled={disabled}
        aria-invalid={!validSerialPath(target.path)} onChange={event => setTarget({ ...target, path: event.target.value })} /></label>
      <SerialCandidateFields disabled={disabled} onSelect={path => setTarget({ ...target, path })} />
      <label>{t("busTransport.source")}<input value={target.source} disabled={disabled}
        aria-invalid={!validSerialSource(target.source)} placeholder="1.1.250" onChange={event => setTarget({ ...target, source: event.target.value })} /></label>
      <label>{t("busTransport.mode")}<select value={target.mode} disabled={disabled}
        onChange={event => setTarget({ ...target, mode: event.target.value === "active" ? "active" : "passive" })}>
        <option value="passive">{t("busTransport.passive")}</option><option value="active">{t("busTransport.active")}</option>
      </select></label>
      <p role="note">{t("busTransport.prerequisite")}</p>
      <p role="note">{t(target.mode === "active" ? "busTransport.activeWarning" : "busTransport.passiveHint")}</p>
      <p>{t("busTransport.ipOnly")}</p>
    </>}
  </div>;
}
