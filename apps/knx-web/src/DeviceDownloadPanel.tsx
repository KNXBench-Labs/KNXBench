/** Download to a device (KNXBench → device over the bus): plan, confirm, and watch every step. */
// "Download" here is only ever KNXBench → device (docs/GLOSSARY.md). The
// panel never decides what is written: the server prepares the plan from
// the open project, the user reads it, `useProgrammingConsent` asks, and
// the server demands the plan's own confirmation phrase and refuses a plan
// the project no longer gives (ADR-0045). Every data block is shown once
// the device has read it back unchanged, with its address and octets.

import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import DeviceLink from "./DeviceLink";
import { emitAchievementEvent } from "./achievementEvents";
import { loadPreferredGateway } from "./gatewayPreference";
import { searchBusInterfaces, useBusDiscovery } from "./busDiscovery";
import DownloadGatewayHolder from "./DownloadGatewayHolder";
import { readinessMarks, type DownloadReadinessMark } from "./downloadReadiness";
import {
  gatewayNote, gatewayOptionLabel, initialGatewayChoice, isFound, pickedGateway, typedGateway, withDiscoveryDefault,
  type GatewayChoice,
} from "./downloadGateway";
import { resolveHandoff, type DownloadActivity, type DownloadHandoff, type HandoffOutcome } from "./downloadHandoff";
import DownloadVerification from "./DownloadVerification";
import DownloadSupportNotice from "./DownloadSupportNotice";
import { guidanceForError, guidanceForMark, type SupportGuidance } from "./downloadSupportGuidance";
import { planWarnings } from "./downloadVerification";
import { currentGuideStep, downloadGuide, type GuideMissing, type GuideStepId } from "./downloadGuide";
import HelpTip from "./HelpTip";
import { useTranslate, type MessageKey } from "./i18n";
import { collectDevices, projectContextKey } from "./treeUtils";
import { useProgrammingConsent } from "./useProgrammingConsent";
import type { ProjectTree } from "./bindings/ProjectTree";

const POLL_INTERVAL_MS = 500;

/** What the plan writes (KL-142): the complete download, or a CP §3.9.2.4
 * partial download of the chosen parts. The server derives the partial plan
 * and refuses a scope it cannot; this panel only asks. */
type DownloadScope = "complete" | "parameters" | "groupAddresses" | "both";
const SCOPES: readonly DownloadScope[] = ["complete", "parameters", "groupAddresses", "both"];
const SCOPE_PARTS: Record<Exclude<DownloadScope, "complete">, api.DeviceDownloadParts> = {
  parameters: { parameters: true, groupAddresses: false },
  groupAddresses: { parameters: false, groupAddresses: true },
  both: { parameters: true, groupAddresses: true },
};

function hex(value: number, digits: number): string {
  return value.toString(16).toUpperCase().padStart(digits, "0");
}

function octetsText(octets: number[]): string {
  return octets.map((octet) => hex(octet, 2)).join(" ");
}

const GUIDE_STEPS: Record<GuideStepId, MessageKey> = {
  device: "deviceDownload.guide.step.device",
  gateway: "deviceDownload.guide.step.gateway",
  scope: "deviceDownload.guide.step.scope",
  plan: "deviceDownload.guide.step.plan",
  confirm: "deviceDownload.guide.step.confirm",
};
const GUIDE_MISSING: Record<GuideMissing, MessageKey> = {
  device: "deviceDownload.guide.missing.device",
  gateway: "deviceDownload.guide.missing.gateway",
  plan: "deviceDownload.guide.missing.plan",
  acknowledgement: "deviceDownload.guide.missing.acknowledgement",
};

interface DeviceDownloadPanelProps {
  project: ProjectTree | null;
  /** A "Download to device…" request from the device views, by stable id. */
  handoff?: DownloadHandoff | null;
  /** Called once per request after it was resolved, whatever the outcome. */
  onHandoffHandled?: (nonce: number) => void;
}

