/** Theme files: install-and-select import, export/remove, diagnostics, storage location. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useEffect, useRef, useState } from "react";
import { useTranslate, type MessageKey } from "./i18n";
import { getAcknowledgedSettings, getSetting, SettingsMutationError, useSettingsRevision } from "./settingsStore";
import { exportThemePack, exportThemeRecovery, readThemePackFile } from "./themePackFiles";
import type { ThemePackFileDiagnostic } from "./themePackFiles";
import ThemePackDiagnostic from "./ThemePackDiagnostic";
import { canonicalJson } from "./canonicalJson";
import { commitThemeMutation, planThemeInstallation, planThemeRemoval } from "./themePackStorage";
import type { ThemeMutationPlan } from "./themePackStorage";
import { readThemePackStore, validateThemePack } from "./themePack";
import type { ThemePack } from "./themePack";
import { findThemePack, isBundledThemeId, THEMES } from "./theme";
import Overlay from "./Overlay";

const THEME_SCOPE = ["theme", "uiThemePacks", "accent"] as const;

/**
 * The theme *choice* is the Appearance dropdown (SettingsPanel). This block
 * only manages files, per the user's decision of 2026-10-05 (ADR-0079): no
 * per-theme cards, no preview — an imported file is installed and selected
 * in one conditional write, and the dropdown takes you back.
 */
