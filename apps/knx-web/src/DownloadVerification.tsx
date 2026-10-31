/** "Help verify this device": explains, checks and hands over a download report (ADR-0121). */
// Shown once a download has ended. The minimised report is built by the
// server; this view only fetches it on request and offers the file and the
// public issue form. The full diagnostic bundle for mail comes from the
// Report problem wizard, preselected with this run (DIAG-03). Nothing is
// sent automatically.

import { useState } from "react";
import * as api from "./api";
import { checkKey, checklist, runCompleted } from "./downloadVerification";
import { useTranslate } from "./i18n";
import { downloadLocalJson } from "./localJsonDownload";
import type { ReportProblemRequest } from "./reportProblem";
import { ReportProblemButton } from "./ReportProblemEntry";

interface DownloadVerificationProps {
  /** The ended download the report is about. */
  downloadId: number;
  /** Whether that run completed with every block written. */
  completed: boolean;
  /** The Report problem shortcut for this run. */
  report: ReportProblemRequest;
}

type Loaded =
  | { state: "idle" }
  | { state: "loading" }
  | { state: "finishing" }
  | { state: "failed"; error: string }
  | { state: "ready"; response: api.DeviceDownloadReportResponse };

export default function DownloadVerification({ downloadId, completed, report: reportRequest }: DownloadVerificationProps) {
  const t = useTranslate();
  const [loaded, setLoaded] = useState<Loaded>({ state: "idle" });
  const [ticked, setTicked] = useState<ReadonlySet<string>>(() => new Set());

  async function create() {
    setLoaded({ state: "loading" });
    try {
      setLoaded({ state: "ready", response: await api.getDeviceDownloadReport(downloadId) });
    } catch (reason) {
      setLoaded(api.errorStatus(reason) === 409
        ? { state: "finishing" }
        : { state: "failed", error: api.errorMessage(reason) });
    }
  }

  const response = loaded.state === "ready" ? loaded.response : null;
  const report = response?.report ?? null;
  // Until the report is there, the run's own status decides the wording.
  const done = report === null ? completed : runCompleted(report);
  const intro = !done ? "deviceDownload.verify.intro.failed"
    : report?.support === "verified" ? "deviceDownload.verify.intro.verified" : "deviceDownload.verify.intro.untested";
  const checks = report === null ? [] : checklist(report);

  return (
    <section className="device-download-verify" aria-label={t("deviceDownload.verify.title")}>
      <h3>{t("deviceDownload.verify.title")}</h3>
      <p>{t(intro)}</p>
      <h4>{t("deviceDownload.verify.steps.title")}</h4>
      <ol className="device-download-verify-steps">
        {done && <li>{t("deviceDownload.verify.step.check")}</li>}
        <li>{t("deviceDownload.verify.step.create")}</li>
        <li>{t("deviceDownload.verify.step.save")}</li>
        <li>{t("deviceDownload.verify.step.send")}</li>
        <li>{t("deviceDownload.verify.step.review")}</li>
      </ol>
      {report === null && (
        <div className="device-download-verify-actions">
          <button type="button" data-verify-create disabled={loaded.state === "loading"} onClick={() => void create()}>
            {t(loaded.state === "loading" ? "deviceDownload.verify.creating" : "deviceDownload.verify.create")}
          </button>
          <ReportProblemButton from="verify" request={reportRequest} />
        </div>
      )}
      {loaded.state === "finishing" && <p className="form-warning" role="status">{t("deviceDownload.verify.stillFinishing")}</p>}
      {loaded.state === "failed" && (
        <p className="form-error" role="alert">{t("deviceDownload.verify.unavailable", { error: loaded.error })}</p>
      )}
      {response !== null && (
        <>
          {checks.length > 0 && (
            <fieldset className="device-download-verify-checklist">
              <legend>{t("deviceDownload.verify.checklist.title")}</legend>
              {checks.map((id) => (
                <label key={id} data-check={id}>
                  <input type="checkbox" checked={ticked.has(id)}
                    onChange={() => setTicked((current) => {
                      const next = new Set(current);
                      if (!next.delete(id)) next.add(id);
                      return next;
                    })} />
                  {t(checkKey(id))}
                </label>
              ))}
              <p className="device-download-verify-note">{t("deviceDownload.verify.checklist.note")}</p>
            </fieldset>
          )}
          <p className="device-download-verify-privacy">{t("deviceDownload.verify.privacy")}</p>
          <div className="device-download-verify-actions">
            <button type="button" data-verify-save
              onClick={() => downloadLocalJson(`${JSON.stringify(response.report, null, 2)}\n`, response.fileName)}>
              {t("deviceDownload.verify.save")}
            </button>
            <a className="device-download-verify-link" data-verify-issue href={response.issueUrl} target="_blank" rel="noopener noreferrer">
              {t("deviceDownload.verify.issue")}
            </a>
            <ReportProblemButton from="verify" request={reportRequest} />
          </div>
          <ul className="device-download-verify-hints">
            <li>{t("deviceDownload.verify.issueHint")}</li>
            <li>{t("deviceDownload.verify.mailHint")}</li>
          </ul>
          <details>
            <summary>{t("deviceDownload.verify.summary")}</summary>
            <pre className="device-download-verify-summary">{response.summary}</pre>
          </details>
        </>
      )}
    </section>
  );
}
