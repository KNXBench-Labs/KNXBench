/** Offline project readiness and opt-in, read-only comparison of one device. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import DeviceLink from "./DeviceLink";
import { emitAchievementEvent } from "./achievementEvents";
import { loadPreferredGateway } from "./gatewayPreference";
import { splitGatewayEndpoint, validateGatewayFields } from "./gatewayEndpoint";
import { useTranslate, type Translate } from "./i18n";
import type { ProjectTree } from "./bindings/ProjectTree";

function gradeLabel(t: Translate, code: string): string {
  switch (code) {
    case "verified": return t("deviceChecks.grade.verified");
    case "untested": return t("deviceChecks.grade.untested");
    case "unsupported": return t("deviceChecks.grade.unsupported");
    case "excluded": return t("deviceChecks.grade.excluded");
    case "no-address": return t("deviceChecks.grade.noAddress");
    default: return t("deviceChecks.grade.unknown", { code });
  }
}

function hex(value: number, digits: number): string {
  return `${value.toString(16).toUpperCase().padStart(digits, "0")}h`;
}

function hexOctets(octets: number[]): string {
  return octets.map((octet) => octet.toString(16).toUpperCase().padStart(2, "0")).join(" ");
}

/** A device the server can prepare a download plan for. */
function isPlannable(device: api.DeviceReadinessRow): boolean {
  return device.readiness === "verified" || device.readiness === "untested";
}

