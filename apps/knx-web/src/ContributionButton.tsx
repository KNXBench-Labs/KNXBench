/** Explicit own-instance analysis, disclosure preview and manual contribution handoff. */
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import Overlay from "./Overlay";
import { useTranslate, type MessageKey } from "./i18n";
import { getActiveUiLanguage } from "./uiLanguage";
import * as api from "./contributionApi";
import "./contribution.css";
import ProcedureResolutionView from "./ProcedureResolutionView";

export const CONTRIBUTION_REPOSITORY = "https://github.com/KNXBench-Labs/KNXBench";
export default function ContributionButton() {
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const previous = useRef(false);
  useEffect(() => {
    if (previous.current && !open) trigger.current?.closest("details")?.querySelector("summary")?.focus();
    previous.current = open;
  }, [open]);
  return <><button ref={trigger} onClick={() => setOpen(true)}>{t("contribution.button")}</button>
    {open && createPortal(<ContributionDialog onClose={() => setOpen(false)} />, document.body)}</>;
}
function ContributionDialog({ onClose }: { onClose: () => void }) {
  const t = useTranslate();
  const [file, setFile] = useState<File | null>(null);
  const [analysis, setAnalysis] = useState<api.ContributionAnalysis | null>(null);
  const [audience, setAudience] = useState<"public" | "private">("public");
  const [sampleIds, setSampleIds] = useState<string[]>([]);
  const [includeOriginal, setOriginal] = useState(false);
  const [originalConsent, setOriginalConsent] = useState(false);
  const [consent, setConsent] = useState(false);
  const [preview, setPreview] = useState<api.ContributionPreview | null>(null);
  const [downloadRequested, setDownloadRequested] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const epoch = useRef(0);
  const controller = useRef<AbortController | null>(null);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => () => { ++epoch.current; controller.current?.abort(); }, []);
  const valid = file && /\.(knxproj|knxprod)$/i.test(file.name) && file.size > 0 && file.size <= 32 * 1024 * 1024;
  function invalidate() { ++epoch.current; controller.current?.abort(); setPreview(null); setConsent(false); setDownloadRequested(false); setError(""); }
  function options(): api.ContributionOptions { return { audience, sampleIds, includeOriginal, originalConsent, consent }; }
  async function run(operation: "analyze" | "preview" | "export") {
    if (!file || !valid || busy) return;
    const id = ++epoch.current;
    const abort = new AbortController(); controller.current = abort;
    setBusy(true); setError("");
    try {
      if (operation === "analyze") {
        setAnalysis(null); setPreview(null); setConsent(false); setDownloadRequested(false);
        const result = await api.analyzeContribution(file, abort.signal);
        if (epoch.current === id) setAnalysis(result);
      } else if (operation === "preview") {
        setPreview(null); setConsent(false); setDownloadRequested(false);
        const result = await api.previewContribution(file, { ...options(), consent: false }, abort.signal);
        if (epoch.current === id) setPreview(result);
      } else {
        if (!preview || !consent) return;
        const blob = await api.exportContribution(file, { ...options(), expectedManifestSha256: preview.manifestSha256 }, abort.signal);
        if (epoch.current !== id) return;
        const url = URL.createObjectURL(blob);
        const anchor = document.createElement("a"); anchor.href = url; anchor.download = "knxbench-evidence.zip";
        document.body.appendChild(anchor); anchor.click(); anchor.remove();
        window.setTimeout(() => URL.revokeObjectURL(url), 10_000);
        setDownloadRequested(true);
      }
    } catch (e) { if (epoch.current === id && !abort.signal.aborted) setError(e instanceof Error ? e.message : String(e)); }
    finally { if (epoch.current === id) setBusy(false); }
  }
  function label(kind: "status" | "category", value: string) {
    const allowed = kind === "status" ? ["measured", "complete", "partial", "refused", "unavailable", "not-examined"] : ["unknown", "unsupported", "inference", "retained", "conflict", "refused", "untested", "excluded", "no-address", "observation"];
    return allowed.includes(value) ? t(`contribution.${kind}.${value}` as MessageKey) : value;
  }
  const actions = { analyze: t("contribution.analyze"), preview: t("contribution.preview"), download: t("contribution.export"), github: t("contribution.github") };
  const next: MessageKey = busy ? "contribution.next.busy" : !valid ? "contribution.next.select" : !analysis ? "contribution.next.analyze" : includeOriginal && !originalConsent ? "contribution.next.original" : !preview ? "contribution.next.preview" : !consent ? "contribution.next.consent" : !downloadRequested ? "contribution.next.download" : audience === "public" ? "contribution.next.public" : "contribution.next.private";
  const issueQuery = new URLSearchParams({ template: "analysis.yml", ...(analysis ? { "analyzer-version": analysis.analyzerVersion } : {}) });
  return <Overlay labelledBy="contribution-title" className="contribution-panel" initialFocusRef={input} resizable={{ width: 900, height: 720 }} onClose={onClose}>
    <div className="contribution-header"><h2 id="contribution-title">{t("contribution.title")}</h2><button onClick={onClose}>{t("contribution.close")}</button></div>
    <p>{t("contribution.intro")}</p>
    <p data-contribution-next aria-live="polite" className="contribution-warning">{t(next, actions)}</p>
    <details data-contribution-guide>
      <summary>{t("contribution.help.title")}</summary>
      <p>{t("contribution.help.intro")}</p>
      <ol>{[1, 2, 3, 4, 5, 6].map(step => <li key={step}>
        <strong>{t(`contribution.help.step${step}.title` as MessageKey)}</strong>
        <p>{t(`contribution.help.step${step}.body` as MessageKey, actions)}</p>
      </li>)}</ol>
      <h3>{t("contribution.help.statusTitle")}</h3><p>{t("contribution.help.statusBody")}</p>
      <h3>{t("contribution.help.problemTitle")}</h3><p>{t("contribution.help.problemBody")}</p>
      <h3>{t("contribution.help.privateTitle")}</h3><p>{t("contribution.help.privateBody")}</p>
      <a href={getActiveUiLanguage() === "de" ? `${CONTRIBUTION_REPOSITORY}/blob/main/docs/contribution-intake/DEUTSCH.md` : `${CONTRIBUTION_REPOSITORY}/blob/main/docs/contribution-intake/README.md`} target="_blank" rel="noopener noreferrer">{t("contribution.help.fullGuide")}</a>
    </details>
    <label className="settings-field"><span>{t("contribution.file")}</span>
      <input ref={input} type="file" accept=".knxproj,.knxprod" disabled={busy} onChange={e => {
        invalidate(); setFile(e.target.files?.[0] ?? null); setAnalysis(null); setSampleIds([]); setOriginal(false); setOriginalConsent(false);
      }} />
    </label>
    {file && !valid && <p role="alert">{t("contribution.invalid")}</p>}
    <div className="contribution-actions"><button disabled={!valid || busy} onClick={() => void run("analyze")}>{t("contribution.analyze")}</button></div>
    {busy && <p role="status">{t("contribution.busy")}</p>}
    {error && <p role="alert">{error}</p>}
    {analysis && <>
      <h3>{t("contribution.checks")}</h3>
      <p>{analysis.kind} · schema {analysis.scheme ?? "?"} · {label("status", analysis.status)} · {analysis.analyzerVersion}</p>
      <ul>{analysis.checks.map(check => <li key={check.name}><strong>{check.name}: {label("status", check.status)}</strong><p>{check.detail}</p></li>)}</ul>
      <details><summary>{t("contribution.checks")} — counts</summary><pre>{JSON.stringify(analysis.metrics, null, 2)}</pre></details>
      <ProcedureResolutionView resolutions={analysis.procedureResolutions ?? []} />
      <h3>{t("contribution.findings")}</h3>
      {!analysis.findings.length && <p>{t("contribution.empty")}</p>}
      <ul>{analysis.findings.map(finding => <li key={finding.id}><strong>{label("category", finding.category)} · {finding.stage} · {finding.name ?? ""} × {finding.occurrences}</strong>
        <p>{finding.detail}</p><code>{finding.xpath}</code></li>)}</ul>
      <details><summary>{t("contribution.structure")} ({analysis.structure.length})</summary><pre>{JSON.stringify(analysis.structure, null, 2)}</pre></details>
      <label className="settings-field"><span>{t("contribution.audience")}</span>
        <select disabled={busy} value={audience} onChange={e => { invalidate(); setAudience(e.target.value as "public" | "private"); setOriginal(false); setOriginalConsent(false); }}>
          <option value="public">{t("contribution.public")}</option><option value="private">{t("contribution.private")}</option>
        </select>
      </label>
      <p className="contribution-warning">{t("contribution.warning")}</p>
      <fieldset disabled={busy}><legend>{t("contribution.samples")}</legend>{analysis.members.map(m => <label key={m.id} className="contribution-sample">
        <input type="checkbox" data-sample={m.id} disabled={!m.sampleAllowed || (!sampleIds.includes(m.id) && sampleIds.length >= 16)} checked={sampleIds.includes(m.id)} onChange={e => {
          invalidate(); setSampleIds(e.target.checked ? [...sampleIds, m.id] : sampleIds.filter(id => id !== m.id));
        }} /><span>{m.path} ({m.size} bytes)<small>{m.reason}</small></span>
      </label>)}</fieldset>
      {audience === "private" && <>
        <p>{t("contribution.emailWarning")}</p>
        {!analysis.originalAllowed && <p>{t("contribution.originalBlocked")}</p>}
        <label className="contribution-sample"><input type="checkbox" disabled={busy || !analysis.originalAllowed} checked={includeOriginal} onChange={e => { invalidate(); setOriginal(e.target.checked); setOriginalConsent(false); }} />{t("contribution.original")}</label>
        {includeOriginal && <label className="contribution-sample"><input type="checkbox" disabled={busy} checked={originalConsent} onChange={e => { invalidate(); setOriginalConsent(e.target.checked); }} />{t("contribution.originalConsent")}</label>}
      </>}
      <div className="contribution-actions"><button disabled={busy || (includeOriginal && !originalConsent)} onClick={() => void run("preview")}>{t("contribution.preview")}</button></div>
      {preview && <section aria-label={t("contribution.previewTitle")}><h3>{t("contribution.previewTitle")}</h3>
        <details open><summary>manifest.json</summary><pre>{JSON.stringify(preview.manifest, null, 2)}</pre></details>
        {preview.files.map(f => <details key={f.path} open><summary>{f.path}</summary><pre>{f.text ?? t("contribution.originalConsent")}</pre></details>)}
        <label className="contribution-sample"><input type="checkbox" data-consent="export" disabled={busy} checked={consent} onChange={e => setConsent(e.target.checked)} />{t("contribution.consent")}</label>
      </section>}
      <div className="contribution-actions"><button disabled={!preview || !consent || busy} onClick={() => void run("export")}>{t("contribution.export")}</button>
        {downloadRequested && consent && (audience === "public" ?
          <a href={`${CONTRIBUTION_REPOSITORY}/issues/new?${issueQuery}`} target="_blank" rel="noopener noreferrer">{t("contribution.github")}</a> :
          <a href="mailto:contribute@knxbench.com?subject=KNXBench%20private%20contribution" target="_blank" rel="noopener noreferrer">{t("contribution.email")}</a>)}</div>
      {downloadRequested && <p role="status">{t("contribution.downloaded")}</p>}
    </>}
  </Overlay>;
}