export default function ThemePackManager(props: { onDownload: (fileName: string, text: string) => void }) {
  const t = useTranslate();
  useSettingsRevision();
  const snapshot = getAcknowledgedSettings(THEME_SCOPE);
  const available = snapshot.ok;
  const authority = snapshot.ok ? canonicalJson(snapshot.settings) : undefined;
  const [message, setMessage] = useState<MessageKey>();
  const [cacheWarning, setCacheWarning] = useState(false);
  const [admission, setAdmission] = useState<ThemePackFileDiagnostic>();
  const [busy, setBusy] = useState(false);
  const [reading, setReading] = useState(false);
  const [replacement, setReplacement] = useState<{ plan: ThemeMutationPlan; incoming: ThemePack }>();
  const [removal, setRemoval] = useState<ThemeMutationPlan>();
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  const importRef = useRef<HTMLInputElement | null>(null);
  const ownerActive = useRef(true);
  const intake = useRef(0);
  const dispatched = useRef(false);
  // The acknowledged settings an open question (replace/remove) was asked against.
  const questionBase = useRef<string | undefined>(undefined);
  useEffect(() => {
    ownerActive.current = true;
    return () => { ownerActive.current = false; intake.current += 1; };
  }, []);

  // A peer change revokes an open replace/remove question: its answer
  // would be about settings that no longer exist.
  useEffect(() => {
    if (questionBase.current === undefined || questionBase.current === authority) return;
    questionBase.current = undefined;
    setReplacement(undefined);
    setRemoval(undefined);
    setMessage("themePack.manager.authorityChanged");
  }, [authority]);

  function closeQuestion() {
    questionBase.current = undefined;
    setReplacement(undefined);
    setRemoval(undefined);
  }

  async function execute(plan: ThemeMutationPlan, replacementConsent = false, success: "saved" | "removed" = "saved") {
    if (dispatched.current) return;
    dispatched.current = true;
    const epoch = intake.current;
    setBusy(true);
    questionBase.current = undefined;
    try {
      const result = await commitThemeMutation(plan, replacementConsent);
      if (!ownerActive.current || intake.current !== epoch) return;
      setMessage(success === "removed" ? "themePack.manager.removed" : "themePack.manager.saved");
      setCacheWarning(result.cacheError);
    } catch (error) {
      if (!ownerActive.current || intake.current !== epoch) return;
      setMessage(error instanceof SettingsMutationError && error.kind === "conflict"
        ? "themePack.manager.conflict" : "themePack.manager.applyFailed");
    } finally {
      dispatched.current = false;
      if (ownerActive.current) setBusy(false);
    }
  }

  async function confirm(plan: ThemeMutationPlan, replacementConsent: boolean, success: "saved" | "removed") {
    if (busy || dispatched.current) return;
    const current = getAcknowledgedSettings(THEME_SCOPE);
    if (!current.ok || canonicalJson(current.settings) !== questionBase.current) {
      closeQuestion();
      setMessage("themePack.manager.authorityChanged");
      return;
    }
    setReplacement(undefined);
    setRemoval(undefined);
    await execute(plan, replacementConsent, success);
  }

  async function importFile(file: File) {
    if (dispatched.current) return;
    closeQuestion();
    const epoch = ++intake.current;
    setAdmission(undefined);
    setMessage(undefined);
    setCacheWarning(false);
    if (!getAcknowledgedSettings(THEME_SCOPE).ok) { setMessage("themePack.manager.unavailable"); return; }
    setReading(true);
    const result = await readThemePackFile(file);
    if (!ownerActive.current || intake.current !== epoch) return;
    setReading(false);
    if (!result.ok) {
      setAdmission(result.diagnostic);
      setMessage("themePack.manager.importRejected");
      return;
    }
    const base = getAcknowledgedSettings(THEME_SCOPE);
    const prepared = planThemeInstallation(result.pack, true);
    if (!base.ok || !prepared.ok) { setMessage("themePack.manager.applyFailed"); return; }
    if (prepared.plan.requiresReplacementConsent) {
      questionBase.current = canonicalJson(base.settings);
      setReplacement({ plan: prepared.plan, incoming: result.pack });
      setMessage("themePack.manager.replacementRequired");
      return;
    }
    await execute(prepared.plan);
  }

  function requestRemoval(id: string) {
    if (busy || dispatched.current) return;
    const base = getAcknowledgedSettings(THEME_SCOPE);
    const prepared = planThemeRemoval(id);
    closeQuestion();
    if (!base.ok || !prepared.ok) { setMessage("themePack.manager.applyFailed"); return; }
    questionBase.current = canonicalJson(base.settings);
    setMessage(undefined);
    setRemoval(prepared.plan);
  }

  function download(result: { ok: true; fileName: string; text: string } | { ok: false }) {
    if (!result.ok) { setMessage("themePack.manager.exportFailed"); return; }
    try { props.onDownload(result.fileName, result.text); }
    catch { setMessage("themePack.manager.exportFailed"); }
  }

  // Diagnostics/recovery may inspect cached data, but mutation still requires
  // the compatible acknowledged snapshot above. Never hide unadmitted entries.
  const selected = getSetting("theme");
  const store = readThemePackStore(getSetting("uiThemePacks"));
  const diagnostics = [...store.diagnostics];
  if (typeof selected === "string" && !THEMES.some((theme) => theme.id === selected) && !findThemePack(store, selected)) {
    diagnostics.push({ id: selected, diagnostic: { kind: "missingSelection", path: "$.theme" } });
  }
  const selectedPack = typeof selected === "string" ? findThemePack(store, selected) : undefined;
  const selectedInstalled = selectedPack !== undefined && !isBundledThemeId(store, selectedPack.id);
  const previousRaw = replacement?.plan.expectedSettings.uiThemePacks;
  const previous = replacement && previousRaw && typeof previousRaw === "object"
    ? validateThemePack((previousRaw as Record<string, unknown>)[replacement.plan.id]) : undefined;

  return <section className="theme-manager" aria-labelledby="theme-pack-manager-heading" aria-busy={busy || reading}>
    <h4 id="theme-pack-manager-heading">{t("themePack.manager.heading")}</h4>
    <label className="settings-field" htmlFor="theme-pack-import">
      <span className="settings-field-label">{t("themePack.manager.import")}</span>
      <input ref={importRef} id="theme-pack-import" type="file" accept=".knx-theme.json,application/json"
        disabled={!available || busy} onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = "";
          if (file) void importFile(file);
        }} />
    </label>
    {selectedPack && <div className="theme-manager-actions">
      <button type="button" disabled={busy} aria-label={t("themePack.manager.exportName", { name: selectedPack.name })}
        onClick={() => download(exportThemePack(selectedPack))}>{t("themePack.manager.export")}</button>
      {selectedInstalled && <button type="button" disabled={!available || busy}
        aria-label={t("themePack.manager.removeName", { name: selectedPack.name })}
        onClick={() => requestRemoval(selectedPack.id)}>{t("themePack.manager.remove")}</button>}
    </div>}
    <p className="settings-field-hint" data-theme-storage>{t("themePack.manager.storage")}</p>
    {!available && <p className="settings-field-hint">{t("themePack.manager.unavailable")}</p>}
    {diagnostics.length > 0 && <div role="status" aria-label={t("themePack.manager.diagnostics")}>
      <p>{t("themePack.manager.storedUnsupported")}</p>
      <ul>{diagnostics.map(({ id, diagnostic }, index) => <li key={`${id ?? "scope"}:${index}`}
        data-theme-diagnostic={diagnostic.kind}>
        <strong>{id ?? t("themePack.manager.unknownEntry")}</strong>
        <ThemePackDiagnostic diagnostic={diagnostic} />
      </li>)}</ul>
      <button type="button" onClick={() => download(exportThemeRecovery(getSetting("uiThemePacks"), getSetting("theme")))}>
        {t("themePack.manager.recovery")}</button>
      <p className="settings-field-hint">{t("themePack.manager.recoveryHint")}</p>
    </div>}
    <div className="theme-manager-state" role="status">
      {busy && <p>{t("themePack.manager.saving")}</p>}
      {reading && <p>{t("themePack.manager.reading")}</p>}
      {message && <p className={message === "themePack.manager.saved" || message === "themePack.manager.removed" ? undefined : "field-error"}>{t(message)}</p>}
      {cacheWarning && <p className="settings-diagnostic">{t("themePack.manager.cacheWarning")}</p>}
      {admission && <ThemePackDiagnostic diagnostic={admission} />}
    </div>
    {replacement && <Overlay label={t("themePack.manager.replaceTitle")}
      className="settings-panel" initialFocusRef={cancelRef} restoreFocusRef={importRef} onClose={closeQuestion}>
      <h3>{t("themePack.manager.replaceTitle")}</h3>
      <p><code>{replacement.plan.id}</code></p>
      <p>{previous?.ok ? t("themePack.manager.previous", { name: previous.pack.name, version: previous.pack.version })
        : t("themePack.manager.previousUnsupported")}</p>
      <p>{t("themePack.manager.incoming", { name: replacement.incoming.name, version: replacement.incoming.version })}</p>
      <p>{t("themePack.manager.replaceWarning")}</p>
      <div className="settings-actions">
        <button type="button" ref={cancelRef} onClick={closeQuestion}>{t("themePack.manager.replaceCancel")}</button>
        <button type="button" disabled={busy || !available} onClick={() => { void confirm(replacement.plan, true, "saved"); }}>{t("themePack.manager.replaceConfirm")}</button>
      </div>
    </Overlay>}
    {removal && <Overlay label={t("themePack.manager.removeTitle")}
      className="settings-panel" initialFocusRef={cancelRef} restoreFocusRef={importRef} onClose={closeQuestion}>
      <h3>{t("themePack.manager.removeTitle")}</h3>
      <p><code>{removal.id}</code></p>
      <p>{t("themePack.manager.removeWarning")}</p>
      <div className="settings-actions">
        <button type="button" ref={cancelRef} onClick={closeQuestion}>{t("themePack.manager.removeCancel")}</button>
        <button type="button" disabled={busy || !available} onClick={() => { void confirm(removal, false, "removed"); }}>{t("themePack.manager.removeConfirm")}</button>
      </div>
    </Overlay>}
  </section>;
}