export default function DeviceInspectionPanel({ project }: { project: ProjectTree | null }) {
  const t = useTranslate();
  const [readiness, setReadiness] = useState<api.DeviceReadinessResponse | null>(null);
  const [readinessError, setReadinessError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [address, setAddress] = useState("");
  const [gateway, setGateway] = useState(loadPreferredGateway);
  const [review, setReview] = useState<{ address: string; gateway: string } | null>(null);
  const [comparing, setComparing] = useState(false);
  const [compareError, setCompareError] = useState<string | null>(null);
  const [result, setResult] = useState<api.DeviceCompareResponse | null>(null);
  const readinessVersion = useRef(0);
  const comparisonVersion = useRef(0);

  async function refresh() {
    const version = ++readinessVersion.current;
    ++comparisonVersion.current;
    setReadiness(null);
    setReadinessError(null);
    setCompareError(null);
    setAddress("");
    setReview(null);
    setResult(null);
    setComparing(false);
    if (project === null) return;
    setLoading(true);
    try {
      const next = await api.getDeviceReadiness();
      if (readinessVersion.current === version) {
        setReadiness(next);
        emitAchievementEvent({
          type: "readinessChecked",
          deviceCount: next.devices.length,
          unplannableCount: next.devices.filter((device) => !isPlannable(device)).length,
        });
      }
    } catch (reason) {
      if (readinessVersion.current === version) setReadinessError(api.errorMessage(reason));
    } finally {
      if (readinessVersion.current === version) setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
    return () => { ++readinessVersion.current; ++comparisonVersion.current; };
  }, [project]);

  const addresses = new Map<string, number>();
  for (const device of readiness?.devices ?? []) {
    if (device.address !== null) addresses.set(device.address, (addresses.get(device.address) ?? 0) + 1);
  }
  const ambiguous = [...addresses.values()].some((count) => count > 1);
  const eligible = readiness?.devices.filter((device) =>
    device.address !== null && addresses.get(device.address) === 1 &&
    isPlannable(device),
  ) ?? [];

  function editAddress(value: string) {
    ++comparisonVersion.current;
    setAddress(value);
    setReview(null);
    setResult(null);
    setCompareError(null);
  }

  function editGateway(value: string) {
    ++comparisonVersion.current;
    setGateway(value);
    setReview(null);
    setResult(null);
    setCompareError(null);
  }

  function reviewComparison() {
    setCompareError(null);
    setResult(null);
    if (!eligible.some((device) => device.address === address)) return;
    const checked = validateGatewayFields(splitGatewayEndpoint(gateway));
    if (!checked.ok) {
      setCompareError(t(checked.reason === "hostRequired" ? "busMonitor.connectNeedsGateway" : checked.reason === "invalidPort" ? "busMonitor.invalidPort" : "busMonitor.invalidHost"));
      return;
    }
    setReview({ address, gateway: checked.endpoint });
  }

  async function compare() {
    if (review === null || !eligible.some((device) => device.address === review.address)) return;
    const selected = review;
    const version = ++comparisonVersion.current;
    setReview(null);
    setCompareError(null);
    setResult(null);
    setComparing(true);
    try {
      // This POST opens a read-only management tunnel. The server accepts
      // no key, phrase or write scope; no call happens on mount or review.
      const next = await api.compareDevice(selected.address, selected.gateway);
      if (comparisonVersion.current !== version) return;
      if (next.written !== false) setCompareError(t("deviceChecks.unexpectedWrite"));
      else if (next.address !== selected.address || next.partial !== false) setCompareError(t("deviceChecks.unexpectedScope"));
      else if (
        next.same !== (next.differingOctets === 0) ||
        next.changes.some((change) => change.device.length !== change.project.length) ||
        next.changes.reduce((sum, change) => sum + change.project.length, 0) !== next.differingOctets
      ) setCompareError(t("deviceChecks.inconsistentResult"));
      else {
        setResult(next);
        emitAchievementEvent({ type: "deviceCompared", differingOctets: next.differingOctets });
      }
    } catch (reason) {
      if (comparisonVersion.current === version) setCompareError(api.errorMessage(reason));
    } finally {
      if (comparisonVersion.current === version) setComparing(false);
    }
  }

  return <section className="device-checks-panel">
    <header>
      <p className="eyebrow">{t("deviceChecks.eyebrow")}</p>
      <h2>{t("deviceChecks.title")}</h2>
      <p>{t("deviceChecks.intro")}</p>
    </header>
    {project === null ? <p>{t("deviceChecks.projectRequired")}</p> : <>
      <section className="device-checks-readiness-section" aria-label={t("deviceChecks.grade")}>
        <div className="device-checks-heading">
          <h3>{t("deviceChecks.grade")}</h3>
          <button type="button" onClick={() => void refresh()} disabled={loading || comparing}>{t("deviceChecks.refresh")}</button>
        </div>
        <p>{t("deviceChecks.scope")}</p>
        {loading && <p role="status">{t("deviceChecks.loading")}</p>}
        {readinessError && <p className="form-error" role="alert">{readinessError}</p>}
        {readiness && <>
          <p>{t("deviceChecks.count", { count: readiness.devices.length })}</p>
          <ul className="device-checks-counts">
            {Object.entries(readiness.counts).map(([code, count]) => <li key={code}>{gradeLabel(t, code)}: <strong>{count}</strong></li>)}
          </ul>
          <div className="device-checks-table-scroll" role="region" aria-label={t("deviceChecks.grade")} tabIndex={0}>
            <table className="device-checks-readiness">
              <thead><tr>
                <th>{t("deviceDownload.device")}</th><th>{t("deviceChecks.address")}</th>
                <th>{t("deviceChecks.grade")}</th><th>{t("deviceChecks.evidence")}</th>
                <th>{t("deviceChecks.planSize")}</th>
              </tr></thead>
              <tbody>{readiness.devices.map((device, index) => <tr key={index}>
                <td><DeviceLink address={device.address}>{device.name}</DeviceLink><small><code>{device.programRef || "—"}</code></small></td>
                <td><code>{device.address ?? "—"}</code></td>
                <td>{gradeLabel(t, device.readiness)}</td>
                <td>{device.category && <code>{device.category}: </code>}{device.detail ?? t("deviceChecks.noEvidence")}</td>
                <td>{device.steps !== null || device.octets !== null
                  ? `${device.steps ?? "—"} / ${device.octets ?? "—"}` : "—"}</td>
              </tr>)}</tbody>
            </table>
          </div>
          {ambiguous && <p className="form-warning">{t("deviceChecks.ambiguous")}</p>}
        </>}
      </section>

      <section className="device-checks-compare" aria-label={t("deviceChecks.compareTitle")}>
        <h3>{t("deviceChecks.compareTitle")}</h3>
        <p>{t("deviceChecks.compareNotice")}</p>
        <p>{t("deviceChecks.fullOnly")}</p>
        {readiness && eligible.length === 0 && <p>{t("deviceChecks.noEligible")}</p>}
        <div className="device-checks-config">
          <label>{t("deviceDownload.device")}
            <select className="device-checks-target" value={address} disabled={loading || comparing || eligible.length === 0}
              onChange={(event) => editAddress(event.target.value)}>
              <option value="">{t("deviceChecks.choose")}</option>
              {eligible.map((device) => <option key={device.address} value={device.address ?? ""}>{device.address} — {device.name}</option>)}
            </select>
          </label>
          <label>{t("deviceDownload.gateway")}
            <input className="device-checks-gateway" value={gateway} disabled={comparing} onChange={(event) => editGateway(event.target.value)} placeholder="192.0.2.10:3671" />
          </label>
          <button type="button" disabled={loading || comparing || !address || !eligible.some((device) => device.address === address)} onClick={reviewComparison}>
            {t("deviceChecks.review")}
          </button>
        </div>
        {address && <p><DeviceLink address={address}>{t("devices.openEditor")}</DeviceLink></p>}
        {review && <div className="device-checks-confirm" role="group" aria-label={t("deviceChecks.review")}>
          <p>{t("deviceChecks.confirmHint", review)}</p>
          <button type="button" onClick={() => void compare()}>{t("deviceChecks.confirm")}</button>
          <button type="button" onClick={() => setReview(null)}>{t("deviceChecks.cancel")}</button>
        </div>}
        {comparing && <p role="status">{t("deviceChecks.comparing")}</p>}
        {compareError && <p className="form-error" role="alert">{compareError}</p>}
        {result && <section className="device-checks-result" aria-label={t("deviceChecks.resultTitle")}>
          <h4>{t("deviceChecks.resultTitle")}</h4>
          <p>{t("deviceChecks.noWrite")}</p>
          <dl>
            <dt>{t("deviceDownload.device")}</dt><dd><code>{result.address}</code> — {result.deviceName}</dd>
            <dt>{t("deviceChecks.program")}</dt><dd><code>{result.programId}</code></dd>
            <dt>{t("deviceChecks.mask")}</dt><dd><code>{hex(result.mask, 4)}</code></dd>
            <dt>{t("deviceChecks.manufacturer")}</dt><dd><code>{hex(result.manufacturer, 4)}</code></dd>
            <dt>{t("deviceChecks.octets")}</dt><dd>{result.octets}</dd>
            <dt>{t("deviceChecks.differing")}</dt><dd>{result.differingOctets}</dd>
          </dl>
          <p>{t(result.same ? "deviceChecks.same" : "deviceChecks.different")}</p>
          <h5>{t("deviceChecks.loadStates")}</h5>
          {result.loadStates.length === 0 ? <p>{t("deviceChecks.noLoadStates")}</p> :
            <ul>{result.loadStates.map((state, index) => <li key={index}>{state.machine}: {state.state}</li>)}</ul>}
          <h5>{t("deviceChecks.changes")}</h5>
          {result.changes.length === 0 ? <p>{t("deviceChecks.noRanges")}</p> : <div className="device-checks-table-scroll" role="region" aria-label={t("deviceChecks.changes")} tabIndex={0}>
            <table className="device-checks-changes">
              <thead><tr><th>{t("deviceChecks.address")}</th><th>{t("deviceChecks.segment")}</th>
                <th>{t("deviceChecks.deviceBytes")}</th><th>{t("deviceChecks.projectBytes")}</th></tr></thead>
              <tbody>{result.changes.map((change, index) => <tr key={index}>
                <td><code>{hex(change.address, 4)}</code></td><td><code>{change.segment ?? "—"}</code></td>
                <td><code>{hexOctets(change.device)}</code></td><td><code>{hexOctets(change.project)}</code></td>
              </tr>)}</tbody>
            </table>
          </div>}
        </section>}
      </section>
    </>}
  </section>;
}
