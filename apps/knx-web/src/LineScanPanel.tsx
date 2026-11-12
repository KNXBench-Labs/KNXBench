/** Read-only line diagnostics with cost preview, protected exclusions and incremental results. */

import ResizableTable from "./ResizableTable";
import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import DeviceLink from "./DeviceLink";
import { emitAchievementEvent } from "./achievementEvents";
import { loadPreferredGateway } from "./gatewayPreference";
import { type Translate, useTranslate } from "./i18n";
import type { IpTunnelTransport } from "./serialTransport";
import ExperimentalTransportBadge from "./ExperimentalTransportBadge";
import LineScanExclusionsEditor from "./LineScanExclusionsEditor";
import {
  hasInvalidLineScanExclusions,
  useLineScanExclusions,
} from "./lineScanExclusions";
import { useSettingsState } from "./settingsStore";
import type { ProjectTree } from "./bindings/ProjectTree";

const LINE_SCAN_COLUMNS = [
  { id: "address", width: 160, min: 100 },
  { id: "outcome", width: 320, min: 120 }
];

const POLL_INTERVAL_MS = 1000;

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

interface LineScanPanelProps {
  projectOpen?: boolean;
  projectRevision?: ProjectTree | null;
  onTreeUpdate?: (tree: ProjectTree) => void | Promise<void>;
}

