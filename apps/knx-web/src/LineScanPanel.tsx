/** Read-only line diagnostics with cost preview, protected exclusions and incremental results. */

import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import { type Translate, useTranslate } from "./i18n";
import { getSetting, setSetting, useSettingsRevision } from "./settingsStore";

const EXCLUSIONS_KEY = "lineScanExclusions";
const POLL_INTERVAL_MS = 1000;

function configuredExclusions(): string[] {
  const value = getSetting(EXCLUSIONS_KEY);
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

function seconds(milliseconds: number): string {
  return `${(milliseconds / 1000).toFixed(1)} s`;
}

function outcomeLabel(outcome: api.LineScanOutcome, t: Translate): string {
  switch (outcome.kind) {
    case "occupied":
      return outcome.maskVersion === null
        ? t("lineScan.outcome.occupiedNoMask")
        : t("lineScan.outcome.occupiedMask", {
            mask: `0x${outcome.maskVersion.toString(16).padStart(4, "0")}`,
          });
    case "occupiedBusy":
      return t("lineScan.outcome.occupiedBusy");
    case "occupiedSilent":
      return t("lineScan.outcome.occupiedSilent");
    case "vacant":
      return t("lineScan.outcome.vacant");
    case "indeterminate":
      return t("lineScan.outcome.indeterminate");
    case "selfAddress":
      return t("lineScan.outcome.selfAddress");
  }
}

function statusKey(status: api.LineScanResultsResponse["status"]) {
  switch (status) {
    case "running": return "lineScan.status.running" as const;
    case "completed": return "lineScan.status.completed" as const;
    case "cancelled": return "lineScan.status.cancelled" as const;
    case "failed": return "lineScan.status.failed" as const;
  }
}

export default function LineScanPanel() {
  const t = useTranslate();
  useSettingsRevision();
  const exclusions = configuredExclusions();
  const [gateway, setGateway] = useState("");
  const [area, setArea] = useState(1);
  const [line, setLine] = useState(1);
  const [firstDevice, setFirstDevice] = useState(1);
  const [lastDevice, setLastDevice] = useState(255);
  const [responseTimeoutMs, setResponseTimeoutMs] = useState(6000);
  const [interProbePauseMs, setInterProbePauseMs] = useState(100);
  const [estimate, setEstimate] = useState<{
    value: api.LineScanEstimate;
    requestFingerprint: string;
  } | null>(null);
  const [response, setResponse] = useState<api.LineScanResultsResponse | null>(null);
  const [results, setResults] = useState<api.LineScanResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [confirmRemoval, setConfirmRemoval] = useState<string | null>(null);
  const [newExclusion, setNewExclusion] = useState("");
  const [starting, setStarting] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [activeExclusions, setActiveExclusions] = useState<string[]>([]);
  const sinceRef = useRef(0);
  const pollGenerationRef = useRef(0);
  const pollInFlightRef = useRef(false);
  const sessionIdRef = useRef<number | null>(null);

  const request = useMemo<api.LineScanRequest>(
    () => ({
      gateway,
      area,
      line,
      firstDevice,
      lastDevice,
      excluded: exclusions,
      responseTimeoutMs,
      interProbePauseMs,
    }),
    [gateway, area, line, firstDevice, lastDevice, exclusions.join("\u0000"), responseTimeoutMs, interProbePauseMs],
  );
  const requestFingerprint = JSON.stringify(request);
  const requestFingerprintRef = useRef(requestFingerprint);
  requestFingerprintRef.current = requestFingerprint;
  const controlsLocked = starting || cancelling || response?.status === "running";
  const displayedExclusions = controlsLocked ? activeExclusions : exclusions;

  function changeNumber(setter: (value: number) => void, value: string) {
    setter(Number(value));
    setEstimate(null);
  }

  function apply(next: api.LineScanResultsResponse, expectedSessionId: number | null) {
    if (expectedSessionId !== null && next.sessionId !== expectedSessionId) return;
    sessionIdRef.current = next.sessionId;
    sinceRef.current = next.nextSince;
    setResponse(next);
    setActiveExclusions(next.excludedAddresses);
    if (next.results.length > 0) setResults((current) => [...current, ...next.results]);
  }

  async function poll(
    since: number,
    generation = pollGenerationRef.current,
    expectedSessionId = sessionIdRef.current,
  ) {
    if (pollInFlightRef.current) return;
    pollInFlightRef.current = true;
    try {
      const next = await api.pollLineScan(since, expectedSessionId ?? undefined);
      if (generation === pollGenerationRef.current) apply(next, expectedSessionId);
    } catch (reason) {
      if (api.errorStatus(reason) !== 404) setError(api.errorMessage(reason));
    } finally {
      pollInFlightRef.current = false;
    }
  }

  useEffect(() => {
    void poll(0);
  }, []);

  useEffect(() => {
    if (response?.status !== "running" || cancelling) return;
    const generation = pollGenerationRef.current;
    const expectedSessionId = response.sessionId;
    const interval = window.setInterval(
      () => void poll(sinceRef.current, generation, expectedSessionId),
      POLL_INTERVAL_MS,
    );
    return () => window.clearInterval(interval);
  }, [response?.status, response?.sessionId, cancelling]);

  async function preview() {
    setError(null);
    const fingerprint = requestFingerprint;
    try {
      const value = await api.estimateLineScan(request);
      if (requestFingerprintRef.current === fingerprint) {
        setEstimate({ value, requestFingerprint: fingerprint });
      }
    } catch (reason) {
      setError(api.errorMessage(reason));
    }
  }

  async function start() {
    if (!estimate || estimate.requestFingerprint !== requestFingerprint) return;
    pollGenerationRef.current += 1;
    setStarting(true);
    setActiveExclusions(request.excluded);
    setError(null);
    setResults([]);
    sinceRef.current = 0;
    try {
      const started = await api.startLineScan(request);
      sessionIdRef.current = started.sessionId;
      setResponse({
        sessionId: started.sessionId,
        status: "running",
        error: null,
        nextSince: 0,
        completedCount: 0,
        totalCount: started.estimate.candidateCount,
        omittedAddresses: started.estimate.omittedAddresses,
        excludedAddresses: request.excluded,
        results: [],
      });
      await poll(0);
    } catch (reason) {
      setError(api.errorMessage(reason));
    } finally {
      setStarting(false);
    }
  }

  async function cancel() {
    const sessionId = sessionIdRef.current;
    if (sessionId === null) return;
    setCancelling(true);
    pollGenerationRef.current += 1;
    setError(null);
    try {
      const cancelled = await api.cancelLineScan(sessionId);
      pollGenerationRef.current += 1;
      sinceRef.current = cancelled.nextSince;
      setResponse(cancelled);
      setResults(cancelled.results);
    } catch (reason) {
      setError(api.errorMessage(reason));
    } finally {
      setCancelling(false);
    }
  }

  function removeExclusion(address: string) {
    if (confirmRemoval !== address) {
      setConfirmRemoval(address);
      return;
    }
    setSetting(EXCLUSIONS_KEY, exclusions.filter((item) => item !== address));
    setConfirmRemoval(null);
    setEstimate(null);
  }

  function addExclusion() {
    const address = newExclusion.trim();
    if (!/^\d{1,2}\.\d{1,2}\.\d{1,3}$/.test(address) || exclusions.includes(address)) return;
    setSetting(EXCLUSIONS_KEY, [...exclusions, address]);
    setNewExclusion("");
    setEstimate(null);
  }

  return (
    <section className="line-scan-panel">
      <header>
        <p className="eyebrow">{t("lineScan.eyebrow")}</p>
        <h2>{t("lineScan.title")}</h2>
        <p>{t("lineScan.readOnly")}</p>
      </header>

      <div className="line-scan-layout">
        <form className="line-scan-config" onSubmit={(event) => event.preventDefault()}>
          <label>{t("lineScan.gateway")}<input value={gateway} onChange={(event) => { setGateway(event.target.value); setEstimate(null); }} placeholder="192.0.2.10:3671" /></label>
          <label>{t("lineScan.area")}<input type="number" min="0" max="15" value={area} onChange={(event) => changeNumber(setArea, event.target.value)} /></label>
          <label>{t("lineScan.line")}<input type="number" min="0" max="15" value={line} onChange={(event) => changeNumber(setLine, event.target.value)} /></label>
          <label>{t("lineScan.firstDevice")}<input type="number" min="1" max="255" value={firstDevice} onChange={(event) => changeNumber(setFirstDevice, event.target.value)} /></label>
          <label>{t("lineScan.lastDevice")}<input type="number" min="1" max="255" value={lastDevice} onChange={(event) => changeNumber(setLastDevice, event.target.value)} /></label>
          <label>{t("lineScan.timeout")}<input type="number" min="1" value={responseTimeoutMs} onChange={(event) => changeNumber(setResponseTimeoutMs, event.target.value)} /></label>
          <label>{t("lineScan.pause")}<input type="number" min="0" value={interProbePauseMs} onChange={(event) => changeNumber(setInterProbePauseMs, event.target.value)} /></label>
          <div className="line-scan-actions">
            <button onClick={() => void preview()}>{t("lineScan.estimate")}</button>
        <button className="line-scan-start primary-action" disabled={!estimate || estimate.requestFingerprint !== requestFingerprint || controlsLocked || gateway.trim() === ""} onClick={() => void start()}>{t("lineScan.start")}</button>
        {response?.status === "running" && <button disabled={cancelling} onClick={() => void cancel()}>{t("lineScan.cancel")}</button>}
          </div>
        </form>

        <aside className="line-scan-exclusions">
          <h3>{t("lineScan.exclusions")}</h3>
      {displayedExclusions.length === 0 ? <p>{t("lineScan.noExclusions")}</p> : <ul>{displayedExclusions.map((address) => <li key={address}><code>{address}</code><span>{t("lineScan.protected")}</span><button disabled={controlsLocked} onClick={() => removeExclusion(address)}>{confirmRemoval === address ? t("lineScan.confirmRemoval") : t("lineScan.remove")}</button></li>)}</ul>}
      <div className="line-scan-add-exclusion"><input disabled={controlsLocked} aria-label={t("lineScan.exclusionAddress")} value={newExclusion} onChange={(event) => setNewExclusion(event.target.value)} placeholder="2.3.42" /><button disabled={controlsLocked} onClick={addExclusion}>{t("lineScan.addExclusion")}</button></div>
        </aside>
      </div>

      {estimate && <section className="line-scan-estimate" aria-live="polite"><h3>{t("lineScan.estimateTitle")}</h3><p>{t("lineScan.candidates", { count: estimate.value.candidateCount })}</p><p>{t("lineScan.basis", { count: estimate.value.vacantConfirmations, timeout: estimate.value.responseTimeoutMs, confirmations: estimate.value.vacantConfirmations, pause: estimate.value.interProbePauseMs })}</p><strong>{t("lineScan.worstCase", { duration: seconds(estimate.value.worstCaseMs) })}</strong></section>}
      {error && <p className="form-error" role="alert">{error}</p>}
      {response && <section className="line-scan-progress" aria-live="polite"><p>{t(statusKey(response.status))} · {response.completedCount}/{response.totalCount}</p><progress max={response.totalCount} value={response.completedCount} />{response.error && <p className="form-error">{response.error}</p>}</section>}
      {results.length > 0 && <table className="line-scan-results"><thead><tr><th>{t("lineScan.address")}</th><th>{t("lineScan.outcome")}</th></tr></thead><tbody>{results.map((result) => <tr key={result.address}><td><code>{result.address}</code></td><td><span data-scan-outcome={result.outcome.kind}>{outcomeLabel(result.outcome, t)}</span></td></tr>)}</tbody></table>}
    </section>
  );
}
