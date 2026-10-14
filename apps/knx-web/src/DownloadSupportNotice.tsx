/** Shows actionable support help before any plan or download report exists. */
import { useRef, useState } from "react";
import { readinessReason } from "./downloadReadiness";
import type { SupportGuidance } from "./downloadSupportGuidance";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import "./DownloadSupportNotice.css";

/** Trusted application-owned targets; no project-derived metadata leaves in these links. */
export function supportTargets(language: string) {
  return {
    instructions: `https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/contribution-intake/${language === "de" ? "DEUTSCH" : "README"}.md`,
    issue: "https://github.com/KNXBench-Labs/KNXBench/issues/new?template=analysis.yml",
    mail: "mailto:issue@knxbench.com?subject=KNXBench%20device%20programming%20support%20gap",
  };
}

export default function DownloadSupportNotice({ guidance, failedRequest = false }: { guidance: SupportGuidance; failedRequest?: boolean }) {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const targets = supportTargets(language);
  const field = useRef<HTMLTextAreaElement>(null);
  const [copyResult, setCopyResult] = useState<{ text: string; state: "copied" | "failed" } | null>(null);
  const reason = guidance.kind === "unsupported" || guidance.kind === "excluded"
    ? readinessReason(guidance.kind === "excluded" ? { kind: "excluded" }
      : { kind: "unsupported", category: guidance.category }, t)
    : t(guidance.kind === "transient" ? "deviceDownload.help.reason.transient"
      : guidance.action === "installData" ? "deviceDownload.help.reason.missingData"
      : guidance.action === "openProject" ? "deviceDownload.help.reason.noProject"
      : guidance.code === "downloadInvalidAddress" ? "deviceDownload.help.reason.address"
      : guidance.action === "fixGateway" ? "deviceDownload.help.reason.gateway"
      : guidance.action === "prepareAgain" ? "deviceDownload.help.reason.plan"
      : guidance.action === "confirmRequest" ? "deviceDownload.help.reason.confirmation"
      : guidance.action === "checkKey" ? "deviceDownload.help.reason.key"
      : guidance.action === "releaseResource" ? "deviceDownload.help.reason.busy"
      : guidance.code === "downloadPartialRefused" ? "deviceDownload.help.reason.partial"
      : "deviceDownload.help.reason.unknown");
  const technical = [guidance.code, guidance.category !== guidance.code ? guidance.category : null, guidance.detail]
    .filter((value): value is string => value !== null).join("\n");
  const copied = copyResult?.text === technical ? copyResult.state : null;
  async function copy() {
    try { await navigator.clipboard.writeText(technical); setCopyResult({ text: technical, state: "copied" }); }
    catch {
      setCopyResult({ text: technical, state: "failed" });
      if (field.current?.value === technical) { field.current.focus(); field.current.select(); }
    }
  }
  return <section className="download-support-notice" data-support-guidance={guidance.kind}
    aria-label={t("deviceDownload.help.title")} role={failedRequest ? "alert" : undefined}>
    <h3>{t("deviceDownload.help.title")}</h3>
    <p role="status" className="form-warning">{reason}</p>
    <p data-next-action={guidance.action}>{t(`deviceDownload.help.action.${guidance.action}`)}</p>
    <p>{t("deviceDownload.help.noBypass")}</p>
    <div className="download-support-actions">
      <a data-support-instructions href={targets.instructions} target="_blank" rel="noopener noreferrer">{t("deviceDownload.help.instructions")}</a>
      <a data-support-issue href={targets.issue} target="_blank" rel="noopener noreferrer">{t("deviceDownload.help.issue")}</a>
      <a data-support-mail href={targets.mail}>{t("deviceDownload.help.mail")}</a>
    </div>
    <p className="download-support-note">{t("deviceDownload.help.privacy")}</p>
    <details>
      <summary>{t("deviceDownload.help.details")}</summary>
      <p>{t("deviceDownload.help.localDetail")}</p>
      <label>{t("deviceDownload.help.details")}
        <textarea ref={field} readOnly spellCheck={false} value={technical || t("deviceDownload.help.noDetail")} />
      </label>
      <button type="button" data-copy-technical disabled={technical === ""} onClick={() => void copy()}>{t("deviceDownload.help.copy")}</button>
      {copied && <p role={copied === "failed" ? "alert" : "status"}>{t(`deviceDownload.help.${copied}`)}</p>}
    </details>
  </section>;
}
