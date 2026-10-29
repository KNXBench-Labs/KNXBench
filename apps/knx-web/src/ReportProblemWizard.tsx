/** Report problem wizard: problem type, contents, preview, consent and ZIP (DIAG-03). */
import { useEffect, useMemo, useRef, useState } from "react";
import { version as packageVersion } from "../package.json";
import { emitAchievementEvent } from "./achievementEvents";
import { errorMessage } from "./api";
import { appUiScale } from "./appUiScale";
import type { ProjectTree } from "./bindings/ProjectTree";
import {
  deleteRecordings, exportBundle, previewBundle, sectionText, PROBLEM_TYPES,
  type BundlePreview, type DeleteOutcome, type OfferView, type ProblemType, type RecordingSummary, type ReviewedBundleQuery, type SectionKind,
} from "./diagnosticBundleApi";
import { isTauri } from "./filePicker";
import { buildIssueUrl } from "./githubIssue";
import { useTranslate } from "./i18n";
import { downloadLocalBlob } from "./localJsonDownload";
import type { MessageKey } from "./messages/en";
import Overlay from "./Overlay";
import {
  buildQuery, bytesByKind, defaultOffer, formatBytes, initialChoice, mailDraftFor, markerKeys, reasonKey,
  boundedClientFacts, operationKindKey, operationOutcomeKey, reportContext, reviewedQuery, resolvedRecording, toggleEarlier, toggleSection, withProblemType,
  type ReportProblemRequest, type WizardChoice,
} from "./reportProblem";
import { useSavedThemeId } from "./theme";
import type { ToastNotification } from "./toast";
import { collectDevices } from "./treeUtils";
import { useUiLanguage } from "./uiLanguage";
import "./reportProblem.css";

type Step = "type" | "contents" | "preview" | "send";
const STEPS: readonly Step[] = ["type", "contents", "preview", "send"];
const STEP_LABEL: Record<Step, MessageKey> = {
  type: "reportProblem.step.type",
  contents: "reportProblem.step.contents",
  preview: "reportProblem.step.preview",
  send: "reportProblem.step.send",
};

type Loaded =
  | { status: "loading"; preview: BundlePreview | null }
  | { status: "ready"; preview: BundlePreview; query: ReviewedBundleQuery }
  | { status: "failed"; preview: BundlePreview | null; error: string };

type SectionText = { status: "loading" } | { status: "ready"; text: string } | { status: "failed"; error: string };

