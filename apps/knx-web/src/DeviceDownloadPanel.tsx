/** Download to a device (KNXBench → device over the bus): plan, confirm, and watch every step. */
// "Download" here is only ever KNXBench → device (docs/GLOSSARY.md). The
// panel never decides what is written: the server prepares the plan from
// the open project, the user reads it, `useProgrammingConsent` asks, and
// the server demands the plan's own confirmation phrase and refuses a plan
// the project no longer gives (ADR-0045). Every data block is shown once
// the device has read it back unchanged, with its address and octets.

import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import { loadPreferredGateway } from "./gatewayPreference";
import { useTranslate } from "./i18n";
import { collectDevices } from "./treeUtils";
import { useProgrammingConsent } from "./useProgrammingConsent";
import type { ProjectTree } from "./bindings/ProjectTree";

const POLL_INTERVAL_MS = 500;

function hex(value: number, digits: number): string {
  return value.toString(16).toUpperCase().padStart(digits, "0");
}

function octetsText(octets: number[]): string {
  return octets.map((octet) => hex(octet, 2)).join(" ");
}

interface DeviceDownloadPanelProps {
  project: ProjectTree | null;
}

export default function DeviceDownloadPanel({ project }: DeviceDownloadPanelProps) {
  const t = useTranslate();
  const consent = useProgrammingConsent();
  const devices = useMemo(
    () =>
      project === null
        ? []
        : [...collectDevices(project).values()]
            .filter((device) => device.address !== null)
            .sort((a, b) => (a.address ?? "").localeCompare(b.address ?? "", undefined, { numeric: true })),
    [project],
  );
  const [address, setAddress] = useState("");
  const [gateway, setGateway] = useState(loadPreferredGateway);
  const [plan, setPlan] = useState<api.DeviceDownloadPlan | null>(null);
  const [planning, setPlanning] = useState(false);
  const [starting, setStarting] = useState(false);
  const [status, setStatus] = useState<api.DeviceDownloadStatusResponse | null>(null);
  const [events, setEvents] = useState<api.DeviceDownloadEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const sinceRef = useRef(0);
  const pollInFlightRef = useRef(false);
  const downloadIdRef = useRef<number | null>(null);

  // A plan belongs to the project it was prepared from; any edit makes it
  // stale, and the server would refuse it anyway (ADR-0045 §3).
  useEffect(() => {
    setPlan(null);
  }, [project]);

  const running = status?.status.state === "running";
  const locked = planning || starting || running;

  function apply(next: api.DeviceDownloadStatusResponse) {
    if (downloadIdRef.current !== null && next.downloadId !== downloadIdRef.current) return;
    downloadIdRef.current = next.downloadId;
    sinceRef.current = next.nextSince;
    setStatus(next);
    if (next.events.length > 0) setEvents((current) => [...current, ...next.events]);
  }

  async function poll() {
    if (pollInFlightRef.current) return;
    pollInFlightRef.current = true;
    try {
      apply(await api.pollDeviceDownload(sinceRef.current, downloadIdRef.current ?? undefined));
    } catch (reason) {
      // 409: a start is connecting right now; the next poll sees it.
      const code = api.errorStatus(reason);
      if (code !== 404 && code !== 409) setError(api.errorMessage(reason));
    } finally {
      pollInFlightRef.current = false;
    }
  }

  // Shows a download that is already running (or finished) when the panel opens.
  useEffect(() => {
    void poll();
  }, []);

  useEffect(() => {
    if (!running) return;
    const interval = window.setInterval(() => void poll(), POLL_INTERVAL_MS);
    return () => window.clearInterval(interval);
  }, [running, status?.downloadId]);

  async function preparePlan() {
    setError(null);
    setPlan(null);
    setPlanning(true);
    try {
      setPlan(await api.planDeviceDownload(address.trim()));
    } catch (reason) {
      setError(api.errorMessage(reason));
    } finally {
      setPlanning(false);
    }
  }

  async function start() {
    if (plan === null) return;
    const shown = plan;
    setError(null);
    if (!(await consent.request(`${shown.address} — ${shown.deviceName}`))) return;
    setStarting(true);
    try {
      const { downloadId } = await api.startDeviceDownload(
        shown.planId,
        gateway.trim(),
        // Built from the plan's own address by the server: the phrase for
        // exactly the device the user just confirmed.
        shown.confirmationPhrase,
      );
      downloadIdRef.current = downloadId;
      sinceRef.current = 0;
      setEvents([]);
      setStatus(null);
      setPlan(null);
      await poll();
    } catch (reason) {
      setError(api.errorMessage(reason));
    } finally {
      setStarting(false);
    }
  }

  const stepsStarted = events.filter((event) => event.kind === "stepStarted").length;
  const blocks = events.filter(
    (event): event is Extract<api.DeviceDownloadEvent, { kind: "dataWritten" }> => event.kind === "dataWritten",
  );
  const octetsWritten = blocks.length === 0 ? 0 : blocks[blocks.length - 1].written;
  const stepNames = new Map(
    events.flatMap((event) => (event.kind === "stepStarted" ? [[event.number, event.step] as const] : [])),
  );
  const observed = events.filter(
    (event): event is Extract<api.DeviceDownloadEvent, { kind: "stepDone" }> =>
      event.kind === "stepDone" && event.observed !== null,
  );

  return (
    <section className="device-download-panel">
      <header>
        <p className="eyebrow">{t("deviceDownload.eyebrow")}</p>
        <h2>{t("deviceDownload.title")}</h2>
        <p>{t("deviceDownload.explainer")}</p>
      </header>

      {project === null ? (
        <p>{t("deviceDownload.projectRequired")}</p>
      ) : (
        <form className="device-download-config" onSubmit={(event) => event.preventDefault()}>
          <label>
            {t("deviceDownload.device")}
            <select value={address} disabled={locked} onChange={(event) => { setAddress(event.target.value); setPlan(null); }}>
              <option value="">{t("deviceDownload.chooseDevice")}</option>
              {devices.map((device) => (
                <option key={device.id} value={device.address ?? ""}>
                  {device.address} — {device.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t("deviceDownload.gateway")}
            <input value={gateway} disabled={locked} onChange={(event) => setGateway(event.target.value)} placeholder="192.0.2.10:3671" />
          </label>
          <div className="device-download-actions">
            <button disabled={locked || address === ""} onClick={() => void preparePlan()}>
              {t("deviceDownload.preparePlan")}
            </button>
          </div>
        </form>
      )}

      {error && <p className="form-error" role="alert">{error}</p>}

      {plan && (
        <section className="device-download-plan" aria-label={t("deviceDownload.planTitle")}>
          <h3>{t("deviceDownload.planTitle")}</h3>
          <p>{t("deviceDownload.planNothingSent")}</p>
          <dl>
            <dt>{t("deviceDownload.target")}</dt>
            <dd><code>{plan.address}</code> — {plan.deviceName}</dd>
            <dt>{t("deviceDownload.program")}</dt>
            <dd><code>{plan.programId}</code></dd>
            <dt>{t("deviceDownload.expectedDevice")}</dt>
            <dd>{t("deviceDownload.expectedDeviceValue", { mask: hex(plan.maskVersion, 4), manufacturer: hex(plan.manufacturer, 4) })}</dd>
            <dt>{t("deviceDownload.fromProject")}</dt>
            <dd>{t("deviceDownload.fromProjectValue", { values: plan.parameterValues, links: plan.groupLinks })}</dd>
            <dt>{t("deviceDownload.octets")}</dt>
            <dd>{t("deviceDownload.octetsValue", { count: plan.dataOctets })}</dd>
          </dl>
          <table className="device-download-segments">
            <thead><tr><th>{t("deviceDownload.segment")}</th><th>{t("deviceDownload.address")}</th><th>{t("deviceDownload.segmentOctets")}</th></tr></thead>
            <tbody>
              {plan.segments.map((segment) => (
                <tr key={segment.id}>
                  <td><code>{segment.id}</code></td>
                  <td><code>{hex(segment.address, 4)}h</code></td>
                  <td>{segment.written}/{segment.size}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <details>
            <summary>{t("deviceDownload.steps", { count: plan.steps.length })}</summary>
            <ol className="device-download-steps">{plan.steps.map((step, index) => <li key={index}>{step}</li>)}</ol>
          </details>
          <button className="primary-action" disabled={locked || gateway.trim() === ""} onClick={() => void start()}>
            {t("deviceDownload.start", { address: plan.address })}
          </button>
        </section>
      )}

      {status && (
        <section className="device-download-progress" aria-live="polite">
          <h3>{t("deviceDownload.progressTitle", { address: status.address, device: status.deviceName })}</h3>
          <p data-download-state={status.status.state}>
            {status.status.state === "running"
              ? t("deviceDownload.running", { step: stepsStarted, of: status.steps, name: stepNames.get(stepsStarted) ?? "" })
              : status.status.state === "finished"
                ? t("deviceDownload.finished")
                : t("deviceDownload.failed", { step: status.status.stoppedInStep ?? "—" })}
          </p>
          <label>
            {t("deviceDownload.stepsProgress", { done: stepsStarted, of: status.steps })}
            <progress max={status.steps} value={stepsStarted} />
          </label>
          <label>
            {t("deviceDownload.octetsProgress", { done: octetsWritten, of: status.dataOctets })}
            <progress max={Math.max(status.dataOctets, 1)} value={octetsWritten} />
          </label>
          {status.status.state !== "running" && (
            <p className="device-download-written" data-written={status.status.written}>
              {t(`deviceDownload.written.${status.status.written}` as const, { count: octetsWritten })}
            </p>
          )}
          {status.status.state === "finished" && status.status.restart === "unconfirmed" && (
            <p className="form-warning" role="status">
              {t("deviceDownload.restartUnconfirmed")} {status.status.restartNote}
            </p>
          )}
          {status.status.state === "finished" && status.status.restart === "acknowledged" && (
            <p>{t("deviceDownload.restartAcknowledged")}</p>
          )}
          {status.status.state === "failed" && <p className="form-error">{status.status.error}</p>}
          {observed.length > 0 && (
            <ul className="device-download-observed">
              {observed.map((event) => <li key={event.number}>[{event.number}] {stepNames.get(event.number)}: {event.observed}</li>)}
            </ul>
          )}
          {blocks.length > 0 && (
            <table className="device-download-blocks">
              <caption>{t("deviceDownload.blocksCaption")}</caption>
              <thead><tr><th>{t("deviceDownload.step")}</th><th>{t("deviceDownload.address")}</th><th>{t("deviceDownload.data")}</th><th>{t("deviceDownload.running_total")}</th></tr></thead>
              <tbody>
                {blocks.map((block, index) => (
                  <tr key={index}>
                    <td>{block.number}</td>
                    <td><code>{hex(block.address, 4)}h</code></td>
                    <td><code>{octetsText(block.octets)}</code></td>
                    <td>{block.written}/{block.of}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </section>
      )}
      {consent.dialog}
    </section>
  );
}
