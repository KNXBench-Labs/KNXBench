/** File-menu entry and dialog collecting a debug report into a local zip or a GitHub issue. */
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import * as api from "./api";
import { buildIssueUrl } from "./githubIssue";
import { isTauri, pickSavePath } from "./filePicker";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import { useThemeId } from "./theme";
import { useUiLanguage } from "./uiLanguage";
import { version as packageVersion } from "../package.json";

// T29. The user asked for this after a testing session: somewhere to put
// "here is what went wrong" that does not mean copying six panels by hand.
//
// Two exits, both of which the user watches happen:
//
// * **Save zip** — writes the bundle to a path picked in the ordinary save
//   dialog. Local, always. There is no upload in this application, no
//   telemetry, and no anonymous usage statistics to opt out of.
// * **Open a GitHub issue** — opens a prefilled `issues/new` page in the
//   browser and stops. No token, no credential, no `POST`: the application
//   cannot file an issue, only offer one for the user to read and submit.
//
// Before either, the dialog says what the bundle will contain, file by
// file, in the user's own language — and says plainly which file is *not*
// redacted, because a half-redacted bundle the user believes is clean is
// worse than one they know is not.
//
// Same precedent as `DocumentationExportButton.tsx`: owns its `./api`,
// `./filePicker` and settings-hook calls outright, and takes only the
// toast plumbing `App.tsx` already owns as props. That keeps `App.tsx`'s
// diff to one import and one element.

/** Every artifact the bundle can hold, in the order the server writes them,
 * paired with the message key describing it and the flag that decides
 * whether it is there at all. `null` means mandatory. */
const CONTENTS = [
  { file: "report.md", key: "debugReport.contents.report", flag: null },
  { file: "environment.json", key: "debugReport.contents.environment", flag: null },
  { file: "log.json", key: "debugReport.contents.log", flag: "log" },
  {
    file: "project-summary.json",
    key: "debugReport.contents.projectSummary",
    flag: "projectSummary",
  },
  { file: "bus-telegrams.json", key: "debugReport.contents.busTelegrams", flag: "busTelegrams" },
] as const;

export default function DebugReportButton(props: {
  onSummary: (message: string) => void;
  onError: (e: unknown) => void;
  onClearErrors: () => void;
}) {
  const { onSummary, onError, onClearErrors } = props;
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const wasOpen = useRef(false);
  // The File menu closes on activation, hiding the opening button. Restore
  // focus to its visible summary after Overlay restores its previous focus.
  useEffect(() => {
    if (wasOpen.current && !open) {
      buttonRef.current?.closest("details")?.querySelector("summary")?.focus();
    }
    wasOpen.current = open;
  }, [open]);

  return (
    <>
      <button ref={buttonRef} onClick={() => setOpen(true)}>{t("debugReport.button")}</button>
      {open && (
        <DebugReportDialog
          onClose={() => setOpen(false)}
          onSummary={onSummary}
          onError={onError}
          onClearErrors={onClearErrors}
        />
      )}
    </>
  );
}

