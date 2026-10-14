/** Explains an unresolved application program and restores missing product data (#21). */
import { useState } from "react";
import * as api from "./api";
import type { ManufacturerFileRestore, UnresolvedProgram } from "./api";
import { pickOpenPath } from "./filePicker";
import { useTranslate } from "./i18n";
import { restoreChangedSomething, unresolvedProgramState } from "./unresolvedProgram";

/**
 * Rendered above the parameter sections whether or not the program
 * resolved, so a restore's summary stays visible after the reload that
 * makes the program appear. The caller keys it by device.
 */
export default function UnresolvedProgramNotice(props: {
  /** `true` while the device's program does not resolve. */
  unresolved: boolean;
  info: UnresolvedProgram | null;
  /** Re-reads the panel after files were installed. */
  onRestored: () => Promise<void> | void;
}) {
  const t = useTranslate();
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ManufacturerFileRestore | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { info, unresolved } = props;

  const restore = async () => {
    const path = await pickOpenPath([{ name: t("app.filterName.etsProject"), extensions: ["knxproj"] }]);
    if (!path) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const restored = await api.restoreManufacturerFiles(path);
      setResult(restored);
      if (restoreChangedSomething(restored)) await props.onRestored();
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const state = info ? unresolvedProgramState(info) : null;
  return (
    <>
      {unresolved && !info && <p className="inspector-description">{t("parameters.noProgram")}</p>}
      {unresolved && info && state && (
        <div className="unresolved-program">
          <p className="inspector-description">
            {state === "restorable"
              ? t("parameters.unresolved.restorable", { count: info.missingFiles })
              : t(state === "noDatabase" ? "parameters.unresolved.noDatabase" : "parameters.unresolved.notInstalled")}
          </p>
          <p className="inspector-description unresolved-program-ref">
            {t("parameters.unresolved.programRef")} <code>{info.programRef}</code>
          </p>
          {state === "restorable" && (
            <button type="button" className="unresolved-program-restore" disabled={busy} onClick={() => void restore()}>
              {busy ? <span className="task-status-text">{t("parameters.unresolved.restoring")}</span>
                : t("parameters.unresolved.restore")}
            </button>
          )}
        </div>
      )}
      {error && <p className="field-error" role="alert">{error}</p>}
      {result && <RestoreSummary result={result} />}
    </>
  );
}

function RestoreSummary({ result }: { result: ManufacturerFileRestore }) {
  const t = useTranslate();
  const installed = result.installed.length;
  return (
    <div className="unresolved-program-result" role="status">
      <p className="inspector-description">
        {installed > 0 ? t("parameters.unresolved.installed", { count: installed })
          : t("parameters.unresolved.nothingInstalled")}
        {installed > 0 && result.stillMissing === 0 && <> {t("parameters.unresolved.saveHint")}</>}
      </p>
      {result.stillMissing > 0 && (
        <p className="inspector-description">{t("parameters.unresolved.stillMissing", { count: result.stillMissing })}</p>
      )}
      {result.mismatched.length > 0 && (
        <p className="inspector-description">{t("parameters.unresolved.mismatched", { paths: result.mismatched.join(", ") })}</p>
      )}
      {result.notInArchive.length > 0 && (
        <p className="inspector-description">{t("parameters.unresolved.notInArchive", { paths: result.notInArchive.join(", ") })}</p>
      )}
      {result.failed.map((failure) => (
        <p key={failure.sourcePath} className="field-error">
          {t("parameters.unresolved.failed", { path: failure.sourcePath, reason: failure.reason })}
        </p>
      ))}
    </div>
  );
}