export default function DeviceDownloadPanel({ project, handoff = null, onHandoffHandled }: DeviceDownloadPanelProps) {
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
  // The device is chosen by stable id; its address is read from the
  // current project, never kept as a possibly outdated copy.
  const [deviceId, setDeviceId] = useState<number | null>(null);
  const selectedDevice = devices.find((device) => device.id === deviceId) ?? null;
  const address = selectedDevice?.address ?? "";
  const [scope, setScope] = useState<DownloadScope>("complete");
  // The scope the shown plan was asked for: its texts must not follow a
  // later radio change (the plan itself is dropped on change anyway).
  const [planScope, setPlanScope] = useState<DownloadScope>("complete");
  const discovery = useBusDiscovery();
  const [gatewayChoice, setGatewayChoice] = useState<GatewayChoice>(() => initialGatewayChoice(loadPreferredGateway()));
  const gateway = gatewayChoice.value;
  const [plan, setPlan] = useState<api.DeviceDownloadPlan | null>(null);
  const [untestedAcknowledgement, setUntestedAcknowledgement] = useState("");
  const [planning, setPlanning] = useState(false);
  const [starting, setStarting] = useState(false);
  const [status, setStatus] = useState<api.DeviceDownloadStatusResponse | null>(null);
  const [events, setEvents] = useState<api.DeviceDownloadEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  // What the last status read said. Until one answers, a running download
  // cannot be ruled out and a device hand-off waits; `connecting` is a 409
  // (a start is opening its tunnel), `unknown` any other failed read.
  const [statusCheck, setStatusCheck] = useState<"checking" | "checked" | "connecting" | "unknown">("checking");
  const [handoffOutcome, setHandoffOutcome] = useState<HandoffOutcome | null>(null);
  const sinceRef = useRef(0);
  const pollInFlightRef = useRef(false);
  const downloadIdRef = useRef<number | null>(null);
  const planningRequest = useRef(0);
  const latestContext = useRef({ project, deviceId, scope });
  latestContext.current = { project, deviceId, scope };
  const [refusal, setRefusal] = useState<{ project: ProjectTree | null; deviceId: number | null; scope: DownloadScope; guidance: SupportGuidance } | null>(null);

  // A plan belongs to the project it was prepared from; any edit makes it
  // stale, and the server would refuse it anyway (ADR-0045 §3).
  useEffect(() => { dropPlan(); }, [project]);

  // A device that left the project or lost its address cannot stay chosen.
  useEffect(() => {
    if (deviceId !== null && selectedDevice === null) setDeviceId(null);
  }, [deviceId, selectedDevice]);

  // Offline readiness (no bus access), re-read for every project version:
  // a device the server cannot plan is marked before anything is prepared.
  const [readinessAttempt, setReadinessAttempt] = useState(0);
  const [readiness, setReadiness] = useState<{ project: ProjectTree; marks: Map<string, DownloadReadinessMark>; error: SupportGuidance | null; checking: boolean } | null>(null);
  const currentReadiness = readiness?.project === project ? readiness : null;
  const marks = currentReadiness?.marks ?? new Map<string, DownloadReadinessMark>();
  useEffect(() => {
    if (project === null) return;
    let current = true;
    setReadiness({ project, marks: new Map(), error: null, checking: true });
    void (async () => {
      try {
        const next = readinessMarks((await api.getDeviceReadiness()).devices);
        if (current) setReadiness({ project, marks: next, error: null, checking: false });
      } catch (reason) {
        // Also a malformed answer: no marks, and the panel says so.
        if (current) setReadiness({ project, marks: new Map(), error: guidanceForError(reason), checking: false });
      }
    })();
    return () => { current = false; };
  }, [project, readinessAttempt]);
  const selectedMark = address === "" ? undefined : marks.get(address);
  const selectedRefusal = refusal?.project === project && refusal.deviceId === deviceId && refusal.scope === scope ? refusal.guidance : null;
  const shownGuidance = selectedRefusal ?? currentReadiness?.error ?? (selectedMark ? guidanceForMark(selectedMark) : null);

  const running = status?.status.state === "running";
  const locked = planning || starting || running;

  function dropPlan() {
    setPlan(null);
    setUntestedAcknowledgement("");
    setRefusal(null);
    setError(null);
    planningRequest.current += 1;
    setPlanning(false);
  }

  // Device, gateway and scope changes discard a shown plan, like a project edit.
  function chooseGateway(next: GatewayChoice) {
    if (next.value !== gatewayChoice.value) dropPlan();
    setGatewayChoice(next);
  }

  // Exactly one interface answered and no gateway was chosen or typed: use
  // it. Never while a plan is shown or a download is busy, and never over a
  // user's choice (`withDiscoveryDefault` only fills an empty `none`).
  useEffect(() => {
    if (locked || plan !== null) return;
    setGatewayChoice((current) => withDiscoveryDefault(current, discovery));
  }, [discovery, locked, plan]);

  // A hand-off only selects. It waits for the first status read, never
  // switches devices while a plan is prepared or a download runs, and is
  // answered exactly once per click.
  const handledRef = useRef<number | null>(null);
  // The same nonce as state, so the pending context below re-renders when it is answered.
  const [answered, setAnswered] = useState<number | null>(null);
  const recheckedRef = useRef<number | null>(null);
  useEffect(() => {
    if (handoff === null || handledRef.current === handoff.nonce || statusCheck === "checking") return;
    // An uncertain answer gets one fresh status read (a server read, no bus
    // access) before this request is answered.
    if ((statusCheck === "unknown" || statusCheck === "connecting") && recheckedRef.current !== handoff.nonce) {
      recheckedRef.current = handoff.nonce;
      setStatusCheck("checking");
      void poll();
      return;
    }
    handledRef.current = handoff.nonce;
    setAnswered(handoff.nonce);
    const activity: DownloadActivity = locked || statusCheck === "connecting" ? "busy"
      : statusCheck === "unknown" ? "unknown" : "idle";
    const outcome = resolveHandoff(handoff, project, activity);
    if (outcome.kind === "selected" && outcome.deviceId !== deviceId) {
      setDeviceId(outcome.deviceId);
      dropPlan();
    }
    setHandoffOutcome(outcome);
    onHandoffHandled?.(handoff.nonce);
  }, [handoff, statusCheck, locked, project, deviceId, onHandoffHandled]);

  // ADR-0089: verified means every block was read back unchanged
  // (`written: "yes"`) and the restart was not left unconfirmed; reported
  // once per download.
  const reportedRef = useRef<number | null>(null);
  function apply(next: api.DeviceDownloadStatusResponse) {
    if (downloadIdRef.current !== null && next.downloadId !== downloadIdRef.current) return;
    const done = next.status;
    if (done.state === "finished" && done.written === "yes" && done.restart !== "unconfirmed"
      && reportedRef.current !== next.downloadId) {
      reportedRef.current = next.downloadId;
      emitAchievementEvent({ type: "deviceDownloadVerified", subject: next.address });
    }
    downloadIdRef.current = next.downloadId;
    sinceRef.current = next.nextSince;
    setStatus(next);
    setStatusCheck("checked");
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
      setStatusCheck(code === 404 ? "checked" : code === 409 ? "connecting" : "unknown");
    } finally {
      pollInFlightRef.current = false;
    }
  }

  // Shows a download that is already running (or finished) when the panel opens.
  useEffect(() => {
    void poll();
  }, []);

  /** #18: one more status read on request; it never selects or plans anything by itself. */
  function recheckStatus() {
    setError(null);
    setStatusCheck("checking");
    void poll();
  }

  // #18: a request from Devices is shown at once as context, but only selected
  // (admitted) once the status read rules out a running download, as above.
  const pendingRequest = handoff !== null && answered !== handoff.nonce;
  const requested = useMemo(() => {
    if (!pendingRequest || project === null || projectContextKey(project) !== handoff.context) return null;
    const device = collectDevices(project).get(handoff.deviceId);
    return device && device.address !== null ? { name: device.name, address: device.address } : null;
  }, [pendingRequest, project, handoff]);

  useEffect(() => {
    if (!running) return;
    const interval = window.setInterval(() => void poll(), POLL_INTERVAL_MS);
    return () => window.clearInterval(interval);
  }, [running, status?.downloadId]);

  async function preparePlan() {
    setError(null);
    setRefusal(null);
    setPlan(null);
    setUntestedAcknowledgement("");
    setPlanning(true);
    const context = { project, deviceId, scope };
    const request = ++planningRequest.current;
    const current = () => request === planningRequest.current &&
      latestContext.current.project === context.project && latestContext.current.deviceId === context.deviceId &&
      latestContext.current.scope === context.scope;
    try {
      const target = address.trim();
      const asked = scope;
      const next = asked === "complete"
        ? await api.planDeviceDownload(target)
        : await api.planDeviceDownload(target, SCOPE_PARTS[asked]);
      if (current()) { setPlan(next); setPlanScope(asked); }
    } catch (reason) {
      if (current()) setRefusal({ ...context, guidance: guidanceForError(reason) });
    } finally {
      if (request === planningRequest.current) setPlanning(false);
    }
  }

  async function start() {
    if (plan === null) return;
    const shown = plan;
    const context = { project, deviceId, scope };
    if (shown.untestedAcknowledgement !== null && untestedAcknowledgement !== shown.untestedAcknowledgement) return;
    setError(null);
    if (!(await consent.request(`${shown.address} — ${shown.deviceName}`))) return;
    if (latestContext.current.project !== context.project || latestContext.current.deviceId !== context.deviceId ||
      latestContext.current.scope !== context.scope) return;
    setStarting(true);
    try {
      const startArgs: [number, string, string, string?] = [
        shown.planId,
        gateway.trim(),
        // Built from the plan's own address by the server: the phrase for
        // exactly the device the user just confirmed.
        shown.confirmationPhrase,
      ];
      if (shown.untestedAcknowledgement !== null) startArgs.push(untestedAcknowledgement);
      const { downloadId } = await api.startDeviceDownload(...startArgs);
      downloadIdRef.current = downloadId;
      sinceRef.current = 0;
      setEvents([]);
      setStatus(null);
      setPlan(null);
      await poll();
    } catch (reason) {
      if (latestContext.current.project === context.project && latestContext.current.deviceId === context.deviceId) {
        const guidance = guidanceForError(reason);
        setRefusal({ ...context, guidance });
        if (guidance.action === "prepareAgain") {
          setPlan(null);
          setUntestedAcknowledgement("");
        }
      }
    } finally {
      setStarting(false);
    }
  }

  const guide = downloadGuide({ deviceSelected: selectedDevice !== null, gateway, plan, acknowledgement: untestedAcknowledgement });
  const currentStep = currentGuideStep(guide);
  const note = gatewayNote(gatewayChoice, discovery);

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
  const shownWarnings = plan === null ? [] : planWarnings(plan.warnings);

  return (
    <section className="device-download-panel">
      <header>
        <p className="eyebrow">{t("deviceDownload.eyebrow")}</p>
        <h2>{t("deviceDownload.title")}</h2>
        <p>{t("deviceDownload.explainer")}</p>
      </header>

      {project !== null && (
        <section className="device-download-guide" aria-label={t("deviceDownload.guide.title")}>
          <h3>{t("deviceDownload.guide.title")}
            <HelpTip labelKey="help.tip.deviceDownload.label" textKey="help.tip.deviceDownload.text" topicId="limits" /></h3>
          <ol>
            {guide.map((step) => (
              <li key={step.id} data-guide-step={step.id} data-guide-state={step.state}
                aria-current={step.id === currentStep ? "step" : undefined}>
                <span className="device-download-guide-name">{t(GUIDE_STEPS[step.id])}</span>
                <span className="device-download-guide-state">{t(`deviceDownload.guide.state.${step.state}` as const)}</span>
                {step.id === "scope" && <span className="device-download-guide-hint">{t("deviceDownload.guide.scope", { scope: t(`deviceDownload.scope.${scope}` as const) })}</span>}
                {step.id === "plan" && plan !== null && <span className="device-download-guide-hint">
                  {t(plan.support.level === "verified" ? "deviceDownload.guide.planReady.verified" : "deviceDownload.guide.planReady.untested")}</span>}
                {step.id === "confirm" && plan !== null && <span className="device-download-guide-hint">{t("deviceDownload.guide.onStart")}</span>}
                {/* A blocked step only says so; its reason is the earlier open step's hint. */}
                {step.state === "open" && step.missing.map((missing) => (
                  <span key={missing} className="device-download-guide-hint">{t(GUIDE_MISSING[missing])}</span>
                ))}
              </li>
            ))}
          </ol>
          <p className="device-download-guide-note">{t("deviceDownload.guide.discards")}</p>
        </section>
      )}

      {project === null ? (
        <p>{t("deviceDownload.projectRequired")}</p>
      ) : (
        <form className="device-download-config" onSubmit={(event) => event.preventDefault()}>
          <label className="device-download-device">
            {t("deviceDownload.device")}
            <select data-download-device value={deviceId ?? ""} disabled={locked}
              onChange={(event) => { setDeviceId(event.target.value === "" ? null : Number(event.target.value)); dropPlan(); setHandoffOutcome(null); }}>
              <option value="">{t("deviceDownload.chooseDevice")}</option>
              {devices.map((device) => {
                const mark = device.address === null ? undefined : marks.get(device.address);
                const suffix = mark === undefined ? ""
                  : ` (${t(mark.kind === "excluded" ? "deviceDownload.readiness.optionMarkExcluded" : "deviceDownload.readiness.optionMark")})`;
                return (
                  <option key={device.id} value={device.id}>
                    {`${device.address} — ${device.name}${suffix}`}
                  </option>
                );
              })}
            </select>
          </label>
          {pendingRequest && (
            <p className="device-download-handoff" role="status" data-handoff="pending">
              {requested ? t("deviceDownload.handoff.pending", requested) : t("deviceDownload.handoff.pendingUnknown")}
            </p>
          )}
          {!pendingRequest && statusCheck !== "checked" && (
            <p className="device-download-status" role="status" data-download-status={statusCheck}>
              {t(`deviceDownload.status.${statusCheck}` as const)}
              {statusCheck !== "checking" && <> <button type="button" data-status-retry onClick={recheckStatus}>
                {t("deviceDownload.status.retry")}</button></>}
            </p>
          )}
          {currentReadiness?.checking && <p className="device-download-readiness-loading" role="status" data-readiness-loading>
            {t("deviceDownload.readiness.loading")}</p>}
          {shownGuidance && <DownloadSupportNotice key={`${deviceId}-${scope}-${shownGuidance.code}-${shownGuidance.detail}`}
            guidance={shownGuidance} failedRequest={selectedRefusal !== null} />}
          {currentReadiness?.error && !currentReadiness.checking && <button type="button" data-readiness-retry
            onClick={() => setReadinessAttempt((n) => n + 1)}>{t("deviceDownload.readiness.retry")}</button>}
          {handoffOutcome && (
            <p className={handoffOutcome.kind === "selected" ? "device-download-handoff" : "device-download-handoff form-warning"}
              role="status" data-handoff={handoffOutcome.kind}>
              {handoffOutcome.kind === "selected"
                ? t("deviceDownload.handoff.selected", { address: handoffOutcome.address, name: handoffOutcome.name })
                : handoffOutcome.kind === "noAddress"
                  ? t("deviceDownload.handoff.noAddress", { name: handoffOutcome.name })
                  : t(`deviceDownload.handoff.${handoffOutcome.kind}` as const)}
            </p>
          )}
          <fieldset className="device-download-gateway">
            <legend>{t("deviceDownload.gatewayLegend")}</legend>
            <p className="device-download-gateway-status" role="status" aria-live="polite">
              {discovery.phase === "idle" ? t("deviceDownload.gatewaySearch.idle")
                : discovery.phase === "searching" ? t("deviceDownload.gatewaySearch.searching")
                  : discovery.phase === "failed" ? t("deviceDownload.gatewaySearch.failed")
                    : discovery.interfaces.length === 0
                      ? t(gateway.trim() === "" ? "deviceDownload.gatewaySearch.none" : "deviceDownload.gatewaySearch.noneUseTyped")
                      : t("deviceDownload.gatewaySearch.found", { count: discovery.interfaces.length })}
            </p>
            {discovery.phase === "failed" && discovery.error && <p className="bus-discovery-detail">{discovery.error}</p>}
            <label>
              {t("deviceDownload.gateway")}
              <input data-download-gateway value={gateway} disabled={locked}
                onChange={(event) => chooseGateway(typedGateway(event.target.value))} placeholder="192.0.2.10:3671" />
            </label>
            {discovery.interfaces.length > 0 && (
              <label>
                {t("deviceDownload.gatewayFound")}
                <select data-download-gateway-found value={isFound(gateway, discovery.interfaces) ? gateway.trim() : ""} disabled={locked}
                  onChange={(event) => { if (event.target.value !== "") chooseGateway(pickedGateway(event.target.value)); }}>
                  <option value="">{t("deviceDownload.gatewayChooseFound")}</option>
                  {discovery.interfaces.map((iface, index) => (
                    <option key={`${iface.controlEndpoint}-${index}`} value={iface.controlEndpoint}>{gatewayOptionLabel(iface)}</option>
                  ))}
                </select>
              </label>
            )}
            {note && <p className="device-download-gateway-note" data-gateway-note={note}>{t(`deviceDownload.gatewayNote.${note}` as const)}</p>}
            <DownloadGatewayHolder gateway={gateway} disabled={locked}
              onUseGateway={(endpoint) => chooseGateway(pickedGateway(endpoint))} />
            <button type="button" className="device-download-search" disabled={locked || discovery.phase === "searching"}
              onClick={() => void searchBusInterfaces()}>
              {t("deviceDownload.gatewaySearchAgain")}
            </button>
          </fieldset>
          <fieldset className="device-download-scope">
            <legend>{t("deviceDownload.scope.legend")}</legend>
            {SCOPES.map((option) => (
              <label key={option}>
                <input type="radio" name="device-download-scope" value={option} checked={scope === option}
                  disabled={locked} onChange={() => { setScope(option); dropPlan(); }} />
                {t(`deviceDownload.scope.${option}` as const)}
              </label>
            ))}
          </fieldset>
          <div className="device-download-actions">
            <button type="button" disabled={locked || address === ""} onClick={() => void preparePlan()}>
              {t("deviceDownload.preparePlan")}
            </button>
            <span className="device-download-actions-note">{t("deviceDownload.preparePlanNote")}</span>
          </div>
        </form>
      )}

      {selectedDevice && <p><DeviceLink deviceId={selectedDevice.id}>{t("devices.openEditor")}</DeviceLink></p>}
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
            <dt>{t("deviceDownload.support")}</dt>
            <dd>
              {plan.support.level === "verified"
                ? <>{t("deviceDownload.verified")} {plan.support.evidence}</>
                : <strong className="form-warning">{t("deviceDownload.untested")}</strong>}
            </dd>
            <dt>{t("deviceDownload.expectedDevice")}</dt>
            <dd>{t("deviceDownload.expectedDeviceValue", { mask: hex(plan.maskVersion, 4), manufacturer: hex(plan.manufacturer, 4) })}</dd>
            <dt>{t("deviceDownload.fromProject")}</dt>
            <dd>{t("deviceDownload.fromProjectValue", { values: plan.parameterValues, links: plan.groupLinks })}</dd>
            <dt>{t("deviceDownload.octets")}</dt>
            <dd>{t("deviceDownload.octetsValue", { count: plan.dataOctets })}</dd>
          </dl>
          {shownWarnings.length > 0 && (
            <section className="device-download-warnings" aria-label={t("deviceDownload.warnings.title")}>
              <h4>{t("deviceDownload.warnings.title")}</h4>
              <p>{t("deviceDownload.warnings.intro")}</p>
              <ul>
                {shownWarnings.map((warning, index) => (
                  <li key={index} data-plan-warning={warning.rule}>
                    {warning.explanation === null ? <code>{warning.text}</code> : t(warning.explanation)}
                  </li>
                ))}
              </ul>
              {shownWarnings.some((warning) => warning.explanation !== null) && (
                <details>
                  <summary>{t("deviceDownload.warnings.details")}</summary>
                  <ul>{plan.warnings.map((text, index) => <li key={index}><code>{text}</code></li>)}</ul>
                </details>
              )}
            </section>
          )}
          {plan.support.inferences.length > 0 && (
            <section className="device-download-inferences" aria-label={t("deviceDownload.inferences.title")}>
              <h4>{t("deviceDownload.inferences.title")}</h4>
              <p>{t("deviceDownload.inferences.intro")}</p>
              <ul>{plan.support.inferences.map((text, index) => <li key={index}><code>{text}</code></li>)}</ul>
            </section>
          )}
          {plan.partial && (
            <section className="device-download-not-written">
              <p><strong>{t(`deviceDownload.partialScope.${planScope === "complete" ? "both" : planScope}` as const)}</strong></p>
              <p>{t("deviceDownload.partialCheck")}</p>
              {plan.notWritten.length === 0 ? (
                <p>{t("deviceDownload.notWrittenNone")}</p>
              ) : (
                <>
                  <p>{t("deviceDownload.notWrittenExplainer")}</p>
                  <table>
                    <thead><tr><th>{t("deviceDownload.address")}</th><th>{t("deviceDownload.notWrittenOctets")}</th></tr></thead>
                    <tbody>
                      {plan.notWritten.map(([at, octets]) => (
                        <tr key={at}><td><code>{hex(at, 4)}h</code></td><td>{octets}</td></tr>
                      ))}
                    </tbody>
                  </table>
                </>
              )}
            </section>
          )}
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
          <p>{t("deviceDownload.backupBeforeWrite")}</p>
          {plan.support.level === "untested" && (
            <><p className="device-download-plan-hint">{t("deviceDownload.help.untested")}</p>
              <p className="device-download-plan-hint" data-verify-hint>{t("deviceDownload.verify.planHint")}</p></>
          )}
          {plan.untestedAcknowledgement !== null && (
            <label className="device-download-acknowledgement">
              {t("deviceDownload.untestedPrompt", { phrase: plan.untestedAcknowledgement })}
              <input data-untested-acknowledgement value={untestedAcknowledgement}
                autoComplete="off" spellCheck={false}
                onChange={(event) => setUntestedAcknowledgement(event.target.value)} disabled={locked} />
            </label>
          )}
          <button className="primary-action" disabled={locked || gateway.trim() === "" ||
            (plan.untestedAcknowledgement !== null && untestedAcknowledgement !== plan.untestedAcknowledgement)} onClick={() => void start()}>
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
          {status.backupFile && (
            <p>{t("deviceDownload.backupKept", { file: status.backupFile })}</p>
          )}
          {events.filter((event) => event.kind === "backupTaken").map((event, index) => (
            <p key={index}>{t("deviceDownload.backupTaken", { octets: event.octets, regions: event.regions })}</p>
          ))}
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
      {status && status.status.state !== "running" && (
        <DownloadVerification key={status.downloadId} downloadId={status.downloadId}
          completed={status.status.state === "finished" && status.status.written === "yes"} />
      )}
      {consent.dialog}
    </section>
  );
}
