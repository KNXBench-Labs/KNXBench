/** "Download all…": review every pending device, confirm the batch once, and watch each device. */
// ADR-0172. Nothing plans or connects until the user asks for the review.
// The review is the server's: which devices are in the batch, their plans and
// phrases, and why the others are not. The user may deselect devices and must
// acknowledge each `untested` one with its own phrase; start stays disabled
// until every selected `untested` device is acknowledged. The batch confirmation
// goes through `useProgrammingConsent().requestBatch`, which asks even when the
// stage's answer is remembered. The server runs the batch and stops it at the
// first doubt; this panel only polls and shows a result per device.

import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import { gatewayRefusalMessage } from "./gatewayRefusal";
import { planWarnings } from "./downloadVerification";
import { useTranslate, type MessageKey } from "./i18n";
import { useProgrammingConsent } from "./useProgrammingConsent";
import { projectContextKey } from "./treeUtils";
import type { ProjectTree } from "./bindings/ProjectTree";

export const BATCH_POLL_INTERVAL_MS = 500;

type InBatch = Extract<api.DeviceDownloadBatchReviewRow, { status: "inBatch" }>;

const REASON: Record<api.DeviceDownloadBatchExclusion["code"], MessageKey> = {
  noAddress: "deviceDownload.batch.reason.noAddress",
  excluded: "deviceDownload.batch.reason.excluded",
  nothingToDownload: "deviceDownload.batch.reason.nothingToDownload",
  addressPending: "deviceDownload.batch.reason.addressPending",
  system2: "deviceDownload.batch.reason.system2",
  unsupported: "deviceDownload.batch.reason.unsupported",
  noPostDownloadCheck: "deviceDownload.batch.reason.noPostDownloadCheck",
};

const NOT_STARTED: Record<api.DeviceDownloadBatchNotStarted, MessageKey> = {
  earlierDevice: "deviceDownload.batch.notStarted.earlierDevice",
  stopRequest: "deviceDownload.batch.notStarted.stopRequest",
  serverStop: "deviceDownload.batch.notStarted.serverStop",
  runEnded: "deviceDownload.batch.notStarted.runEnded",
};

const DOUBT: Record<api.DeviceDownloadBatchDoubt, MessageKey> = {
  notWritten: "deviceDownload.batch.doubt.notWritten",
  restartUnconfirmed: "deviceDownload.batch.doubt.restartUnconfirmed",
  checkNotChecked: "deviceDownload.batch.doubt.checkNotChecked",
  checkUnsupported: "deviceDownload.batch.doubt.checkUnsupported",
  checkNotPerformed: "deviceDownload.batch.doubt.checkNotPerformed",
  checkInterrupted: "deviceDownload.batch.doubt.checkInterrupted",
  checkNotReached: "deviceDownload.batch.doubt.checkNotReached",
  checkReachUnknown: "deviceDownload.batch.doubt.checkReachUnknown",
  identityUnavailable: "deviceDownload.batch.doubt.identityUnavailable",
  identityMismatch: "deviceDownload.batch.doubt.identityMismatch",
  loadStateDiffers: "deviceDownload.batch.doubt.loadStateDiffers",
  loadStateUnknown: "deviceDownload.batch.doubt.loadStateUnknown",
};

const WRITTEN: Record<api.DeviceDownloadWritten, MessageKey> = {
  yes: "deviceDownload.batch.written.yes",
  no: "deviceDownload.batch.written.no",
  partially: "deviceDownload.batch.written.partially",
};

function label(row: { address: string | null; name: string }): string {
  return row.address === null ? row.name : `${row.address} — ${row.name}`;
}

/** The exclusion as a sentence; its server detail is shown verbatim. */
export function reasonText(t: ReturnType<typeof useTranslate>, reason: api.DeviceDownloadBatchExclusion): string {
  return "detail" in reason ? t(REASON[reason.code], { detail: reason.detail }) : t(REASON[reason.code]);
}

/** One status row as a sentence. */
export function stateText(t: ReturnType<typeof useTranslate>, row: api.DeviceDownloadBatchDeviceState): string {
  switch (row.state) {
    case "waiting": return t("deviceDownload.batch.state.waiting");
    case "running": return t("deviceDownload.batch.state.running");
    case "done": return t(row.restart === "notInPlan" ? "deviceDownload.batch.state.doneNoRestart" : "deviceDownload.batch.state.done");
    case "unconfirmed": return t(row.doubt === "restartUnconfirmed" ? "deviceDownload.batch.state.restartUnconfirmed"
      : row.doubt === "notWritten" ? "deviceDownload.batch.state.notWritten"
        : "deviceDownload.batch.state.checkNotConfirmed", { doubt: t(DOUBT[row.doubt]) });
    case "failed":
      // A start refused before its download ran names its kind (nothing was
      // written then); the rest keep the server's text and say what was written.
      if (row.written === "no" && (row.kind === "gatewayTimeout" || row.kind === "gatewayRefused")) return t("deviceDownload.batch.state.gatewayFailed");
      if (row.written === "no" && row.kind === "downloadPlanStale") return t("deviceDownload.batch.state.planChanged");
      return `${t("deviceDownload.batch.state.failed", { error: row.error })}. ${t(WRITTEN[row.written])}`;
    case "stopped": return t("deviceDownload.batch.state.stopped");
    case "notStarted": return t(NOT_STARTED[row.reason]);
    case "notInBatch": return reasonText(t, row.reason);
    case "deselected": return t("deviceDownload.batch.state.deselected");
  }
}