export default function LineScanPanel({
  projectOpen = true,
  projectRevision,
  onTreeUpdate,
}: LineScanPanelProps = {}) {
  const t = useTranslate();
  const exclusions = useLineScanExclusions();
  const settingsState = useSettingsState();
  const [gateway, setGateway] = useState(loadPreferredGateway);
  const gatewayTouchedRef = useRef(false);
  const gatewaySeedResolvedRef = useRef(settingsState.hydration === "hydrated");
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
  const [starting, setStarting] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [activeExclusions, setActiveExclusions] = useState<string[]>([]);
  const [comparison, setComparison] = useState<api.LineScanComparison | null>(null);
  const [comparisonLoading, setComparisonLoading] = useState(false);
  const [comparisonRevision, setComparisonRevision] = useState(0);
  const [selectedUnexpected, setSelectedUnexpected] = useState<string[]>([]);
  const [selectedMissing, setSelectedMissing] = useState<string[]>([]);
  const [reconciling, setReconciling] = useState(false);
  const [transport, setTransport] = useState<IpTunnelTransport>("udp");
  const [authorityUnknown, setAuthorityUnknown] = useState(false);
  const sinceRef = useRef(0);
  const pollGenerationRef = useRef(0);
  const pollInFlightRef = useRef<symbol | null>(null);
  const identityRef = useRef<(api.BoundBusSession & { transport: IpTunnelTransport }) | null>(null);
  const mountedRef = useRef(true);
  const pendingStartRef = useRef<{ cancel: boolean } | null>(null);
  const request = useMemo<api.LineScanRequest>(() => ({
    gateway, area, line, firstDevice, lastDevice, excluded: exclusions,
    responseTimeoutMs, interProbePauseMs,
  }), [gateway, area, line, firstDevice, lastDevice, exclusions.join("\u0000"), responseTimeoutMs, interProbePauseMs]);
  const requestFingerprint = JSON.stringify({ request, transport });
  const requestFingerprintRef = useRef(requestFingerprint);
  requestFingerprintRef.current = requestFingerprint;
  const controlsLocked = starting || cancelling || reconciling || authorityUnknown || response?.status === "running";
  const hasInvalidExclusions = hasInvalidLineScanExclusions(exclusions);
  function sameIdentity(left: api.BoundBusSession | null, right: api.BoundBusSession | null) {
    return left === right || (left !== null && right !== null
      && left.sessionId === right.sessionId && left.serverIncarnation === right.serverIncarnation);
  }
  function current(generation: number, identity: api.BoundBusSession | null) {
    return mountedRef.current && generation === pollGenerationRef.current && sameIdentity(identityRef.current, identity);
  }
  useEffect(() => {
    if (settingsState.hydration !== "hydrated" || gatewaySeedResolvedRef.current) return;
    if (!gatewayTouchedRef.current && identityRef.current === null && estimate === null) setGateway(loadPreferredGateway());
    gatewaySeedResolvedRef.current = true;
  }, [settingsState.hydration, estimate]);
  function changeNumber(setter: (value: number) => void, value: string) {
    setter(Number(value)); setEstimate(null);
  }
  const reportedScanRef = useRef<string | null>(null);
  function apply(next: api.LineScanResultsResponse, replace = false) {
    identityRef.current = { sessionId: next.sessionId, serverIncarnation: next.serverIncarnation, transport: next.transport };
    const key = `${next.serverIncarnation}:${next.sessionId}`;
    if (next.status === "completed" && reportedScanRef.current !== key) {
      reportedScanRef.current = key;
      emitAchievementEvent({ type: "lineScanCompleted" });
    }
    gatewaySeedResolvedRef.current = true;
    sinceRef.current = next.nextSince;
    setResponse(next); setAuthorityUnknown(false);
    setActiveExclusions(next.excludedAddresses);
    setResults(rows => replace ? next.results : [...rows, ...next.results]);
  }
  async function poll(since: number, generation = pollGenerationRef.current, identity = identityRef.current) {
    if (pollInFlightRef.current) return;
    const flight = Symbol(); pollInFlightRef.current = flight;
    try {
      const next = await api.pollLineScan(since, identity ?? undefined);
      if (current(generation, identity)) apply(next);
    } catch (reason) {
      if (!current(generation, identity)) return;
      if (api.errorStatus(reason) !== 404 || identity !== null) {
        setAuthorityUnknown(true); setError(api.errorMessage(reason));
      }
    } finally {
      if (pollInFlightRef.current === flight) pollInFlightRef.current = null;
    }
  }
  useEffect(() => {
    mountedRef.current = true;
    void poll(0);
    return () => {
      mountedRef.current = false;
      // Keep an in-flight read attached across React's setup/cleanup replay.
      // A true detach remains fenced by this instance's mounted ref; remount gets new refs.
      // Navigation is not cancellation: the server owns the retained scan.
    };
  }, []);
  useEffect(() => {
    if (response?.status !== "running" || cancelling || authorityUnknown) return;
    const generation = pollGenerationRef.current;
    const identity = identityRef.current;
    const interval = window.setInterval(() => void poll(sinceRef.current, generation, identity), POLL_INTERVAL_MS);
    return () => window.clearInterval(interval);
  }, [response?.status, response?.sessionId, response?.serverIncarnation, cancelling, authorityUnknown]);
  useEffect(() => {
    if (response?.status !== "completed" || !projectOpen || authorityUnknown) { setComparison(null); return; }
    let active = true;
    const generation = pollGenerationRef.current;
    const identity = identityRef.current;
    if (!identity) return;
    setComparisonLoading(true); setSelectedUnexpected([]); setSelectedMissing([]);
    void api.compareLineScan(identity).then(value => {
      if (active && current(generation, identity)) setComparison(value);
    }).catch(reason => {
      if (active && current(generation, identity)) setError(api.errorMessage(reason));
    }).finally(() => {
      if (active && current(generation, identity)) setComparisonLoading(false);
    });
    return () => { active = false; };
  }, [response?.status, response?.sessionId, response?.serverIncarnation, projectOpen, projectRevision, comparisonRevision, authorityUnknown]);
  async function preview() {
    gatewaySeedResolvedRef.current = true; setError(null);
    const fingerprint = requestFingerprint;
    try {
      const value = await api.estimateLineScan(request);
      if (mountedRef.current && requestFingerprintRef.current === fingerprint) setEstimate({ value, requestFingerprint: fingerprint });
    } catch (reason) {
      if (mountedRef.current && requestFingerprintRef.current === fingerprint) setError(api.errorMessage(reason));
    }
  }
  async function settleCancel(identity: api.BoundBusSession, generation: number) {
    try {
      const cancelled = await api.cancelLineScan(identity);
      if (current(generation, identity)) apply(cancelled, true);
    } catch (reason) {
      if (current(generation, identity)) { setAuthorityUnknown(true); setError(api.errorMessage(reason)); }
    } finally {
      if (current(generation, identity)) setCancelling(false);
    }
  }
  async function start() {
    if (pendingStartRef.current || controlsLocked || !estimate || estimate.requestFingerprint !== requestFingerprint) return;
    const generation = ++pollGenerationRef.current;
    const pending = { cancel: false };
    pendingStartRef.current = pending;
    identityRef.current = null;
    pollInFlightRef.current = null;
    gatewaySeedResolvedRef.current = true;
    setStarting(true); setActiveExclusions(request.excluded); setError(null);
    setResults([]); setComparison(null); setSelectedUnexpected([]); setSelectedMissing([]); sinceRef.current = 0;
    try {
      const started = await api.startLineScan(request, transport);
      const identity = { sessionId: started.sessionId, serverIncarnation: started.serverIncarnation, transport: started.transport };
      // Retain accepted ownership synchronously, before React can render or detach.
      // Once the pair is known, Cancel must not wait for the first attached poll.
      if (pendingStartRef.current === pending) pendingStartRef.current = null;
      if (current(generation, null)) {
        identityRef.current = identity;
        setStarting(false);
        setResponse({ ...started, status: "running", error: null, cleanupError: null,
          disconnectConfirmed: null, nextSince: 0, completedCount: 0,
          totalCount: started.estimate.candidateCount, omittedAddresses: started.estimate.omittedAddresses,
          excludedAddresses: request.excluded, results: [] });
      }
      // Explicit pending intent survives detach and never borrows a newer identity.
      if (pending.cancel) await settleCancel(identity, generation);
      else if (current(generation, identity)) await poll(0, generation, identity);
    } catch (reason) {
      if (current(generation, null)) {
        // These validation/unsupported-route refusals open no scan. A lost or
        // malformed success and every other failure keep uncertain ownership.
        const status = api.errorStatus(reason);
        setAuthorityUnknown(status !== 400 && status !== 404 && status !== 405);
        setError(transport === "tcp" && [404, 405].includes(api.errorStatus(reason) ?? 0)
          ? t("lineScan.tcpUnsupported") : api.errorMessage(reason));
      }
    } finally {
      if (pendingStartRef.current === pending) {
        pendingStartRef.current = null;
        if (mountedRef.current && generation === pollGenerationRef.current) { setStarting(false); setCancelling(false); }
      }
    }
  }
  async function cancel() {
    if (pendingStartRef.current) {
      pendingStartRef.current.cancel = true; setCancelling(true); return;
    }
    const identity = identityRef.current;
    if (!identity || cancelling || authorityUnknown) return;
    const generation = ++pollGenerationRef.current;
    setCancelling(true); setError(null);
    await settleCancel(identity, generation);
  }
  function toggleSelection(address: string, selected: string[], setSelected: (value: string[]) => void) {
    setSelected(selected.includes(address) ? selected.filter(candidate => candidate !== address) : [...selected, address]);
  }
  async function reconcile() {
    const identity = identityRef.current;
    const generation = pollGenerationRef.current;
    if (!identity || authorityUnknown || selectedUnexpected.length + selectedMissing.length === 0) return;
    setReconciling(true); setError(null);
    try {
      const tree = await api.reconcileLineScan(identity, selectedUnexpected, selectedMissing);
      if (!current(generation, identity)) return;
      await onTreeUpdate?.(tree);
      if (!current(generation, identity)) return;
      setSelectedUnexpected([]); setSelectedMissing([]); setComparisonRevision(revision => revision + 1);
    } catch (reason) {
      if (current(generation, identity)) setError(api.errorMessage(reason));
    } finally {
      if (current(generation, identity)) setReconciling(false);
    }
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
          <label>{t("lineScan.transport")}<select disabled={controlsLocked} value={transport} onChange={event => { setTransport(event.target.value === "tcp" ? "tcp" : "udp"); setEstimate(null); }}><option value="udp">UDP</option><option value="tcp">{t("busTransport.ipTunnel.tcp")}</option></select></label>
          {transport === "tcp" && <p role="note" className="line-scan-experimental">{t("transport.experimentalNote")}</p>}
        <label>{t("lineScan.gateway")}<input disabled={controlsLocked} value={gateway} onChange={(event) => { gatewayTouchedRef.current = true; gatewaySeedResolvedRef.current = true; setGateway(event.target.value); setEstimate(null); }} placeholder="192.0.2.10:3671" /></label>
          <label>{t("lineScan.area")}<input disabled={controlsLocked} type="number" min="0" max="15" value={area} onChange={(event) => changeNumber(setArea, event.target.value)} /></label>
          <label>{t("lineScan.line")}<input disabled={controlsLocked} type="number" min="0" max="15" value={line} onChange={(event) => changeNumber(setLine, event.target.value)} /></label>
          <label>{t("lineScan.firstDevice")}<input disabled={controlsLocked} type="number" min="1" max="255" value={firstDevice} onChange={(event) => changeNumber(setFirstDevice, event.target.value)} /></label>
          <label>{t("lineScan.lastDevice")}<input disabled={controlsLocked} type="number" min="1" max="255" value={lastDevice} onChange={(event) => changeNumber(setLastDevice, event.target.value)} /></label>
          <label>{t("lineScan.timeout")}<input disabled={controlsLocked} type="number" min="1" value={responseTimeoutMs} onChange={(event) => changeNumber(setResponseTimeoutMs, event.target.value)} /></label>
          <label>{t("lineScan.pause")}<input disabled={controlsLocked} type="number" min="0" value={interProbePauseMs} onChange={(event) => changeNumber(setInterProbePauseMs, event.target.value)} /></label>
          <div className="line-scan-actions">
            <button disabled={hasInvalidExclusions} onClick={() => void preview()}>{t("lineScan.estimate")}</button>
        <button className="line-scan-start primary-action" disabled={!estimate || estimate.requestFingerprint !== requestFingerprint || controlsLocked || gateway.trim() === "" || hasInvalidExclusions} onClick={() => void start()}>{t("lineScan.start")}</button>
        {(starting || response?.status === "running") && <button disabled={cancelling || authorityUnknown} onClick={() => void cancel()}>{t("lineScan.cancel")}</button>}
          </div>
        </form>

        <LineScanExclusionsEditor disabled={controlsLocked} values={controlsLocked ? activeExclusions : undefined} />
      </div>

      {hasInvalidExclusions && <p className="form-error" role="alert">{t("lineScan.invalidExclusionsBlock")}</p>}

      {estimate && <section className="line-scan-estimate" aria-live="polite"><h3>{t("lineScan.estimateTitle")}</h3><p>{t("lineScan.candidates", { count: estimate.value.candidateCount })}</p><p>{t("lineScan.basis", { count: estimate.value.vacantConfirmations, timeout: estimate.value.responseTimeoutMs, confirmations: estimate.value.vacantConfirmations, pause: estimate.value.interProbePauseMs })}</p><strong>{t("lineScan.worstCase", { duration: seconds(estimate.value.worstCaseMs) })}</strong></section>}
      {error && <p className="form-error" role="alert">{error}</p>}
      {response && <section className="line-scan-progress" aria-live="polite"><p><span className={response.status === "running" ? "task-status-text" : undefined}>{t(statusKey(response.status))}</span> · {response.completedCount}/{response.totalCount}</p><progress max={response.totalCount} value={response.completedCount} />{response.error && <p className="form-error">{response.error}</p>}</section>}
      {response && <p>{t("lineScan.actualTransport", { transport: response.transport.toUpperCase() })}{response.transport === "tcp" && <ExperimentalTransportBadge />}</p>}
      {response?.skippedFrames !== null && response?.skippedFrames !== undefined && response.skippedFrames > 0 && <p role="status">{t("lineScan.skippedFrames", { count: response.skippedFrames })}</p>}
      {response?.transport === "tcp" && response.status !== "running" && <p>{t("lineScan.skipSample")}</p>}
      {response?.disconnectConfirmed === false && <p role="alert" className="form-error">{t("lineScan.cleanupUnconfirmed", { error: response.cleanupError ?? "" })}</p>}
      {results.length > 0 && <ResizableTable tableId="line-scan" columns={LINE_SCAN_COLUMNS} className="line-scan-results"><thead><tr><th>{t("lineScan.address")}</th><th>{t("lineScan.outcome")}</th></tr></thead><tbody>{results.map((result) => <tr key={result.address}><td><DeviceLink address={result.address}><code>{result.address}</code></DeviceLink></td><td><span data-scan-outcome={result.outcome.kind}>{outcomeLabel(result.outcome, t)}</span></td></tr>)}</tbody></ResizableTable>}
      {response?.status === "completed" && !projectOpen && <p>{t("lineScan.reconciliation.projectRequired")}</p>}
      {response?.status === "completed" && projectOpen && comparisonLoading && <p><span className="task-status-text">{t("lineScan.reconciliation.loading")}</span></p>}
      {response?.status === "completed" && projectOpen && comparison && <section className="line-scan-reconciliation" aria-live="polite">
        <h3>{t("lineScan.reconciliation.title")}</h3>
        <div className="line-scan-reconciliation-group">
          <h4>{t("lineScan.reconciliation.unexpected")}</h4>
          <p>{t("lineScan.reconciliation.unexpectedHelp")}</p>
          {comparison.unexpected.length === 0 ? <p>{t("lineScan.reconciliation.none")}</p> : <ul>{comparison.unexpected.map((address) => <li key={address}><label><input type="checkbox" checked={selectedUnexpected.includes(address)} onChange={() => toggleSelection(address, selectedUnexpected, setSelectedUnexpected)} /><code>{address}</code></label></li>)}</ul>}
        </div>
        <div className="line-scan-reconciliation-group">
          <h4>{t("lineScan.reconciliation.missing")}</h4>
          <p>{t("lineScan.reconciliation.missingHelp")}</p>
          {comparison.missing.length === 0 ? <p>{t("lineScan.reconciliation.none")}</p> : <ul>{comparison.missing.map((address) => <li key={address}><label><input type="checkbox" checked={selectedMissing.includes(address)} onChange={() => toggleSelection(address, selectedMissing, setSelectedMissing)} /><code>{address}</code></label></li>)}</ul>}
        </div>
        <div className="line-scan-reconciliation-group line-scan-reconciliation-excluded">
          <h4>{t("lineScan.reconciliation.excluded")}</h4>
          <p>{t("lineScan.reconciliation.excludedHelp")}</p>
          {comparison.excludedInProject.length === 0 ? <p>{t("lineScan.reconciliation.none")}</p> : <ul>{comparison.excludedInProject.map((address) => <li key={address}><code>{address}</code></li>)}</ul>}
        </div>
        {/* Creating devices is routine; any selected missing device makes this a deletion. */}
        <button className={selectedMissing.length > 0 ? "danger-action" : "primary-action"} disabled={reconciling || selectedUnexpected.length + selectedMissing.length === 0} onClick={() => void reconcile()}>{t("lineScan.reconciliation.apply")}</button>
      </section>}
    </section>
  );
}
