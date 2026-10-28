/** Present server-owned experimental downloads and explicit original-state recovery. */
import { useEffect, useRef, useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import * as api from "./system2DownloadApi";
import { useTranslate } from "./i18n";
import { useProgrammingConsent } from "./useProgrammingConsent";
import { downloadLocalJson } from "./localJsonDownload";
import { getSystem2Snapshot, publishSystem2Status, publishSystem2Unavailable, refreshSystem2Status } from "./system2Status";
import "./System2DownloadPanel.css";

interface Props {
  project: ProjectTree | null; deviceId: number | null; gateway: string; enabled: boolean; ordinaryBusy: boolean;
  onBusyChange?: (busy: boolean) => void;
}
type Phrases = { confirmation: string; hypothesisConfirmation: string; downloadConfirmation: string; releaseConfirmation: string };
const EMPTY: Phrases = { confirmation: "", hypothesisConfirmation: "", downloadConfirmation: "", releaseConfirmation: "" };
const active = (s: api.System2Status | null) => s?.state === "running" || s?.state === "restoring";
// Only the server's confirmed "no mutation could have started" earns the nothing-written text.
const nothingWritten = (s: api.System2Status) => (s.state === "failed" || s.state === "cancelled") && !s.mutationMayHaveStarted;
// Only a failed run without typed metadata earns "not available"; nothing is inferred from message or refusal.
// The download's final-step cause belongs only to a transfer that stopped inside the final step;
// a recovery's own cause is shown when present and never attributed to the download.
const unfinishedVerification = (s: api.System2Status) => s.downloadResult === "transferCompleteVerificationUnfinished";
const finalStep = (s: api.System2Status, run: "download" | "restore") =>
  run === "download" && !unfinishedVerification(s) ? undefined : s.finalStepFailures.find((f) => f.run === run);
const finalStepVisible = (s: api.System2Status) => unfinishedVerification(s) || finalStep(s, "restore") !== undefined;
const unavailableRun = (s: api.System2Status): "download" | "restore" | null =>
  s.state === "failed" ? "download" : s.state === "restoreFailed" ? "restore" : null;
function sameTarget(a: { binding: api.System2Binding; address: string }, b: { binding: api.System2Binding; address: string }) {
  return a.address === b.address && a.binding.serverIncarnation === b.binding.serverIncarnation &&
    a.binding.projectIncarnation === b.binding.projectIncarnation && a.binding.projectRevision === b.binding.projectRevision && a.binding.deviceId === b.binding.deviceId;
}
function requireOriginal(next: api.System2Status, original: api.System2Status) {
  if (next.operationId !== original.operationId || !sameTarget(next, original)) throw new Error("Mismatched System-2 operation response");
}
export default function System2DownloadPanel({ project, deviceId, gateway, enabled, ordinaryBusy, onBusyChange }: Props) {
  const t = useTranslate(); const consent = useProgrammingConsent();
  const [preview, setPreview] = useState<api.System2Preview | null>(null);
  const [status, setStatus] = useState<api.System2Status | null>(null);
  const statusRef = useRef<api.System2Status | null>(null);
  const [phrases, setPhrases] = useState<Phrases>(EMPTY);
  const [track, setTrack] = useState<api.RecoveryTrack | "">("");
  const [recovery, setRecovery] = useState({ recoveryConfirmation: "", downloadConfirmation: "", releaseConfirmation: "" });

  const [checking, setChecking] = useState(true); const [unavailable, setUnavailable] = useState(false);
  const [pending, setPending] = useState(false); const action = useRef(false);
  const [preparing, setPreparing] = useState(false);
  const [error, setError] = useState(false); const [unknown, setUnknown] = useState<"start" | "cancel" | "recover" | null>(null);
  const [startRefusal, setStartRefusal] = useState<string | null>(null);
  const [reportOutcome, setReportOutcome] = useState<{ id: string; kind: "saved" | "failed" } | null>(null);
  const reportBusy = useRef(false);
  const mounted = useRef(false); const generation = useRef(0); const reads = useRef(0); const pollInFlight = useRef(false);
  const context = useRef({ project, deviceId, gateway, enabled }); context.current = { project, deviceId, gateway, enabled };

  useEffect(() => {
    generation.current += 1; setPreview(null); setPhrases(EMPTY); setTrack(""); setPreparing(false); setError(false);
  }, [project, deviceId, gateway, enabled]);

  function accept(next: api.System2Status) {
    const prior = statusRef.current;
    if (active(prior) && prior?.operationId !== next.operationId) { setUnavailable(true); return; }
    if (prior?.operationId !== next.operationId || prior?.state !== next.state || prior?.recoveryPhrase !== next.recoveryPhrase ||
      prior?.downloadPhrase !== next.downloadPhrase || prior?.releasePhrase !== next.releasePhrase) {
      setRecovery({ recoveryConfirmation: "", downloadConfirmation: "", releaseConfirmation: "" });
    }
    statusRef.current = next; setStatus(next); setUnavailable(false); setChecking(false);
    // The shared lifetime: the device table reads what this single poll owner saw.
    publishSystem2Status(next);
  }
  async function poll() {
    if (pollInFlight.current || action.current) return;
    pollInFlight.current = true; const read = ++reads.current;
    try {
      await refreshSystem2Status();
      if (!mounted.current || read !== reads.current) return;
      const shared = getSystem2Snapshot();
      if (shared.failed || shared.authority === "unavailable") {
        setUnavailable(true); setChecking(false); return;
      }
      const next = shared.status;
      if (next === null) {
        // A disappeared record is not proof of rollback or permission to resend.
        setUnavailable(statusRef.current !== null); setChecking(false);
        if (statusRef.current !== null) publishSystem2Unavailable();
      } else accept(next);
    } catch {
      if (mounted.current && read === reads.current) { setUnavailable(true); setChecking(false); publishSystem2Unavailable(); }
    } finally { pollInFlight.current = false; }
  }
  useEffect(() => {
    mounted.current = true; void poll();
    const timer = window.setInterval(() => void poll(), 1000);
    return () => { mounted.current = false; generation.current += 1; reads.current += 1; window.clearInterval(timer); };
  }, []);
  const busy = pending || active(status) || checking || unavailable || unknown !== null;
  // Preserve ordinary-download availability when this optional endpoint is unavailable.
  // Both write routes still require the server’s shared admission guard.
  const ordinaryBlocked = pending || active(status) || unknown !== null;
  useEffect(() => { onBusyChange?.(ordinaryBlocked); }, [ordinaryBlocked, onBusyChange]);

  async function prepare() {
    if (deviceId === null || project === null || active(status) || action.current || checking || unavailable || ordinaryBusy) return;
    const original = context.current; const ticket = ++generation.current;
    setPreparing(true); setPreview(null); setPhrases(EMPTY); setTrack(""); setError(false);
    try {
      const next = await api.prepareSystem2(deviceId);
      if (!mounted.current || ticket !== generation.current || context.current.project !== original.project || context.current.deviceId !== original.deviceId || !context.current.enabled) return;
      // Stable selection is checked locally too; revision/source authority remains server-side.
      if (next.binding.deviceId !== original.deviceId ||
        (project.server_incarnation !== undefined && next.binding.serverIncarnation !== project.server_incarnation) ||
        (project.project_incarnation !== undefined && next.binding.projectIncarnation !== String(project.project_incarnation))) {
        setError(true); return;
      }
      setPreview(next); setUnknown(null);
    } catch { if (mounted.current && ticket === generation.current) setError(true); }
    finally { if (mounted.current && ticket === generation.current) setPreparing(false); }
  }
  const expected: Phrases | null = preview === null ? null : {
    confirmation: preview.confirmationPhrase, hypothesisConfirmation: preview.hypothesisPhrase,
    downloadConfirmation: preview.downloadPhrase, releaseConfirmation: preview.releasePhrase,
  };
  const exact = expected !== null && Object.keys(expected).every((key) => phrases[key as keyof Phrases] === expected[key as keyof Phrases]);
  const canStart = enabled && exact && track !== "" && gateway.trim() !== "" && !busy && !ordinaryBusy && !preparing;

  async function start() {
    if (!canStart || preview === null || action.current) return;
    const shown = preview; const original = context.current; const ticket = generation.current;
    const body: api.System2Start = { preparationId: shown.preparationId, gateway: gateway.trim(), ...phrases, recoveryTrack: track };
    action.current = true; setPending(true); setError(false); setStartRefusal(null); reads.current += 1;
    try {
      if (!(await consent.request(`${shown.address} — ${shown.deviceName}`))) return;
      if (!mounted.current || ticket !== generation.current || context.current.project !== original.project || context.current.deviceId !== original.deviceId || context.current.gateway !== original.gateway || !context.current.enabled) return;
      // Drop local execution capability before sending, even when its response is lost.
      setPreview(null); setPhrases(EMPTY); setTrack("");
      // A previous settled operation cannot stand in for status of this new start.
      statusRef.current = null; setStatus(null); publishSystem2Status(null);
      try {
        const next = await api.startSystem2(body);
        if (!sameTarget(next, shown)) throw new Error("Mismatched System-2 start response");
        if (mounted.current) accept(next);
      } catch (reason) {
        // A named refusal is shown, but the outcome stays unknown: never resent.
        if (mounted.current) { setUnknown("start"); setStartRefusal(api.refusalKind(reason)); }
      }
    } finally {
      action.current = false; reads.current += 1;
      if (mounted.current) { setPending(false); void poll(); }
    }
  }
  async function cancel() {
    const original = statusRef.current;
    if (!active(original) || original === null || action.current) return;
    action.current = true; setPending(true); reads.current += 1;
    try { const next = await api.cancelSystem2(original.operationId); requireOriginal(next, original); if (mounted.current) accept(next); }
    catch { if (mounted.current) setUnknown("cancel"); }
    finally { action.current = false; reads.current += 1; if (mounted.current) { setPending(false); void poll(); } }
  }
  // Local save only: the server composes a bounded journal-fact summary, nothing is uploaded.
  // An answer for an operation that is no longer the attached one is dropped.
  async function saveReport() {
    const original = statusRef.current;
    if (original === null || active(original) || reportBusy.current) return;
    reportBusy.current = true; setReportOutcome(null);
    const current = () => mounted.current && statusRef.current?.operationId === original.operationId && !active(statusRef.current);
    try {
      const report = await api.getSystem2Report(original.operationId);
      if (!current()) return;
      downloadLocalJson(JSON.stringify(report, null, 2), "knxbench-system2-report.json");
      setReportOutcome({ id: original.operationId, kind: "saved" });
    } catch { if (current()) setReportOutcome({ id: original.operationId, kind: "failed" }); }
    finally { reportBusy.current = false; }
  }
  // Only the server's original-operation status can supply recovery terms, including
  // after reattachment or a lost start response. A later preview is never a fallback.
  const originalTerms = status !== null && status.downloadPhrase !== null && status.releasePhrase !== null ?
    { download: status.downloadPhrase, release: status.releasePhrase } : null;
  // A lost start is never resent. A subsequent read-only attachment may offer a
  // separately acknowledged backup recovery; a lost recovery/cancel stays blocked.
  const canRecover = status !== null && !active(status) && status.backupAvailable && !status.prestateDiffers && status.recoveryPhrase !== null && originalTerms !== null &&
    recovery.recoveryConfirmation === status.recoveryPhrase && recovery.downloadConfirmation === originalTerms.download && recovery.releaseConfirmation === originalTerms.release &&
    gateway.trim() !== "" && !pending && !checking && !unavailable && !ordinaryBusy && (unknown === null || unknown === "start");
  async function recover() {
    if (!canRecover || status === null || action.current) return;
    const original = status; const originalGateway = gateway; const ticket = generation.current;
    const body: api.System2Recovery = { operationId: original.operationId, gateway: gateway.trim(), ...recovery };
    action.current = true; setPending(true); reads.current += 1;
    try {
      if (!(await consent.request(`${original.address} — ${original.deviceName}`))) return;
      if (!mounted.current || generation.current !== ticket || context.current.gateway !== originalGateway || statusRef.current !== original) return;
      setRecovery({ recoveryConfirmation: "", downloadConfirmation: "", releaseConfirmation: "" });
      try { const next = await api.recoverSystem2(body); requireOriginal(next, original); if (mounted.current) accept(next); }
      catch { if (mounted.current) setUnknown("recover"); }
    } finally { action.current = false; reads.current += 1; if (mounted.current) { setPending(false); void poll(); } }
  }
  // Keep polling while the ordinary mode is selected, without adding a second
  // download-plan surface until an actual operation exists (or consent is pending).
  if (!enabled && status === null && !pending) return null;
  return <section className="system2-download-panel device-download-plan" aria-label={t("system2.title")}>
    <h3>{t("system2.title")}</h3>
    <p className="form-warning">{t("system2.risk")}</p>
    <p>{t("system2.lifetime")}</p>
    {checking && <p role="status">{t("system2.checking")}</p>}
    {unavailable && <p role="alert" className="form-warning">{t("system2.statusUnavailable")}</p>}
    {unknown && <p data-system2-unknown role="alert" className="form-warning">{t("system2.unknown")}</p>}
    {unknown === "start" && startRefusal && <p data-system2-start-refusal>{t("system2.startRefused", { reason: startRefusal })}</p>}
    {error && <p role="alert" className="form-error">{t("system2.prepareFailed")}</p>}
    <button type="button" data-system2-refresh disabled={pending} onClick={() => void poll()}>{t("system2.refresh")}</button>
    {enabled && <>
      <p>{t("system2.configured")}</p>
      <button type="button" data-system2-prepare disabled={project === null || deviceId === null || pending || active(status) || checking || unavailable || ordinaryBusy || preparing} onClick={() => void prepare()}>{t("system2.prepare")}</button>
      {preparing && <p role="status">{t("system2.preparing")}</p>}
    </>}
    {enabled && preview && expected && <section data-system2-preview aria-label={t("system2.preview")}>
      <h4>{t("system2.preview")}</h4><p>{t("system2.offline")}</p>
      <dl>
        <dt>{t("deviceDownload.target")}</dt><dd>{preview.address} — {preview.deviceName}</dd>
        <dt>{t("deviceDownload.program")}</dt><dd>{preview.programId}</dd>
        <dt>{t("system2.profile")}</dt><dd>{preview.profileId}</dd>
        <dt>{t("system2.configuration")}</dt><dd style={{ overflowWrap: "anywhere" }}><code>{preview.configurationSha256}</code></dd>
        <dt>{t("system2.plan")}</dt><dd style={{ overflowWrap: "anywhere" }}><code>{preview.planSha256}</code></dd>
        <dt>{t("system2.counts")}</dt><dd>{t("system2.countsValue", { parameters: preview.parameterValues, links: preview.groupLinks, flags: preview.flagOverrides })}</dd>
        <dt>{t("system2.ranges")}</dt><dd>{t("system2.rangesValue", { data: preview.dataOctets, allocation: preview.allocationOctets, notWritten: preview.notWrittenOctets })}</dd>
      </dl>
      <h4>{t("system2.steps")}</h4><ol>{preview.steps.map((step, i) => <li key={i}>{step}</li>)}</ol>
      <h4>{t("deviceDownload.warnings.title")}</h4><ul>{preview.warnings.map((warning, i) => <li key={i}>{warning}</li>)}</ul>
      <p>{t("system2.backup")}</p>
      <fieldset data-system2-consent>
      <legend>{t("system2.track")}</legend>
      <label>{t("system2.track")}<select data-system2-track value={track} disabled={pending} onChange={(e) => setTrack(e.target.value as api.RecoveryTrack | "")}>
        <option value="">{t("system2.chooseTrack")}</option><option value="knxbenchBestEffort">{t("system2.track.knxbenchBestEffort")}</option><option value="externalETS">{t("system2.track.externalETS")}</option>
      </select></label><p>{t("system2.trackDetails")}</p>
      {Object.entries(expected).map(([key, phrase]) => <label className="device-download-acknowledgement" key={key}>
        {t(`system2.phrase.${key as keyof Phrases}`)} <code style={{ overflowWrap: "anywhere" }}>{phrase}</code>
        <input data-system2-phrase={key} autoComplete="off" spellCheck={false} disabled={pending} value={phrases[key as keyof Phrases]} onChange={(e) => setPhrases((p) => ({ ...p, [key]: e.target.value }))} />
      </label>)}
      <button type="button" data-system2-start disabled={!canStart} onClick={() => void start()}>{t("system2.start")}</button>
      </fieldset>
    </section>}
    {status && <section data-system2-status aria-live="polite">
      <h4>{t("system2.operation")}: {status.address} — {status.deviceName}</h4>
      <p style={{ overflowWrap: "anywhere" }}><code>{status.operationId}</code> · {t(nothingWritten(status) ? `system2.state.${status.state === "failed" ? "failed" : "cancelled"}BeforeWrite` : `system2.state.${status.state}`)}</p>
      {nothingWritten(status) && <p data-system2-nothing-written>{t("system2.nothingWritten")}</p>}
      <p>{status.totalSteps === null ? t("system2.checkpointsUnknown", { done: status.completedSteps }) : t("system2.checkpoints", { done: status.completedSteps, total: status.totalSteps })}</p>
      {status.totalSteps !== null && status.totalSteps > 0 && <progress aria-label={t("system2.steps")} value={status.completedSteps} max={status.totalSteps} />}
      {!active(status) && <section data-system2-result aria-label={t("system2.result.heading")}>
        <p data-system2-result-text data-system2-download-result={status.downloadResult}><strong>{t("system2.result.heading")}:</strong> {t(`system2.result.${status.downloadResult === "inProgress" ? "unknown" : status.downloadResult}`)}</p>
        <p>{t("system2.result.note")}</p>
        {finalStepVisible(status) && <section data-system2-final-steps aria-label={t("system2.finalStep.heading")}>
          <h5>{t("system2.finalStep.heading")}</h5><p>{t("system2.finalStep.note")}</p>
          <ul>
            {(["download", "restore"] as const).map((run) => {
              const f = finalStep(status, run);
              if (f) return <li key={run} data-system2-final-step={run} data-system2-final-category={f.category} data-system2-final-stage={f.stage}>
                {t(`system2.finalStep.${run}`, { category: t(`system2.finalStep.category.${f.category}`), stage: t(`system2.finalStep.stage.${f.stage}`) })}</li>;
              return run === "download" && unfinishedVerification(status) ? <li key={run} data-system2-final-step-unavailable={run}>{t("system2.finalStep.unavailable")}</li> : null;
            })}
          </ul>
        </section>}
        <p data-system2-settlement={status.projectSettlement}><strong>{t("system2.settlement.heading")}:</strong> {t(`system2.settlement.${status.projectSettlement}`)}</p>
        {status.restoreAttempted && <p data-system2-download-counts>{status.downloadTotalSteps === null ? t("system2.downloadCountsUnknown", { done: status.downloadCompletedSteps }) : t("system2.downloadCounts", { done: status.downloadCompletedSteps, total: status.downloadTotalSteps })}</p>}
        <button type="button" data-system2-report-save disabled={pending} onClick={() => void saveReport()}>{t("system2.report.save")}</button>
        <p data-system2-report-guidance>{t("system2.report.guidance")} <a href="mailto:issue@knxbench.com?subject=KNXBench%20System-2%20download%20report">issue@knxbench.com</a></p>
        {reportOutcome?.id === status.operationId && reportOutcome.kind === "saved" && <p role="status" data-system2-report-saved>{t("system2.report.saved")}</p>}
        {reportOutcome?.id === status.operationId && reportOutcome.kind === "failed" && <p role="alert" data-system2-report-failed className="form-warning">{t("system2.report.failed")}</p>}
      </section>}
      {status.message && <p>{status.message}</p>}
      {status.refusal && <p data-system2-refusal>{t("system2.refusal", { reason: status.refusal })}</p>}
      {status.prestateUnreadableAllocations.length > 0 ? <p data-system2-prestate-unreadable className="form-warning">
        {t("system2.prestateUnreadable", { allocations: status.prestateUnreadableAllocations.map((a) => t("system2.prestateUnreadableAllocation", { object: a.object, address: a.address, length: a.length })).join("; ") })}
      </p> : <p>{t(status.backupAvailable ? "system2.backupAvailable" : "system2.backupUnavailable")}</p>}
      {(status.pointerObservations.length > 0 || status.programVersionObservation !== null) && <section data-system2-observations aria-label={t("system2.observations")}>
        <h4>{t("system2.observations")}</h4><p>{t("system2.observationsNote")}</p>
        <ul>
          {status.programVersionObservation && <li>{t("system2.observation.programVersion", { expected: status.programVersionObservation.expected, observed: status.programVersionObservation.observed || t("system2.observation.refused") })}</li>}
          {status.pointerObservations.map((o, i) => <li key={i}>{o.run === "restore" && `${t("system2.observation.restoreRun")} `}{t(`system2.observation.${o.phase}`, { object: o.object, step: o.operationIndex ?? "", expected: o.expected, observed: o.observed === "refused" ? t("system2.observation.refused") : o.observed, state: o.loadState })}</li>)}
        </ul>
      </section>}
      {(status.failureDiagnostics.length > 0 || unavailableRun(status) !== null) && <section data-system2-diagnostics aria-label={t("system2.diagnostics")}>
        <h4>{t("system2.diagnostics")}</h4><p>{t("system2.diagnosticsNote")}</p>
        <ul>
          {(["download", "restore"] as const).map((run) => {
            const d = status.failureDiagnostics.find((row) => row.run === run);
            if (d) return <li key={run} data-system2-diagnostic={run}>{t("system2.diagnostic.entry", { run: t(`system2.diagnostic.run.${run}`), reason: t(`system2.diagnostic.reason.${d.reason}`), object: d.object, property: d.propertyId, elements: d.elements, octets: d.octets })}</li>;
            return unavailableRun(status) === run ? <li key={run} data-system2-diagnostic-unavailable={run}>{t("system2.diagnostic.unavailable", { run: t(`system2.diagnostic.run.${run}`) })}</li> : null;
          })}
        </ul>
      </section>}
      {active(status) && <button type="button" data-system2-cancel disabled={pending} onClick={() => void cancel()}>{t("system2.cancel")}</button>}
      <p>{t("system2.cancelNote")}</p>
      {!active(status) && status.prestateDiffers && <p data-system2-recovery-unavailable className="form-warning">{t("system2.recoveryUnavailablePrestate")}</p>}
      {!active(status) && status.backupAvailable && !status.prestateDiffers && <section aria-label={t("system2.recovery")}>
        <h4>{t("system2.recovery")}</h4><p>{t("system2.recoveryNote")}</p>
        {originalTerms === null ? <p className="form-warning">{t("system2.recoveryTermsUnavailable")}</p> : status.recoveryPhrase !== null && <>
          {Object.entries({ recoveryConfirmation: status.recoveryPhrase, downloadConfirmation: originalTerms.download, releaseConfirmation: originalTerms.release }).map(([key, phrase]) => <label key={key} className="device-download-acknowledgement">
            {t(key === "recoveryConfirmation" ? "system2.phrase.recoveryConfirmation" : key === "downloadConfirmation" ? "system2.phrase.downloadConfirmation" : "system2.phrase.releaseConfirmation")} <code>{phrase}</code>
            <input data-system2-recovery={key} autoComplete="off" spellCheck={false} disabled={pending || (unknown !== null && unknown !== "start")} value={recovery[key as keyof typeof recovery]} onChange={(e) => setRecovery((p) => ({ ...p, [key]: e.target.value }))} />
          </label>)}
          <button type="button" data-system2-recover disabled={!canRecover} onClick={() => void recover()}>{t("system2.recover")}</button>
        </>}
      </section>}
    </section>}
    {consent.dialog}
  </section>;
}