/** What a failed or stopped row adds: the server's hint and the kept backup. */
function stateDetails(t: ReturnType<typeof useTranslate>, row: api.DeviceDownloadBatchDeviceState): string[] {
  const details: string[] = [];
  if (row.state === "failed" && row.hint) details.push(row.hint);
  if ((row.state === "failed" || row.state === "stopped") && row.backupFile) details.push(t("deviceDownload.backupKept", { file: row.backupFile }));
  return details;
}

function errorKind(reason: unknown): string | undefined {
  const body = (reason as { body?: unknown } | null)?.body;
  return typeof body === "object" && body !== null && typeof (body as { kind?: unknown }).kind === "string"
    ? (body as { kind: string }).kind : undefined;
}

export default function DeviceDownloadBatch({ project, gateway, disabled, openRequest, onOpenHandled, onBusyChange, onProjectChanged }: {
  project: ProjectTree;
  /** The download tab's gateway; the batch uses the same one for every device. */
  gateway: string;
  /** Another download on this page is being planned, started or running. */
  disabled: boolean;
  /** A "Download all…" request from the pending block; reviewed once per nonce. */
  openRequest: number | null;
  onOpenHandled?: (nonce: number) => void;
  /** `true` while a batch is starting or running: the single download waits. */
  onBusyChange?: (busy: boolean) => void;
  /** The project as reloaded after a batch ended (ADR-0129 settlement). */
  onProjectChanged?: (tree: ProjectTree) => void;
}) {
  const t = useTranslate();
  const consent = useProgrammingConsent();
  const [review, setReview] = useState<api.DeviceDownloadBatchReview | null>(null);
  const [reviewing, setReviewing] = useState(false);
  const [excluded, setExcluded] = useState<Set<number>>(new Set());
  const [acknowledged, setAcknowledged] = useState<Record<number, string>>({});
  const [starting, setStarting] = useState(false);
  const [status, setStatus] = useState<api.DeviceDownloadBatchStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const request = useRef(0);
  const reloaded = useRef<number | null>(null);
  const handled = useRef<number | null>(null);
  const running = status?.state === "running";
  const busy = starting || running;

  useEffect(() => { onBusyChange?.(busy); }, [busy, onBusyChange]);
  // A review belongs to the project state it was prepared from; a reload of
  // the same state (same incarnation and revision) keeps it.
  // Compared, not merely re-run: a re-run effect (React's strict mode) must not
  // discard a review that is being prepared for the same state.
  const projectState = `${projectContextKey(project)}#${project.snapshot_revision}`;
  const reviewedState = useRef(projectState);
  useEffect(() => {
    if (reviewedState.current === projectState) return;
    reviewedState.current = projectState;
    dropReview();
  }, [projectState]);

  function dropReview() {
    request.current += 1;
    setReview(null);
    setReviewing(false);
    setExcluded(new Set());
    setAcknowledged({});
  }

  async function prepareReview() {
    dropReview();
    setError(null);
    setReviewing(true);
    const mine = request.current;
    try {
      const next = await api.planDeviceDownloadBatch();
      if (mine !== request.current) return;
      setReview(next);
    } catch (reason) {
      if (mine === request.current) setError(api.errorMessage(reason));
    } finally {
      if (mine === request.current) setReviewing(false);
    }
  }

  // A request waits while another download on this page is busy (it may only
  // be loading its status); a running batch answers it by its status alone.
  useEffect(() => {
    if (openRequest === null || handled.current === openRequest || disabled) return;
    handled.current = openRequest;
    onOpenHandled?.(openRequest);
    if (!busy) void prepareReview();
  }, [openRequest, busy, disabled, onOpenHandled]);

  function apply(next: api.DeviceDownloadBatchStatus) {
    setStatus(next);
    if (next.state === "ended" && reloaded.current !== next.batchId) {
      reloaded.current = next.batchId;
      setRefreshError(null);
      if (onProjectChanged) api.currentProject().then(onProjectChanged, (reason) => setRefreshError(api.errorMessage(reason)));
    }
  }

  async function poll(batchId?: number) {
    try {
      apply(await api.pollDeviceDownloadBatch(batchId));
    } catch (reason) {
      if (api.errorStatus(reason) !== 404) { setError(api.errorMessage(reason)); return; }
      if (batchId === undefined) return;
      // Our batch was replaced (another page started one): show whichever is
      // current now, or none, so the panel does not stay "running" forever.
      try {
        const current = await api.pollDeviceDownloadBatch();
        if (current.state === "ended") reloaded.current = current.batchId;
        setStatus(current);
      } catch {
        setStatus(null);
      }
    }
  }

  // A batch keeps running without this page; show it when the page opens.
  useEffect(() => {
    let current = true;
    void (async () => {
      try {
        const found = await api.pollDeviceDownloadBatch();
        // An ended batch from earlier is not reloaded into the project again.
        if (current) { if (found.state === "ended") reloaded.current = found.batchId; setStatus(found); }
      } catch { /* none yet, or unreadable: nothing to show */ }
    })();
    return () => { current = false; };
  }, []);

  useEffect(() => {
    if (!running || status === null) return;
    const batchId = status.batchId;
    const interval = window.setInterval(() => void poll(batchId), BATCH_POLL_INTERVAL_MS);
    return () => window.clearInterval(interval);
  }, [running, status?.batchId]);

  const inBatch = (review?.devices ?? []).filter((row): row is InBatch => row.status === "inBatch");
  const selected = inBatch.filter((row) => !excluded.has(row.planId));
  const missingAcknowledgement = selected.filter((row) =>
    row.untestedAcknowledgement !== null && acknowledged[row.planId] !== row.untestedAcknowledgement);
  const blocker: MessageKey | null = gateway.trim() === "" ? "deviceDownload.batch.blocked.gateway"
    : selected.length === 0 ? "deviceDownload.batch.blocked.none"
      : missingAcknowledgement.length > 0 ? "deviceDownload.batch.blocked.acknowledgement" : null;

  async function start() {
    if (review === null || blocker !== null || busy || disabled) return;
    const shown = review;
    const devices = selected;
    const mine = request.current;
    setError(null);
    if (!(await consent.requestBatch(devices.map(label)))) return;
    // The review was dropped (project edit, close) while the question was open.
    if (mine !== request.current) return;
    setStarting(true);
    try {
      // The phrases are built only now, after the batch request said yes.
      const entries: api.DeviceDownloadBatchStartDevice[] = devices.map((row) => {
        const entry: api.DeviceDownloadBatchStartDevice = { address: row.address as string, confirmation: row.confirmationPhrase };
        if (row.untestedAcknowledgement !== null) entry.acceptUntested = acknowledged[row.planId];
        return entry;
      });
      const { batchId } = await api.startDeviceDownloadBatch(shown.batchId, gateway.trim(), entries);
      setReview(null);
      await poll(batchId);
    } catch (reason) {
      if (api.errorStatus(reason) === 409 && errorKind(reason) === "downloadPlanStale") {
        // A single plan or a newer review replaced this one on the server.
        dropReview();
        setError(t("deviceDownload.batch.reviewReplaced"));
        return;
      }
      setError(gatewayRefusalMessage(reason) ?? api.errorMessage(reason));
    } finally {
      setStarting(false);
    }
  }

  async function stop() {
    if (status === null) return;
    try {
      await api.stopDeviceDownloadBatch(status.batchId);
      await poll(status.batchId);
    } catch (reason) {
      setError(api.errorMessage(reason));
    }
  }

  const position = status === null ? 0 : status.devices.filter((row) => "downloadId" in row).findIndex((row) => row.state === "running") + 1;
  const total = status === null ? 0 : status.devices.filter((row) => "downloadId" in row).length;

  return (
    <section className="device-download-batch" aria-label={t("deviceDownload.batch.title")}>
      <h3>{t("deviceDownload.batch.title")}</h3>
      {review === null && !reviewing && (
        <div className="device-download-actions">
          <button type="button" data-batch-open disabled={busy || disabled} onClick={() => void prepareReview()}>
            {t("deviceDownload.batch.open")}</button>
          <span className="device-download-actions-note">{t("deviceDownload.batch.openNote")}</span>
        </div>
      )}
      {reviewing && <p role="status" data-batch-reviewing><span className="task-status-text">{t("deviceDownload.batch.reviewing")}</span></p>}
      {review !== null && (
        <div className="device-download-batch-review" data-batch-review>
          <p>{t("deviceDownload.batch.nothingSent")}</p>
          {review.devices.length === 0 && <p data-batch-empty>{t("deviceDownload.batch.empty")}</p>}
          <ul className="device-download-batch-list" aria-label={t("deviceDownload.batch.listLabel")}>
            {review.devices.map((row) => (
              <li key={row.deviceId} data-batch-device={row.address ?? `id-${row.deviceId}`} data-batch-status={row.status}
                className="device-download-batch-row">
                {row.status === "inBatch" ? <>
                  <label className="device-download-batch-include">
                    <input type="checkbox" checked={!excluded.has(row.planId)} disabled={busy}
                      onChange={(event) => setExcluded((current) => {
                        const next = new Set(current);
                        if (event.target.checked) next.delete(row.planId); else next.add(row.planId);
                        return next;
                      })} />
                    <span><code>{row.address}</code> {row.name}</span>
                  </label>
                  <span className="device-download-batch-facts">
                    {t(`deviceDownload.scope.${row.scope}` as const)} · {t(row.support === "verified"
                      ? "deviceDownload.batch.support.verified" : "deviceDownload.batch.support.untested")}
                    {" · "}{t("deviceDownload.batch.steps", { count: row.steps })}
                    {" · "}{t("deviceDownload.octetsValue", { count: row.dataOctets })}
                  </span>
                  {(row.warnings.length > 0 || row.inferences.length > 0) && (
                    <details className="device-download-batch-notes" data-batch-notes>
                      <summary>{t("deviceDownload.batch.notes", { count: row.warnings.length + row.inferences.length })}</summary>
                      {row.warnings.length > 0 && <>
                        <h4>{t("deviceDownload.warnings.title")}</h4>
                        <ul>{planWarnings(row.warnings).map((warning, index) => (
                          <li key={index} data-plan-warning={warning.rule}>
                            {warning.explanation === null ? <code>{warning.text}</code> : t(warning.explanation)}</li>))}</ul>
                      </>}
                      {row.inferences.length > 0 && <>
                        <h4>{t("deviceDownload.inferences.title")}</h4>
                        <ul>{row.inferences.map((text, index) => <li key={index}><code>{text}</code></li>)}</ul>
                      </>}
                    </details>
                  )}
                  {row.untestedAcknowledgement !== null && !excluded.has(row.planId) && (
                    <label className="device-download-batch-ack">
                      {t("deviceDownload.untestedPrompt", { phrase: row.untestedAcknowledgement })}
                      <input data-batch-ack value={acknowledged[row.planId] ?? ""} disabled={busy}
                        aria-invalid={acknowledged[row.planId] !== row.untestedAcknowledgement}
                        onChange={(event) => setAcknowledged((current) => ({ ...current, [row.planId]: event.target.value }))} />
                    </label>
                  )}
                </> : <>
                  <span><code>{row.address ?? "—"}</code> {row.name}</span>
                  <span className="device-download-batch-reason">
                    {t("deviceDownload.batch.notInBatch")}: {reasonText(t, row.reason)}</span>
                </>}
              </li>
            ))}
          </ul>
          {blocker !== null && inBatch.length > 0
            && <p className="form-warning" id="device-download-batch-blocked" data-batch-blocked>{t(blocker)}</p>}
          <div className="device-download-actions">
            <button type="button" className="primary-action" data-batch-start
              aria-describedby={blocker !== null && inBatch.length > 0 ? "device-download-batch-blocked" : undefined}
              disabled={blocker !== null || busy || disabled} onClick={() => void start()}>
              {t("deviceDownload.batch.start", { count: selected.length })}</button>
            <button type="button" data-batch-close disabled={busy} onClick={dropReview}>{t("deviceDownload.batch.close")}</button>
          </div>
        </div>
      )}
      {status !== null && (
        <div className="device-download-batch-status" data-batch-run={status.state}>
          <p role="status">{status.state === "running"
            ? t("deviceDownload.batch.running", { position: Math.max(position, 1), total })
            : t("deviceDownload.batch.ended")}
          {status.stopRequested && status.state === "running" && <> {t("deviceDownload.batch.stopRequested")}</>}</p>
          <ul className="device-download-batch-list" aria-label={t("deviceDownload.batch.resultLabel")}>
            {status.devices.map((row) => (
              <li key={row.deviceId} className="device-download-batch-row" data-batch-device={row.address ?? `id-${row.deviceId}`}
                data-batch-state={row.state}>
                <span><code>{row.address ?? "—"}</code> {row.name}</span>
                <span className="device-download-batch-result">{stateText(t, row)}</span>
                {stateDetails(t, row).map((detail, index) => (
                  <span key={index} className="device-download-batch-result" data-batch-detail>{detail}</span>))}
              </li>
            ))}
          </ul>
          {status.state === "running" && (
            <button type="button" data-batch-stop disabled={status.stopRequested} onClick={() => void stop()}>
              {t("deviceDownload.batch.stop")}</button>
          )}
        </div>
      )}
      {error && <p className="form-error" role="alert" data-batch-error>{error}</p>}
      {refreshError && <p className="form-warning" role="alert">{t("programming.refreshFailed", { reason: refreshError })}</p>}
      {consent.dialog}
    </section>
  );
}
