/** Dialog that previews the project documentation and exports or prints the selected sections. */
import { useEffect, useId, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import * as api from "./api";
import type { DocumentationPreview } from "./api";
import {
  DOCUMENTATION_SECTIONS,
  documentationOptionsFor,
  type DocumentationSection,
} from "./documentationOptions";
import { pickSavePath } from "./filePicker";
import { useTranslate } from "./i18n";
import Overlay from "./Overlay";
import { useUiLanguage } from "./uiLanguage";

// The preview renders server-generated HTML (`crates/knx-report`). No
// `allow-scripts`: nothing inside the document may ever run. `allow-same-origin`
// lets this page reach the frame's window to call `print()` on it, and
// `allow-modals` is what the HTML print steps require of a sandboxed
// document before they open the print dialog. The dangerous pairing is
// `allow-scripts` together with `allow-same-origin`, which this avoids.
export const PREVIEW_SANDBOX = "allow-same-origin allow-modals";

type PreviewState =
  | { status: "loading" }
  | { status: "ready"; preview: DocumentationPreview }
  | { status: "failed"; message: string };

export default function DocumentationDialog(props: {
  onSummary: (message: string) => void;
  onError: (e: unknown) => void;
  onClearErrors: () => void;
  onClose: () => void;
}) {
  const { onSummary, onError, onClearErrors, onClose } = props;
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const titleId = useId();
  const warningsId = useId();
  const frameRef = useRef<HTMLIFrameElement | null>(null);
  const [selected, setSelected] = useState<ReadonlySet<DocumentationSection>>(
    () => new Set(DOCUMENTATION_SECTIONS),
  );
  const [previewState, setPreviewState] = useState<PreviewState>({ status: "loading" });

  // One object feeds both the preview and the export, so the file written
  // is the document that was previewed.
  const options = useMemo(
    () => documentationOptionsFor(selected, uiLanguage),
    [selected, uiLanguage],
  );

  useEffect(() => {
    // A response for a selection that has since changed must not replace
    // the preview of the current one.
    let current = true;
    setPreviewState({ status: "loading" });
    api.previewDocumentation(options).then(
      (preview) => {
        if (current) setPreviewState({ status: "ready", preview });
      },
      (e: unknown) => {
        if (current) setPreviewState({ status: "failed", message: api.errorMessage(e) });
      },
    );
    return () => {
      current = false;
    };
  }, [options]);

  function toggleSection(section: DocumentationSection) {
    setSelected((previous) => {
      const next = new Set(previous);
      if (!next.delete(section)) next.add(section);
      return next;
    });
  }

  function printPreview() {
    try {
      frameRef.current?.contentWindow?.print();
    } catch (e) {
      onError(e);
    }
  }

  async function exportDocumentation() {
    const filter = [{ name: t("documentationExport.filterName"), extensions: ["html"] }];
    const path = await pickSavePath(filter, "project-documentation.html");
    if (!path) return;
    // Cleared before the export runs, so a stale error toast never sits on
    // screen through a subsequent success.
    onClearErrors();
    try {
      const { warnings } = await api.exportDocumentation(path, options);
      const count = warnings.length;
      onSummary(
        count === 0
          ? t("documentationExport.summaryNone")
          : t("documentationExport.summaryWithWarnings", { count }),
      );
      onClose();
    } catch (e) {
      onError(e);
    }
  }

  const preview = previewState.status === "ready" ? previewState.preview : null;

  // Portalled to `<body>`: the button that opens this dialog lives in the
  // File menu's `<details>`, which closes on activation and would hide
  // anything still rendered inside it.
  return createPortal(
    <Overlay labelledBy={titleId} className="documentation-dialog" onClose={onClose}>
      <h2 id={titleId} className="documentation-dialog-title">
        {t("documentationExport.dialogTitle")}
      </h2>
      <fieldset className="documentation-sections">
        <legend>{t("documentationExport.sectionsLegend")}</legend>
        {DOCUMENTATION_SECTIONS.map((section) => (
          <label key={section}>
            <input
              type="checkbox"
              name="documentationSection"
              value={section}
              checked={selected.has(section)}
              onChange={() => toggleSection(section)}
            />
            {t(`documentationExport.section.${section}`)}
          </label>
        ))}
        <p className="documentation-hint">{t("documentationExport.alwaysIncluded")}</p>
      </fieldset>
      <div className="documentation-body">
        <div className="documentation-preview" aria-busy={previewState.status === "loading"}>
          {previewState.status === "loading" && (
            <p className="documentation-status">{t("documentationExport.previewLoading")}</p>
          )}
          {previewState.status === "failed" && (
            <p className="documentation-status" role="alert">
              {t("documentationExport.previewError", { message: previewState.message })}
            </p>
          )}
          {preview && (
            <iframe
              ref={frameRef}
              className="documentation-frame"
              title={t("documentationExport.previewTitle")}
              sandbox={PREVIEW_SANDBOX}
              srcDoc={preview.html}
            />
          )}
        </div>
        <aside className="documentation-warnings" aria-labelledby={warningsId}>
          <h3 id={warningsId}>
            {t("documentationExport.warningsHeading", { count: preview?.warnings.length ?? 0 })}
          </h3>
          {preview && preview.warnings.length === 0 && (
            <p className="documentation-status">{t("documentationExport.warningsNone")}</p>
          )}
          {preview && preview.warnings.length > 0 && (
            <ul className="documentation-warning-list">
              {preview.warnings.map((warning, index) => (
                <li key={index}>
                  <strong>{warning.location}</strong> {warning.detail}
                </li>
              ))}
            </ul>
          )}
        </aside>
      </div>
      <div className="documentation-actions">
        <button onClick={printPreview} disabled={!preview}>
          {t("documentationExport.print")}
        </button>
        <button onClick={exportDocumentation}>{t("documentationExport.export")}</button>
        <button onClick={onClose}>{t("documentationExport.close")}</button>
      </div>
    </Overlay>,
    document.body,
  );
}