function DebugReportDialog(props: {
  onClose: () => void;
  onSummary: (message: string) => void;
  onError: (e: unknown) => void;
  onClearErrors: () => void;
}) {
  const { onClose, onSummary, onError, onClearErrors } = props;
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const [themeId] = useThemeId();
  const [description, setDescription] = useState("");
  const [includeLog, setIncludeLog] = useState(true);
  const [includeProjectSummary, setIncludeProjectSummary] = useState(false);
  const [includeBusTelegrams, setIncludeBusTelegrams] = useState(false);
  const [busy, setBusy] = useState(false);
  const descriptionRef = useRef<HTMLTextAreaElement>(null);

  const flags = {
    log: includeLog,
    projectSummary: includeProjectSummary,
    busTelegrams: includeBusTelegrams,
  };

  function requestFor(path: string | null): api.DebugReportRequest {
    return {
      path,
      description,
      includeLog,
      includeProjectSummary,
      includeBusTelegrams,
      client: {
        appVersion: packageVersion,
        // Which window this is running in. The desktop shell and a browser
        // tab fail differently often enough that it is worth one word.
        shell: isTauri() ? "tauri" : "browser",
        uiLanguage,
        theme: themeId,
      },
    };
  }

  async function saveZip() {
    const path = await pickSavePath(
      // Rebuilt per call, never hoisted to module scope: a module-level
      // `const` would freeze the filter name in whichever language was
      // active at import time. Same trap `DocumentationExportButton` names.
      [{ name: t("debugReport.filterName"), extensions: ["zip"] }],
      "knxbench-debug-report.zip",
    );
    if (!path) return;
    // Cleared before the operation, not after — so a stale error toast
    // never sits on screen through a subsequent success.
    onClearErrors();
    setBusy(true);
    try {
      const report = await api.createDebugReport(requestFor(path));
      onSummary(t("debugReport.saved", { count: report.files.length }));
      onClose();
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }

  async function openIssue() {
    onClearErrors();
    setBusy(true);
    try {
      // `path: null`: this builds the bundle in memory to get `report.md`'s
      // text and writes nothing. Offering an issue is not a reason to drop
      // a file on someone's disk.
      const report = await api.createDebugReport(requestFor(null));
      const { url, truncated } = buildIssueUrl(t("debugReport.issueTitle"), report.reportMarkdown);
      window.open(url, "_blank", "noopener,noreferrer");
      onSummary(t(truncated ? "debugReport.issueTruncated" : "debugReport.issueOpened"));
      onClose();
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  }

  // This button lives in File's closing <details>. Keep the shared overlay
  // outside that hidden subtree, as the documentation dialog already does.
  return createPortal(
    <Overlay
      labelledBy="debug-report-title"
      className="debug-report-panel"
      resizable={{ width: 820, height: 640 }}
      onClose={onClose}
      initialFocusRef={descriptionRef}
    >
      <h2 className="settings-panel-title" id="debug-report-title">
        {t("debugReport.title")}
      </h2>
      <p className="debug-report-intro">{t("debugReport.intro")}</p>

      <label className="settings-field">
        <span className="settings-field-label">{t("debugReport.descriptionLabel")}</span>
        <textarea
          ref={descriptionRef}
          rows={4}
          value={description}
          aria-label={t("debugReport.descriptionLabel")}
          placeholder={t("debugReport.descriptionPlaceholder")}
          onChange={(e) => setDescription(e.target.value)}
        />
        <span className="settings-field-hint">{t("debugReport.descriptionHint")}</span>
      </label>

      <Checkbox
        checked={includeLog}
        onChange={setIncludeLog}
        label={t("debugReport.include.log.label")}
        hint={t("debugReport.include.log.hint")}
      />
      <Checkbox
        checked={includeProjectSummary}
        onChange={setIncludeProjectSummary}
        label={t("debugReport.include.projectSummary.label")}
        hint={t("debugReport.include.projectSummary.hint")}
      />
      <Checkbox
        checked={includeBusTelegrams}
        onChange={setIncludeBusTelegrams}
        label={t("debugReport.include.busTelegrams.label")}
        hint={t("debugReport.include.busTelegrams.hint")}
      />

      <section className="debug-report-contents">
        <h3>{t("debugReport.contentsTitle")}</h3>
        <ul>
          {CONTENTS.filter((entry) => entry.flag === null || flags[entry.flag]).map((entry) => (
            <li key={entry.file}>{t(entry.key)}</li>
          ))}
        </ul>
        <p>{t("debugReport.privacyRedacted")}</p>
        {includeBusTelegrams && (
          // Shown only when it applies, and shown before the buttons: this
          // is the one file the redaction pass deliberately leaves alone.
          <p className="debug-report-warning">{t("debugReport.privacyTelegrams")}</p>
        )}
      </section>

      <div className="new-project-actions">
        <button type="button" onClick={onClose}>
          {t("debugReport.close")}
        </button>
        <button type="button" disabled={busy} onClick={() => void openIssue()}>
          {busy ? t("debugReport.busy") : t("debugReport.openIssue")}
        </button>
        <button type="button" className="primary-action" disabled={busy} onClick={() => void saveZip()}>
          {busy ? t("debugReport.busy") : t("debugReport.save")}
        </button>
      </div>
    </Overlay>,
    document.body,
  );
}

function Checkbox(props: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
  hint: string;
}) {
  const { checked, onChange, label, hint } = props;
  return (
    <label className="settings-field debug-report-option">
      <span className="settings-field-label">
        <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
        {label}
      </span>
      <span className="settings-field-hint">{hint}</span>
    </label>
  );
}
