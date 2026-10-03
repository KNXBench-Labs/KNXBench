/** Appearance file admission delegates preview to the existing root theme owner. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useEffect, useRef, useState } from "react";
import { useTranslate, type MessageKey } from "./i18n";
import { getAcknowledgedSettings, getSetting, SettingsMutationError, useSettingsRevision } from "./settingsStore";
import { exportThemePack, exportThemeRecovery, readThemePackFile } from "./themePackFiles";
import type { ThemePackFileDiagnostic } from "./themePackFiles";
import ThemePackDiagnostic from "./ThemePackDiagnostic";
import { canonicalJson } from "./canonicalJson";
import { commitThemeMutation, planThemeInstallation, planThemeRemoval, planThemeSelection } from "./themePackStorage";
import type { ThemeMutationPlan } from "./themePackStorage";
import { readThemePackStore, validateThemePack } from "./themePack";
import type { ThemePack } from "./themePack";
import type { ThemePreview } from "./theme";
import { THEMES, type ThemeDef } from "./theme";
import Overlay from "./Overlay";

const THEME_SCOPE = ["theme", "uiThemePacks", "accent"] as const;
type ThemeCandidate = { pack: ThemePack; source: "file" | "installed" }
  | { theme: ThemeDef; source: "builtin" };

export default function ThemePackManager(props: {
  onPreview: (preview: ThemePreview | undefined) => void;
  onDownload: (fileName: string, text: string) => void;
}) {
  const t = useTranslate();
  useSettingsRevision();
  const snapshot = getAcknowledgedSettings(THEME_SCOPE);
  const available = snapshot.ok;
  const authority = snapshot.ok ? canonicalJson(snapshot.settings) : undefined;
  const [candidate, setCandidate] = useState<ThemeCandidate>();
  const [message, setMessage] = useState<MessageKey>();
  const [cacheWarning, setCacheWarning] = useState(false);
  const [admission, setAdmission] = useState<ThemePackFileDiagnostic>();
  const [busy, setBusy] = useState(false);
  const [reading, setReading] = useState(false);
  const [replacement, setReplacement] = useState<ThemeMutationPlan>();
  const [removal, setRemoval] = useState<ThemeMutationPlan>();
  const cancelReplacementRef = useRef<HTMLButtonElement | null>(null);
  const importRef = useRef<HTMLInputElement | null>(null);
  const previewOwner = useRef(props.onPreview);
  const ownerActive = useRef(true);
  const intake = useRef(0);
  const dispatched = useRef(false);
  const previewBase = useRef<string | undefined>(undefined);
  previewOwner.current = props.onPreview;
  useEffect(() => {
    ownerActive.current = true;
    return () => {
      ownerActive.current = false;
      intake.current += 1;
      previewOwner.current(undefined);
    };
  }, []);

  useEffect(() => {
    if (previewBase.current === undefined || previewBase.current === authority) return;
    intake.current += 1;
    previewBase.current = undefined;
    previewOwner.current(undefined);
    setCandidate(undefined);
    setReplacement(undefined);
    setMessage("themePack.manager.authorityChanged");
    setRemoval(undefined);
    setReading(false);
  }, [authority, t]);

  function cancelPreview() {
    intake.current += 1;
    previewBase.current = undefined;
    props.onPreview(undefined);
    setCandidate(undefined);
    setReplacement(undefined);
    setMessage(undefined);
    setRemoval(undefined);
    setAdmission(undefined);
    setReading(false);
    setCacheWarning(false);
  }

  async function applyCandidate() {
    if (!candidate || busy || dispatched.current) return;
    const current = getAcknowledgedSettings(THEME_SCOPE);
    if (!current.ok || canonicalJson(current.settings) !== previewBase.current) {
      cancelPreview();
      setMessage("themePack.manager.authorityChanged");
      return;
    }
    const prepared = candidate.source === "file"
      ? planThemeInstallation(candidate.pack, true)
      : planThemeSelection(candidate.source === "builtin" ? candidate.theme.id : candidate.pack.id);
    if (!prepared.ok) {
      cancelPreview();
      setMessage("themePack.manager.applyFailed");
      return;
    }
    if (prepared.plan.requiresReplacementConsent) {
      setReplacement(prepared.plan);
      setMessage("themePack.manager.replacementRequired");
      return;
    }
    await executePreparedPlan(prepared.plan);
  }

  async function confirmPlan(plan: ThemeMutationPlan, replacementConsent = false, success: "saved" | "removed" = "saved") {
    if (busy || dispatched.current) return;
    const current = getAcknowledgedSettings(THEME_SCOPE);
    if (!current.ok || canonicalJson(current.settings) !== previewBase.current) {
      cancelPreview();
      setMessage("themePack.manager.authorityChanged");
      return;
    }
    setReplacement(undefined);
    setRemoval(undefined);
    await executePreparedPlan(plan, replacementConsent, success);
  }

  async function executePreparedPlan(plan: ThemeMutationPlan, replacementConsent = false, success: "saved" | "removed" = "saved") {
    if (dispatched.current) return;
    dispatched.current = true;
    const epoch = intake.current;
    setBusy(true);
    // The conditional plan owns staleness once dispatched; the resulting ACK
    // is an intentional change, not a peer edit that should erase success.
    previewBase.current = undefined;
    try {
      const result = await commitThemeMutation(plan, replacementConsent);
      if (!ownerActive.current || intake.current !== epoch) return;
      cancelPreview();
      setMessage(success === "removed" ? "themePack.manager.removed" : "themePack.manager.saved");
      setCacheWarning(result.cacheError);
    } catch (error) {
      if (!ownerActive.current || intake.current !== epoch) return;
      cancelPreview();
      setMessage(error instanceof SettingsMutationError && error.kind === "conflict"
        ? "themePack.manager.conflict" : "themePack.manager.applyFailed");
    } finally {
      dispatched.current = false;
      if (ownerActive.current) setBusy(false);
    }
  }

  async function importFile(file: File) {
    if (dispatched.current) return;
    cancelPreview();
    const epoch = ++intake.current;
    setAdmission(undefined);
    setReplacement(undefined);
    setRemoval(undefined);
    const base = getAcknowledgedSettings(THEME_SCOPE);
    if (!base.ok) {
      setMessage("themePack.manager.unavailable");
      return;
    }
    previewBase.current = canonicalJson(base.settings);
    setReading(true);
    const result = await readThemePackFile(file);
    if (!ownerActive.current || intake.current !== epoch) return;
    setReading(false);
    if (!result.ok) {
      setAdmission(result.diagnostic);
      setMessage("themePack.manager.importRejected");
      return;
    }
    setCandidate({ pack: result.pack, source: "file" });
    setMessage(undefined);
    props.onPreview({ themeId: result.pack.id, pack: result.pack });
  }

  function previewInstalled(pack: ThemePack) {
    if (busy || dispatched.current) return;
    const base = getAcknowledgedSettings(THEME_SCOPE);
    if (!base.ok) return;
    cancelPreview();
    intake.current += 1;
    previewBase.current = canonicalJson(base.settings);
    setReplacement(undefined);
    setCandidate({ pack, source: "installed" });
    setRemoval(undefined);
    setMessage(undefined);
    props.onPreview({ themeId: pack.id, pack });
  }

  function downloadPack(pack: ThemePack) {
    const result = exportThemePack(pack);
    if (!result.ok) { setMessage("themePack.manager.exportFailed"); return; }
    try { props.onDownload(result.fileName, result.text); }
    catch { setMessage("themePack.manager.exportFailed"); }
  }

  function previewBuiltin(theme: ThemeDef) {
    if (busy || dispatched.current) return;
    const base = getAcknowledgedSettings(THEME_SCOPE);
    if (!base.ok) return;
    cancelPreview();
    intake.current += 1;
    previewBase.current = canonicalJson(base.settings);
    setCandidate({ theme, source: "builtin" });
    setReplacement(undefined);
    setRemoval(undefined);
    setAdmission(undefined);
    setMessage(undefined);
    props.onPreview({ themeId: theme.id });
  }

  async function resetToSystem() {
    if (busy || dispatched.current) return;
    const prepared = planThemeSelection("system");
    cancelPreview();
    if (!prepared.ok) { setMessage("themePack.manager.applyFailed"); return; }
    await executePreparedPlan(prepared.plan);
  }

  function requestRemoval(id: string) {
    if (busy || dispatched.current) return;
    const base = getAcknowledgedSettings(THEME_SCOPE);
    const prepared = planThemeRemoval(id);
    cancelPreview();
    if (!base.ok || !prepared.ok) { setMessage("themePack.manager.applyFailed"); return; }
    previewBase.current = canonicalJson(base.settings);
    setRemoval(prepared.plan);
  }

  function downloadRecovery() {
    const result = exportThemeRecovery(getSetting("uiThemePacks"), getSetting("theme"));
    if (!result.ok) { setMessage("themePack.manager.exportFailed"); return; }
    try { props.onDownload(result.fileName, result.text); }
    catch { setMessage("themePack.manager.exportFailed"); }
  }

  // Diagnostics/recovery may inspect cached data, but mutation still requires
  // the compatible acknowledged snapshot above. Never hide unadmitted entries.
  const selected = getSetting("theme");
  const store = readThemePackStore(getSetting("uiThemePacks"));
  const diagnostics = [...store.diagnostics];
  if (typeof selected === "string" && !THEMES.some((theme) => theme.id === selected)
    && !store.packs.some((pack) => pack.id === selected)) {
    diagnostics.push({ id: selected, diagnostic: { kind: "missingSelection", path: "$.theme" } });
  }
  const previousRaw = replacement?.expectedSettings.uiThemePacks;
  const previous = replacement && previousRaw && typeof previousRaw === "object"
    ? validateThemePack((previousRaw as Record<string, unknown>)[replacement.id]) : undefined;
  const candidateName = candidate?.source === "builtin" ? candidate.theme.name : candidate?.pack.name;
  const candidateVersion = candidate?.source === "builtin" ? t("themePack.manager.included") : candidate?.pack.version;

  return <section className="theme-manager" aria-labelledby="theme-pack-manager-heading" aria-busy={busy || reading}>
    <h4 id="theme-pack-manager-heading">{t("themePack.manager.heading")}</h4>
    <button type="button" disabled={!available || busy} onClick={() => { void resetToSystem(); }}>{t("themePack.manager.systemReset")}</button>
    <label className="settings-field" htmlFor="theme-pack-import">
      <span className="settings-field-label">{t("themePack.manager.import")}</span>
      <input ref={importRef} id="theme-pack-import" type="file" accept=".knx-theme.json,application/json"
        disabled={!available || busy} onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = "";
          if (file) void importFile(file);
        }} />
    </label>
    {!available && <p className="settings-field-hint">{t("themePack.manager.unavailable")}</p>}
    {diagnostics.length > 0 && <div role="status" aria-label={t("themePack.manager.diagnostics")}>
      <p>{t("themePack.manager.storedUnsupported")}</p>
      <ul>{diagnostics.map(({ id, diagnostic }, index) => <li key={`${id ?? "scope"}:${index}`}
        data-theme-diagnostic={diagnostic.kind}>
        <strong>{id ?? t("themePack.manager.unknownEntry")}</strong>
        <ThemePackDiagnostic diagnostic={diagnostic} />
      </li>)}</ul>
    </div>}
    <button type="button" onClick={downloadRecovery}>{t("themePack.manager.recovery")}</button>
    <p className="settings-field-hint">{t("themePack.manager.recoveryHint")}</p>
    <ul className="theme-manager-list">
      {THEMES.map((theme) => <li className="theme-manager-entry" key={theme.id} data-theme-id={theme.id} data-theme-origin="builtin">
        <strong>{theme.name}</strong> <span>{t("themePack.manager.builtin")}</span>
        <p><code>{theme.id}</code> {t("themePack.manager.included")}</p>
        {snapshot.ok && snapshot.settings.theme === theme.id && <p>{t("themePack.manager.selected")}</p>}
        <div className="theme-manager-actions">
          <button type="button" disabled={!available || busy} aria-label={t("themePack.manager.previewName", { name: theme.name })}
            onClick={() => previewBuiltin(theme)}>{t("themePack.manager.preview")}</button>
        </div>
      </li>)}
      {store.packs.map((pack) => <li className="theme-manager-entry" key={pack.id} data-theme-id={pack.id} data-theme-origin="imported">
        <strong>{pack.name}</strong> <span>{t("themePack.manager.imported")}</span>
        <p><code>{pack.id}</code> {t("themePack.manager.version", { version: pack.version })}</p>
        {snapshot.ok && snapshot.settings.theme === pack.id && <p>{t("themePack.manager.selected")}</p>}
        <div className="theme-manager-actions">
          <button type="button" disabled={!available || busy} aria-label={t("themePack.manager.previewName", { name: pack.name })}
            onClick={() => previewInstalled(pack)}>{t("themePack.manager.preview")}</button>
          <button type="button" disabled={busy} aria-label={t("themePack.manager.exportName", { name: pack.name })}
            onClick={() => downloadPack(pack)}>{t("themePack.manager.export")}</button>
          <button type="button" disabled={!available || busy} aria-label={t("themePack.manager.removeName", { name: pack.name })}
            onClick={() => requestRemoval(pack.id)}>{t("themePack.manager.remove")}</button>
        </div>
      </li>)}
    </ul>
    <div className="theme-manager-state" role="status">
      {busy && <p>{t("themePack.manager.saving")}</p>}
      {reading && <p>{t("themePack.manager.reading")}</p>}
      {candidate && <>
        <p>{t("themePack.manager.previewing", { name: candidateName!, version: candidateVersion! })}</p>
        <button type="button" disabled={!available || busy} onClick={() => { void applyCandidate(); }}>{t("themePack.manager.apply")}</button>
      </>}
      {(candidate || reading) && <button type="button" disabled={busy} onClick={cancelPreview}>{t("themePack.manager.cancel")}</button>}
      {message && <p className={message === "themePack.manager.saved" || message === "themePack.manager.removed" ? undefined : "field-error"}>{t(message)}</p>}
      {cacheWarning && <p className="settings-diagnostic">{t("themePack.manager.cacheWarning")}</p>}
      {admission && <ThemePackDiagnostic diagnostic={admission} />}
    </div>
    {replacement && candidate && <Overlay label={t("themePack.manager.replaceTitle")}
      className="settings-panel" initialFocusRef={cancelReplacementRef} restoreFocusRef={importRef} onClose={cancelPreview}>
      <h3>{t("themePack.manager.replaceTitle")}</h3>
      <p><code>{replacement.id}</code></p>
      <p>{previous?.ok ? t("themePack.manager.previous", { name: previous.pack.name, version: previous.pack.version })
        : t("themePack.manager.previousUnsupported")}</p>
      <p>{t("themePack.manager.incoming", { name: candidateName!, version: candidateVersion! })}</p>
      <p>{t("themePack.manager.replaceWarning")}</p>
      <div className="settings-actions">
        <button type="button" ref={cancelReplacementRef} onClick={cancelPreview}>{t("themePack.manager.replaceCancel")}</button>
        <button type="button" disabled={busy || !available} onClick={() => { void confirmPlan(replacement, true); }}>{t("themePack.manager.replaceConfirm")}</button>
      </div>
    </Overlay>}
    {removal && <Overlay label={t("themePack.manager.removeTitle")}
      className="settings-panel" initialFocusRef={cancelReplacementRef} restoreFocusRef={importRef} onClose={cancelPreview}>
      <h3>{t("themePack.manager.removeTitle")}</h3>
      <p><code>{removal.id}</code></p>
      <p>{t("themePack.manager.removeWarning")}</p>
      <div className="settings-actions">
        <button type="button" ref={cancelReplacementRef} onClick={cancelPreview}>{t("themePack.manager.removeCancel")}</button>
        <button type="button" disabled={busy || !available} onClick={() => { void confirmPlan(removal, false, "removed"); }}>{t("themePack.manager.removeConfirm")}</button>
      </div>
    </Overlay>}
  </section>;
}