export default function ReportProblemWizard({ request, tree, notifications, onClose }: {
  request: ReportProblemRequest;
  tree: ProjectTree | null;
  notifications: readonly ToastNotification[];
  onClose: () => void;
}) {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const themeId = useSavedThemeId();
  // What the browser says about itself is taken once, when the wizard opens.
  const [client] = useState(() => boundedClientFacts({
    appVersion: packageVersion,
    shell: isTauri() ? "tauri" : "browser",
    uiLanguage: language,
    theme: themeId,
    viewport: { width: window.innerWidth, height: window.innerHeight },
    uiScale: appUiScale(),
  }, notifications));
  const [step, setStep] = useState<Step>(request.problemType ? "contents" : "type");
  const [choice, setChoice] = useState<WizardChoice>(() => initialChoice(request));
  const [loaded, setLoaded] = useState<Loaded>({ status: "loading", preview: null });
  const [reload, setReload] = useState(0);
  const [texts, setTexts] = useState<ReadonlyMap<string, SectionText>>(new Map());
  const [shown, setShown] = useState<ReadonlySet<string>>(new Set());
  const [consent, setConsent] = useState(false);
  const [description, setDescription] = useState("");
  const [saving, setSaving] = useState(false);
  // A void blob click proves initiation only; the user confirms the actual file.
  const [saved, setSaved] = useState<string | null>(null);
  const [fileConfirmed, setFileConfirmed] = useState(false);
  const confirmationAwarded = useRef(false);
  const fileConfirmRef = useRef<HTMLInputElement>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleted, setDeleted] = useState<{ outcome: DeleteOutcome } | { error: string } | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const mailRef = useRef<HTMLAnchorElement>(null);
  const saveErrorRef = useRef<HTMLParagraphElement>(null);
  const deleteResultRef = useRef<HTMLParagraphElement>(null);
  const epoch = useRef(0);
  const mounted = useRef(true);

  const contextKey = JSON.stringify(reportContext(tree));
  const query = useMemo(() => ({ ...buildQuery(choice, client as unknown as Record<string, unknown>),
    ...(reportContext(tree) ? { context: reportContext(tree) } : {}) }), [choice, client, contextKey]);
  const queryKey = JSON.stringify(query);
  const collecting = step !== "type";

  // Set on every mount: React's development double mount runs this cleanup once in between.
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; ++epoch.current; }; }, []);
  useEffect(() => { headingRef.current?.focus(); }, [step]);

  // Every change of the contents asks the server again; an older answer is dropped.
  // Consent and a saved file belong to the contents they were given for.
  useEffect(() => {
    const id = ++epoch.current;
    setLoaded((previous) => ({ status: "loading", preview: collecting ? previous.preview : null }));
    setTexts(new Map());
    setShown(new Set());
    setConsent(false);
    setSaved(null);
    setFileConfirmed(false);
    setSaveError(null);
    setSaving(false);
    if (!collecting) return;
    previewBundle(query).then(
      (preview) => { if (id === epoch.current) setLoaded({ status: "ready", preview, query: reviewedQuery(query, preview) }); },
      (reason) => {
        if (id !== epoch.current) return;
        if (stale(reason)) withdrawReview();
        else setLoaded((previous) => ({ status: "failed", preview: previous.preview, error: errorMessage(reason) }));
      },
    );
    // `query` is read through `queryKey`: an equal request is not asked twice.
  }, [collecting, queryKey, reload]);

  const preview = loaded.preview;
  const ready = loaded.status === "ready";
  const stepIndex = STEPS.indexOf(step);
  const reachable = (which: Step) => which === "type" || which === "contents" || ready;
  const devices = tree ? [...collectDevices(tree).values()] : [];
  const sizes = preview ? bytesByKind(preview.sections) : new Map<SectionKind, number>();
  const operationOffered = defaultOffer(choice.problemType, "operation") !== "notOffered";
  const deviceOffered = defaultOffer(choice.problemType, "device") !== "notOffered"
    || defaultOffer(choice.problemType, "catalogue") !== "notOffered";
  const operation = preview ? resolvedRecording(preview.operationRecordingId, preview.recordings) : null;
  const choosable = preview ? preview.recordings.filter((r) => !r.active) : [];

  function recordingLabel(recording: RecordingSummary): string {
    let started = recording.startedAt ?? recording.id;
    try {
      if (recording.startedAt) started = new Date(recording.startedAt).toLocaleString(language);
    } catch { /* an Intl-refused language tag keeps the raw timestamp */ }
    return t("reportProblem.operation.option", {
      started,
      kind: recording.operationKind === null ? "?" : operationKindKey(recording.operationKind) ? t(operationKindKey(recording.operationKind)!) : recording.operationKind,
      outcome: recording.outcome === null ? t("reportProblem.operation.unfinished")
        : operationOutcomeKey(recording.outcome) ? t(operationOutcomeKey(recording.outcome)!) : recording.outcome,
    });
  }

  function reason(text: string): string {
    const key = reasonKey(text);
    return t("reportProblem.unavailable", { reason: key ? t(key) : text });
  }

  function withdrawReview() {
    ++epoch.current;
    setConsent(false); setSaved(null); setFileConfirmed(false); setSaving(false);
    setTexts(new Map()); setShown(new Set());
    setLoaded({ status: "failed", preview: null, error: t("reportProblem.reviewChanged") });
  }

  function close() {
    withdrawReview();
    onClose();
  }

  const stale = (error: unknown) => typeof error === "object" && error !== null && "status" in error && error.status === 409;

  async function showText(key: string) {
    if (loaded.status !== "ready") return;
    if (shown.has(key)) {
      setShown((current) => { const next = new Set(current); next.delete(key); return next; });
      return;
    }
    setShown((current) => new Set(current).add(key));
    if (texts.get(key)?.status === "ready") return;
    const id = epoch.current;
    setTexts((current) => new Map(current).set(key, { status: "loading" }));
    try {
      if (loaded.status !== "ready") return;
      const text = await sectionText(loaded.query, key);
      if (id === epoch.current && mounted.current) setTexts((current) => new Map(current).set(key, { status: "ready", text }));
    } catch (e) {
      if (id === epoch.current && mounted.current) {
        if (stale(e)) withdrawReview();
        else setTexts((current) => new Map(current).set(key, { status: "failed", error: errorMessage(e) }));
      }
    }
  }

  async function save() {
    if (!consent || loaded.status !== "ready" || saving || !preview) return;
    const id = epoch.current;
    setSaving(true);
    setSaved(null); setFileConfirmed(false); confirmationAwarded.current = false;
    setSaveError(null);
    try {
      const trimmed = description.trim();
      const bundle = await exportBundle(trimmed ? { ...loaded.query, description: trimmed } : loaded.query, preview.fileName);
      if (id !== epoch.current || !mounted.current) return;
      downloadLocalBlob(bundle.blob, bundle.fileName);
      setSaved(bundle.fileName);

    } catch (e) {
      if (id === epoch.current && mounted.current) {
        if (stale(e)) withdrawReview();
        else setSaveError(errorMessage(e));
      }
    } finally {
      if (id === epoch.current && mounted.current) setSaving(false);
    }
  }

  // The Save button disables itself while it runs; hand focus to the result.
  useEffect(() => {
    if (saved !== null) {
      if (fileConfirmed) mailRef.current?.focus();
      else fileConfirmRef.current?.focus();
    }
    else if (saveError !== null) saveErrorRef.current?.focus();
  }, [saved, saveError, fileConfirmed]);

  async function removeRecordings() {
    setConfirmDelete(false);
    try {
      const outcome = await deleteRecordings();
      if (!mounted.current) return;
      setDeleted({ outcome });
      setChoice((current) => ({ ...current, earlier: [] }));
      setReload((n) => n + 1);
    } catch (e) {
      if (mounted.current) setDeleted({ error: errorMessage(e) });
    }
  }
  useEffect(() => { if (deleted !== null) deleteResultRef.current?.focus(); }, [deleted, loaded.status]);

  const mailDraft = preview && saved !== null && fileConfirmed ? mailDraftFor(preview.mailDraft, preview.fileName, saved) : null;
  const issue = buildIssueUrl(t("reportProblem.issueTitle"), t("reportProblem.issueBody", {
    version: packageVersion, problem: t(`reportProblem.type.${choice.problemType}`),
  }));

  function offerRow(offer: OfferView) {
    const kind = offer.kind;
    const label = t(`reportProblem.section.${kind}`);
    const fixed = offer.offer === "fixed";
    const size = sizes.get(kind);
    if (kind === "earlierOperation") {
      const others = choosable.filter((r) => r.id !== preview?.operationRecordingId);
      return <li key={kind} data-report-section={kind}>
        <fieldset className="report-problem-earlier">
          <legend>{label}{" "}<span className="report-problem-size" data-report-size>
            {size !== undefined ? formatBytes(size, language) : t("reportProblem.sizeNotIncluded")}</span></legend>
          {offer.unavailable && <p className="settings-field-hint" data-report-unavailable>{reason(offer.unavailable)}</p>}
          {others.length === 0 ? <p className="settings-field-hint">{t("reportProblem.earlier.none")}</p>
            : <ul aria-label={t("reportProblem.earlier.legend")}>{others.map((recording) => (
              <li key={recording.id}><label>
                <input type="checkbox" data-report-earlier={recording.id} checked={choice.earlier.includes(recording.id)}
                  onChange={(e) => setChoice((current) => toggleEarlier(current, recording.id, e.target.checked))} />
                {recordingLabel(recording)}
              </label></li>
            ))}</ul>}
        </fieldset>
      </li>;
    }
    const reasonId = `report-problem-reason-${kind}`;
    const checked = fixed || (offer.unavailable === null && choice.sections.has(kind));
    return <li key={kind} data-report-section={kind}>
      <label>
        <input type="checkbox" checked={checked} disabled={fixed || offer.unavailable !== null}
          aria-describedby={offer.unavailable ? reasonId : undefined}
          onChange={(e) => setChoice((current) => toggleSection(current, kind, e.target.checked))} />
        {label}
      </label>
      <span className="report-problem-size" data-report-size>
        {fixed && `${t("reportProblem.fixed")} · `}
        {checked && size !== undefined ? formatBytes(size, language) : t("reportProblem.sizeNotIncluded")}
      </span>
      {offer.unavailable && <span className="settings-field-hint report-problem-reason" id={reasonId} data-report-unavailable>
        {reason(offer.unavailable)}</span>}
    </li>;
  }

  return (
    <Overlay labelledBy="report-problem-title" className="new-project-panel project-wizard report-problem"
      initialFocusRef={headingRef} onClose={close}>
      <h2 className="settings-panel-title" id="report-problem-title">{t("reportProblem.title")}</h2>
      <ol className="project-wizard-steps" aria-label={t("projectWizard.stepsLabel")}>
        {STEPS.map((which, index) => (
          <li key={which}>
            <button type="button" className="project-wizard-step-link" aria-current={which === step ? "step" : undefined}
              disabled={!reachable(which)} onClick={() => setStep(which)}>
              <span className="project-wizard-step-number" aria-hidden="true">{index + 1}</span>
              {t(STEP_LABEL[which])}
            </button>
          </li>
        ))}
      </ol>
      <h3 className="project-wizard-step-title" ref={headingRef} tabIndex={-1}>
        {t("projectWizard.stepOf", { current: stepIndex + 1, total: STEPS.length, step: t(STEP_LABEL[step]) })}
      </h3>

      {step === "type" && <>
        <p className="project-wizard-intro">{t("reportProblem.intro")}</p>
        <fieldset className="report-problem-types">
          <legend>{t("reportProblem.type.legend")}</legend>
          {PROBLEM_TYPES.map((type: ProblemType) => (
            <label key={type} className="report-problem-type">
              <input type="radio" name="report-problem-type" value={type} checked={choice.problemType === type}
                data-report-type={type} onChange={() => setChoice((current) => withProblemType(current, type))} />
              <span><strong>{t(`reportProblem.type.${type}`)}</strong>
                <span className="settings-field-hint">{t(`reportProblem.type.${type}.hint`)}</span></span>
            </label>
          ))}
        </fieldset>
      </>}

      {collecting && loaded.status === "loading" && (
        <p role="status" className="report-problem-loading"><span className="task-status-text">{t("reportProblem.loading")}</span></p>
      )}
      {collecting && loaded.status === "failed" && (
        <div><p role="alert" className="form-warning">{t("reportProblem.previewFailed", { error: loaded.error })}</p>
          <button type="button" data-report-refresh onClick={() => setReload((n) => n + 1)}>{t("reportProblem.refresh")}</button></div>
      )}

      {step === "contents" && preview && <div className="report-problem-contents">
        {!preview.recordingEnabled && <p className="form-warning" data-report-recording-off>{t("reportProblem.recordingOff")}</p>}
        {operationOffered && <label className="settings-field">
          <span className="settings-field-label">{t("reportProblem.operation.label")}</span>
          <select data-report-operation value={preview.operationRecordingId ?? ""}
            onChange={(e) => { const id = e.target.value; if (id) setChoice((current) => ({ ...current, operation: { recordingId: id } })); }}>
            {operation === null && <option value="" disabled>{t("reportProblem.operation.none")}</option>}
            {choosable.map((recording) => <option key={recording.id} value={recording.id}>{recordingLabel(recording)}</option>)}
          </select>
          {operation === null && choice.operation !== null && "relatedKey" in choice.operation && (
            <span className="settings-field-hint" data-report-operation-missing>{t("reportProblem.operation.notFound")}</span>
          )}
        </label>}
        {deviceOffered && tree && <label className="settings-field">
          <span className="settings-field-label">{t("reportProblem.device.label")}</span>
          <select data-report-device value={choice.deviceId ?? ""}
            onChange={(e) => { const value = e.target.value; setChoice((current) => ({ ...current, deviceId: value === "" ? null : Number(value) })); }}>
            <option value="">{t("reportProblem.device.auto")}</option>
            {devices.map((d) => <option key={d.id} value={d.id}>{d.address ?? "—"} · {d.name}</option>)}
          </select>
        </label>}
        {choice.sections.has("clientState") && client.truncated && <p className="settings-field-hint" data-report-client-truncated>
          {t("reportProblem.clientTruncated", { omitted: client.omittedErrorCount, shortened: client.truncatedErrorCount })}</p>}
        <ul className="report-problem-offers">{preview.offers.map(offerRow)}</ul>
        <p className="report-problem-total" data-report-total>{t("reportProblem.total", { size: formatBytes(preview.totalBytes, language) })}</p>
        {preview.exceedsMailHint && <p className="form-warning" data-report-mail-hint>{t("reportProblem.mailHint")}</p>}
        <p className="settings-field-hint">{t("reportProblem.retention")}</p>
        <div className="report-problem-delete">
          <button type="button" data-report-delete onClick={() => { setDeleted(null); setConfirmDelete(true); }}>{t("reportProblem.delete.button")}</button>
          {deleted && ("outcome" in deleted
            ? <p role="status" tabIndex={-1} ref={deleteResultRef} data-report-deleted>
              {t("reportProblem.delete.done", { count: deleted.outcome.removed })}
              {deleted.outcome.keptActive.length > 0 && ` ${t("reportProblem.delete.kept", { count: deleted.outcome.keptActive.length })}`}
            </p>
            : <p role="alert" tabIndex={-1} ref={deleteResultRef} className="form-warning">{t("reportProblem.delete.failed", { error: deleted.error })}</p>)}
        </div>
        {confirmDelete && <section className="project-wizard-discard" role="alert" data-report-delete-confirm>
          <p>{t("reportProblem.delete.question")}</p>
          <div className="new-project-actions">
            <button type="button" onClick={() => setConfirmDelete(false)}>{t("reportProblem.delete.keep")}</button>
            <button type="button" className="danger-action" onClick={() => void removeRecordings()}>{t("reportProblem.delete.confirm")}</button>
          </div>
        </section>}
      </div>}

      {step === "preview" && preview && <div className="report-problem-preview">
        <p className="project-wizard-intro">{t("reportProblem.preview.intro")}</p>
        <p className="report-problem-removed" data-report-removed>{t("reportProblem.removed")}</p>
        <p className="form-warning" data-report-not-removed>{t("reportProblem.notRemoved")}</p>
        <ul className="report-problem-sections">{preview.sections.map((section) => {
          const markers = markerKeys(section.markers);
          const text = texts.get(section.key);
          const open = shown.has(section.key);
          const textId = `report-problem-text-${section.key.replace(/[^A-Za-z0-9_-]/g, "-")}`;
          return <li key={section.key} data-report-preview={section.key}>
            <h4>{t(`reportProblem.section.${section.kind}`)}{section.label && <> · <code>{section.label}</code></>}</h4>
            <p className="report-problem-facts">
              <span data-report-preview-size>{formatBytes(section.bytes, language)}</span>
              {" · "}<span>{t("reportProblem.preview.entries", { count: section.entries })}</span>
            </p>
            <ul className="report-problem-markers" data-report-markers>
              {markers.length === 0 ? <li>{t("reportProblem.preview.noMarkers")}</li> : markers.map((key) => <li key={key}>{t(key)}</li>)}
            </ul>
            {section.files.length > 0 && <p className="settings-field-hint">{t("reportProblem.preview.files", { files: section.files.join(", ") })}</p>}
            {section.notes.length > 0 && <div className="report-problem-notes"><span className="settings-field-hint">{t("reportProblem.preview.notes")}</span>
              <ul>{section.notes.map((note, i) => <li key={i}>{note}</li>)}</ul></div>}
            <button type="button" data-report-show={section.key} aria-expanded={open} aria-controls={open ? textId : undefined}
              onClick={() => void showText(section.key)}>{t(open ? "reportProblem.preview.hide" : "reportProblem.preview.show")}</button>
            {open && (text === undefined || text.status === "loading"
              ? <p role="status" id={textId}>{t("reportProblem.preview.textLoading")}</p>
              : text.status === "failed"
                ? <p role="alert" id={textId} className="form-warning">{t("reportProblem.preview.textFailed", { error: text.error })}</p>
                : <pre className="report-problem-text" id={textId} tabIndex={0} data-report-text={section.key}
                  aria-label={t(`reportProblem.section.${section.kind}`)}>{text.text}</pre>)}
          </li>;
        })}</ul>
        <p className="report-problem-total">{t("reportProblem.total", { size: formatBytes(preview.totalBytes, language) })}</p>
      </div>}

      {step === "send" && preview && <div className="report-problem-send">
        <label className="settings-field">
          <span className="settings-field-label">{t("reportProblem.description.label")}</span>
          <textarea rows={4} maxLength={8000} value={description} data-report-description
            onChange={(e) => { ++epoch.current; setDescription(e.target.value); setConsent(false); setSaved(null); setFileConfirmed(false); setSaving(false); }} />
          <span className="settings-field-hint">{t("reportProblem.description.hint")}</span>
        </label>
        <label className="report-problem-consent">
          <input type="checkbox" data-report-consent checked={consent} disabled={!ready || saving}
            onChange={(e) => { setConsent(e.target.checked); if (!e.target.checked) { setSaved(null); setFileConfirmed(false); } }} />
          {t("reportProblem.consent")}
        </label>
        <div className="report-problem-handoff">
          <button type="button" className="primary-action" data-report-save disabled={!consent || !ready || saving}
            onClick={() => void save()}>{t(saving ? "reportProblem.saving" : "reportProblem.save")}</button>
          {mailDraft !== null
            ? <a className="report-problem-mail" data-report-mail href={mailDraft}
              ref={mailRef}>{t("reportProblem.mail")}</a>
            : <button type="button" data-report-mail disabled aria-describedby="report-problem-mail-after">{t("reportProblem.mail")}</button>}
        </div>
        {saved !== null && <>
          <p role="status" data-report-requested>{t("reportProblem.downloadRequested", { file: saved })}</p>
          <label className="report-problem-consent">
            <input type="checkbox" data-report-file-confirm checked={fileConfirmed} ref={fileConfirmRef}
              onChange={(e) => {
                setFileConfirmed(e.target.checked);
                if (e.target.checked && !confirmationAwarded.current) {
                  confirmationAwarded.current = true;
                  emitAchievementEvent({ type: "debugReportCreated" });
                }
              }} />{t("reportProblem.fileConfirm")}
          </label>
        </>}
        {saveError !== null && <p role="alert" tabIndex={-1} className="form-warning" data-report-save-failed
          ref={saveErrorRef}>{t("reportProblem.saveFailed", { error: saveError })}</p>}
        {mailDraft === null && <p className="settings-field-hint" id="report-problem-mail-after">{t("reportProblem.mailAfterSave")}</p>}
        <p className="form-warning" data-report-mail-note>{t("reportProblem.mailNote")}</p>
        <p className="report-problem-issue">
          <a data-report-issue href={issue.url} target="_blank" rel="noopener noreferrer">{t("reportProblem.issue")}</a>
          {" "}<span className="settings-field-hint">{t("reportProblem.issueHint")}</span>
        </p>
      </div>}

      <div className="new-project-actions report-problem-actions">
        <button type="button" onClick={close}>{t("reportProblem.close")}</button>
        <button type="button" disabled={stepIndex === 0} onClick={() => setStep(STEPS[stepIndex - 1])}>{t("projectWizard.back")}</button>
        {stepIndex < STEPS.length - 1 && (
          <button type="button" className="primary-action" data-report-next
            disabled={stepIndex > 0 && !ready} onClick={() => setStep(STEPS[stepIndex + 1])}>{t("projectWizard.next")}</button>
        )}
      </div>
    </Overlay>
  );
}
