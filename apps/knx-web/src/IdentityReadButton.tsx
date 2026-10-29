/** Reads a device identity with or without a project, then hands off to the report wizard. */
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useFileMenuItemProps } from "./FileMenu";
import { splitGatewayEndpoint, validateGatewayFields } from "./gatewayEndpoint";
import { useTranslate } from "./i18n";
import { validateIndividualAddress } from "./lineScanExclusions";
import Overlay from "./Overlay";
import { reportContext } from "./reportProblem";
import { useReportProblem } from "./ReportProblemEntry";
import { diagnoseSupport, previewSupport, type SupportPreview } from "./supportApi";
import { collectDevices } from "./treeUtils";
import "./universalSupport.css";

export default function IdentityReadButton({ project }: { project: ProjectTree | null }) {
  const t = useTranslate();
  const entry = useFileMenuItemProps();
  const binding = JSON.stringify(reportContext(project)) + (project === null ? "/closed" : "/open");
  const [openedAt, setOpenedAt] = useState<string | null>(null);
  useLayoutEffect(() => { if (openedAt !== null && openedAt !== binding) setOpenedAt(null); }, [binding, openedAt]);
  return <>
    <button type="button" {...entry} data-identity-read-open onClick={() => setOpenedAt(binding)}>{t("identityRead.title")}</button>
    {openedAt === binding && createPortal(<IdentityReadDialog project={project} close={() => setOpenedAt(null)} />, document.body)}
  </>;
}

function IdentityReadDialog({ project, close }: { project: ProjectTree | null; close: () => void }) {
  const t = useTranslate();
  const report = useReportProblem();
  const devices = project ? [...collectDevices(project).values()] : [];
  const [deviceId, setDeviceId] = useState<number | null>(null);
  const [address, setAddress] = useState("");
  const [programId, setProgramId] = useState("");
  const [mask, setMask] = useState("");
  const [gateway, setGateway] = useState("");
  const [preview, setPreview] = useState<SupportPreview | null>(null);
  const [complete, setComplete] = useState(false);
  const [consent, setConsent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const epoch = useRef(0);
  const abort = useRef<AbortController | null>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => () => { ++epoch.current; abort.current?.abort(); }, []);
  const selected = devices.find((d) => d.id === deviceId);
  const target = deviceId === null ? address : selected?.address ?? "";
  const valid = validateIndividualAddress(target) && (deviceId === null || reportContext(project) !== undefined)
    && (mask === "" || /^[a-fA-F0-9]{4}$/.test(mask));
  const endpoint = validateGatewayFields(splitGatewayEndpoint(gateway));
  function invalidate() {
    ++epoch.current; abort.current?.abort(); setBusy(false); setPreview(null); setComplete(false); setConsent(false); setError(false);
  }
  function requestClose() { invalidate(); close(); }
  async function perform(read: boolean) {
    if (busy || !valid || (read && (!preview || !consent || !endpoint.ok || complete))) return;
    const id = ++epoch.current;
    const controller = new AbortController(); abort.current = controller;
    setBusy(true); setError(false); setConsent(false);
    try {
      const next = read
        ? await diagnoseSupport(preview!.token, endpoint.ok ? endpoint.endpoint : "", controller.signal)
        : await previewSupport({ includeContext: false, contextRights: false, functionalCheck: "not-checked",
          ...(programId ? { programId } : {}), ...(mask ? { mask: parseInt(mask, 16) } : {}) },
        deviceId, deviceId !== null ? reportContext(project) : undefined, undefined, deviceId === null ? address : undefined, controller.signal);
      if (id !== epoch.current) return;
      if (next.localAddress !== target || (read && next.token !== preview!.token)) throw new Error("support-target-changed");
      setPreview(next); setComplete(read);
    } catch { if (id === epoch.current && !controller.signal.aborted) { setError(true); setPreview(null); setComplete(false); } }
    finally { if (id === epoch.current) setBusy(false); }
  }
  return <Overlay labelledBy="identity-read-title" className="universal-support" resizable={{ width: 850, height: 760 }} initialFocusRef={heading} onClose={requestClose}>
    <div data-identity-read-dialog>
      <header><h2 id="identity-read-title" tabIndex={-1} ref={heading}>{t("identityRead.title")}</h2>
        <button type="button" data-support-close onClick={requestClose}>{t("support.close")}</button></header>
      <p>{t("identityRead.intro")}</p>
      <fieldset disabled={busy}>
        {project && <label>{t("support.device")}<select data-support-device value={deviceId ?? ""} onChange={(e) => { invalidate(); setDeviceId(e.target.value === "" ? null : Number(e.target.value)); }}>
          <option value="">{t("identityRead.manual")}</option>{devices.map((d) => <option key={d.id} value={d.id}>{d.address ?? "—"} · {d.name}</option>)}
        </select></label>}
        {deviceId === null && <label>{t("identityRead.address")}<input data-support-local-address value={address} maxLength={10}
          aria-invalid={address !== "" && !validateIndividualAddress(address)} onChange={(e) => { invalidate(); setAddress(e.target.value); }} placeholder="1.1.67" /></label>}
        <label>{t("identityRead.program")}<input data-support-program value={programId} maxLength={64} onChange={(e) => { invalidate(); setProgramId(e.target.value); }} /></label>
        <label>{t("identityRead.mask")}<input data-support-mask value={mask} maxLength={4} aria-invalid={mask !== "" && !/^[a-fA-F0-9]{4}$/.test(mask)} onChange={(e) => { invalidate(); setMask(e.target.value); }} placeholder="0025" /></label>
        <button type="button" data-support-preview disabled={!valid} onClick={() => void perform(false)}>{t("identityRead.preview")}</button>
      </fieldset>
      {busy && <p role="status" className="task-status-text">{t("identityRead.busy")}</p>}
      {error && <p role="alert">{t("identityRead.failed")}</p>}
      {preview && <>
        <section data-support-preview-content><h3>{t("identityRead.scope")}</h3>
          <p>{preview.localAddress} · {preview.profile} · {t("identityRead.deadline", { seconds: preview.deadlineSeconds })}</p>
          <ul>{preview.reads.map((read, i) => <li key={i}>{read}</li>)}</ul>
          <pre>{preview.summary}</pre>
          <details><summary>{t("identityRead.result")}</summary><pre>{JSON.stringify(preview.report, null, 2)}</pre></details>
        </section>
        {!complete && <>
          <p className="form-warning">{t("identityRead.warning")}</p>
          <label>{t("support.gateway")}<input data-support-gateway value={gateway} disabled={busy} onChange={(e) => { setGateway(e.target.value); setConsent(false); }} placeholder="192.0.2.10:3671" /></label>
          <label className="support-check"><input type="checkbox" data-support-live-consent checked={consent} disabled={busy || !endpoint.ok} onChange={(e) => setConsent(e.target.checked)} />{t("identityRead.consent")}</label>
          <button type="button" data-support-read disabled={busy || !consent || !endpoint.ok} onClick={() => void perform(true)}>{t("identityRead.read")}</button>
        </>}
        {complete && report && <>
          <p>{t("identityRead.handoff")}</p>
          <button type="button" data-report-problem-open="support-read" onClick={() => {
            const operation = { relatedKey: "supportToken", relatedValue: preview.token };
            requestClose(); report({ problemType: "deviceReading", operation, ...(deviceId !== null ? { deviceId } : {}) });
          }}>{t("reportProblem.button")}</button>
        </>}
      </>}
    </div>
  </Overlay>;
}
