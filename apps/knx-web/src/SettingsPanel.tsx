/** Settings overlay for theme, motion, UI/product language, and language-pack management. */
import { ACCENTS, DENSITIES, type useAppearance } from "./appearance";
import { useState } from "react";
import type { ThemeDef } from "./theme";
import type { MotionLevelDef, MotionStyleDef } from "./motion";
import type { ProductLanguage } from "./api";
import { AVAILABLE_UI_LANGUAGES, useUiLanguage } from "./uiLanguage";
import { useTranslate } from "./i18n";
import type { Translate } from "./i18n";
import Overlay from "./Overlay";
import {
  exportEnglishTemplate,
  exportLanguagePack,
  grandfatheredHint,
  importLanguagePack,
  removeLanguagePack,
  useLanguagePacks,
} from "./languagePack";
import type { LanguagePackImportReport, LanguagePackRejectionReason } from "./languagePack";

/** Whether `tag` is one of the compiled-in catalogues — the shadowing trap
 * `languagePack.ts`'s `exportEnglishTemplate` doc comment warns about
 * (its own placeholder tag is `"en"`), generalised to `"de"` too: either
 * built-in is checked before any installed pack in `i18n.ts`'s
 * `resolveCatalog`, so a pack tagged one of these is dead on arrival. */
function isShadowedByBuiltIn(tag: string): boolean {
  return (AVAILABLE_UI_LANGUAGES as readonly string[]).includes(tag);
}

/** Ties the accent select to the sentence explaining why it is disabled.
 * The panel is a singleton overlay, so a constant id is safe. */
const ACCENT_HINT_ID = "settings-accent-unavailable-hint";

/** `LanguagePackRejectionReason` plus the one rejection that never reaches
 * `languagePack.ts` at all — the uploaded file failing `JSON.parse` before
 * `importLanguagePack` is ever called. Handled by the same
 * `describeRejectionReason` switch below so both paths compose the same
 * translated "Import rejected: …" sentence, closing KNOWN_LIMITATIONS.md
 * §67. */
type ImportRejection = LanguagePackRejectionReason | { kind: "invalidJson" };

type ImportOutcome =
  | { ok: true; report: LanguagePackImportReport }
  | { ok: false; error: string; reason: ImportRejection; hint?: { oldTag: string; modernTag: string } };

/**
 * Composes the reason clause of `languagePack.importReport.rejected`
 * ("Import rejected: {reason}") in the active UI language, mirroring
 * `CatalogBrowser.tsx`'s `describeCreationDiagnostic` for
 * `CreationDiagnostic` — one catalogue key per `reason.kind` instead of
 * rendering `languagePack.ts`'s English `error` string verbatim, which is
 * exactly what KNOWN_LIMITATIONS.md §67 used to describe: a translated
 * sentence with an untranslated English clause sitting inside it. The
 * `default` falls back to `fallbackError` (the untranslated `.error` this
 * build's `reason` was derived from) purely as a belt-and-braces measure —
 * every `kind` this build can produce is handled above it, so it should
 * never actually run, the same reasoning as that function's own fallback.
 */
function describeRejectionReason(t: Translate, reason: ImportRejection, fallbackError: string): string {
  switch (reason.kind) {
    case "invalidJson":
      return t("languagePack.importReport.invalidJson");
    case "notObject":
      return t("languagePack.rejection.notObject");
    case "formatVersionMissing":
      return t("languagePack.rejection.formatVersionMissing");
    case "tagMissing":
      return t("languagePack.rejection.tagMissing");
    case "tagMalformed":
      return t("languagePack.rejection.tagMalformed", { tag: reason.tag });
    case "nameMissing":
      return t("languagePack.rejection.nameMissing");
    case "messagesMissing":
      return t("languagePack.rejection.messagesMissing");
    case "messageValueNotString":
      return t("languagePack.rejection.messageValueNotString", {
        key: reason.key,
        valueType: reason.valueType,
      });
    case "englishNameNotString":
      return t("languagePack.rejection.englishNameNotString");
    case "basedOnNotString":
      return t("languagePack.rejection.basedOnNotString");
    case "packVersionNotString":
      return t("languagePack.rejection.packVersionNotString");
    case "pluralCategoriesInvalid":
      return t("languagePack.rejection.pluralCategoriesInvalid");
    case "storageFailure":
      return t("languagePack.rejection.storageFailure", { detail: reason.detail });
    default:
      return fallbackError;
  }
}

/** Triggers a browser "save this file" flow for `data`, JSON-encoded. The
 * only DOM-touching part of export — kept to one tiny function so the
 * data-shaping half (what `handleExportTemplate`/`handleExportPack` build)
 * stays trivial to unit-test without needing a real download to happen. */
function downloadJson(filename: string, data: unknown): void {
  const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

/** Renders one outcome of an import — success (with its full report) or
 * rejection (with the reason and, where recognised, a grandfathered-tag
 * hint) — as a `role="status"` block so assistive technology announces it
 * the moment it appears, the same reason `Toast.tsx` uses that role. */
function ImportReport(props: { t: Translate; outcome: ImportOutcome }) {
  const { t, outcome } = props;

  if (!outcome.ok) {
    return (
      <div className="language-pack-report" role="status">
        <p className="field-error">
          {t("languagePack.importReport.rejected", {
            reason: describeRejectionReason(t, outcome.reason, outcome.error),
          })}
        </p>
        {outcome.hint && (
          <p>
            {t("languagePack.importReport.grandfatheredHint", {
              oldTag: outcome.hint.oldTag,
              modernTag: outcome.hint.modernTag,
            })}
          </p>
        )}
      </div>
    );
  }

  const { report } = outcome;
  return (
    <div className="language-pack-report" role="status">
      <p>{t("languagePack.importReport.heading", { name: report.name })}</p>
      <p>
        {t("languagePack.importReport.appliedKeys", { count: report.appliedKeyCount })}{" "}
        {t("languagePack.importReport.missingKeys", { count: report.missingKeyCount })}
      </p>
      {report.missingKeyCount > 0 && (
        <p>{t("languagePack.importReport.missingKeysSample", { keys: report.missingKeysSample.join(", ") })}</p>
      )}
      {report.unknownKeys.length > 0 && (
        <p>
          {t("languagePack.importReport.unknownKeys", { count: report.unknownKeys.length })}{" "}
          {report.unknownKeys.join(", ")}
        </p>
      )}
      <p>
        {report.pluralRulesSupported
          ? t("languagePack.importReport.pluralSupported")
          : t("languagePack.importReport.pluralUnsupported")}
      </p>
      {isShadowedByBuiltIn(report.tag) && (
        <p className="field-error">
          {t("languagePack.importReport.shadowedByBuiltIn", { tag: report.tag })}
        </p>
      )}
    </div>
  );
}

/**
 * The application's one settings surface (design D32): Theme, Motion
 * style, Motion level, Product data language and UI language, each a
 * registry-backed `<select>` that applies immediately — no Save button,
 * no reload.
 *
 * Built on the shared `Overlay` shell (T31), the same one `Search.tsx`
 * and `CommandPalette.tsx` use — dismissal by backdrop click and by
 * `Escape`, initial focus, a focus trap and focus restoration all come
 * from there now, closing the gap KNOWN_LIMITATIONS.md §20 used to
 * describe.
 *
 * Purely presentational (T26) for the first four fields: `App.tsx` owns
 * their state and fetches `productLanguages` itself; this component only
 * renders props and forwards `onChange`. UI language (T25 task 2) is the
 * deliberate exception — it reads `uiLanguage.ts`'s shared store directly
 * via `useUiLanguage()` instead of taking it as a prop, precisely so that
 * changing it needs no `App.tsx` wiring and reaches every other mounted
 * consumer of that store (any future `useTranslate()` caller) without a
 * remount.
 *
 * T25 task 7 grows the UI-language field into a small language-pack
 * manager: `languagePack.ts` (task 6) built the format, import and export
 * as plain functions with no UI of their own — this component is the
 * only place any of it becomes visible. Installed packs are read through
 * `useLanguagePacks()`, the same shared-store shape `useUiLanguage()`
 * uses: `i18n.ts`'s `useTranslate()` subscribes to the very same store
 * (fix round 1), so this panel no longer needs to hand-refresh a local
 * copy after every import/remove — the shared store already notifies
 * every mounted reader, itself included.
 */
export default function SettingsPanel(props: {
  appearance?: ReturnType<typeof useAppearance>;
  themes: readonly ThemeDef[];
  activeThemeId: string;
  onSelectTheme: (id: string) => void;
  motionStyles: readonly MotionStyleDef[];
  activeMotionStyle: string;
  onSelectMotionStyle: (id: string) => void;
  motionLevels: readonly MotionLevelDef[];
  activeMotionLevel: string;
  onSelectMotionLevel: (id: string) => void;
  productLanguages: readonly ProductLanguage[];
  activeProductLanguage: string | null;
  onSelectProductLanguage: (language: string | null) => void;
  onClose: () => void;
}) {
  const {
    themes,
    activeThemeId,
    onSelectTheme,
    motionStyles,
    activeMotionStyle,
    onSelectMotionStyle,
    motionLevels,
    activeMotionLevel,
    onSelectMotionLevel,
    productLanguages,
    activeProductLanguage,
    onSelectProductLanguage,
    onClose,
  } = props;

  const [uiLanguage, setUiLanguage] = useUiLanguage();
  const t = useTranslate();

  const packs = useLanguagePacks();
  const [importOutcome, setImportOutcome] = useState<ImportOutcome | null>(null);

  // Cupertino, Neon Grid and Bitcoin DeFi treat the accent as identity and
  // declare no `[data-accent="…"]` variations (ADR-0022) — the control
  // below would silently do nothing for them, so it is disabled instead.
  const activeTheme = themes.find((theme) => theme.id === activeThemeId);
  const accentUnavailable = activeTheme !== undefined && !activeTheme.hasAccentVariations;

  async function handleImportFile(file: File) {
    let raw: unknown;
    try {
      raw = JSON.parse(await file.text());
    } catch {
      setImportOutcome({
        ok: false,
        error: t("languagePack.importReport.invalidJson"),
        reason: { kind: "invalidJson" },
      });
      return;
    }

    const result = importLanguagePack(raw);
    if (result.ok) {
      setImportOutcome(result);
      return;
    }

    const rawTag = typeof raw === "object" && raw !== null ? (raw as Record<string, unknown>).tag : undefined;
    const modernTag = grandfatheredHint(rawTag);
    setImportOutcome({
      ok: false,
      error: result.error,
      reason: result.reason,
      hint: modernTag !== undefined ? { oldTag: rawTag as string, modernTag } : undefined,
    });
  }

  function handleExportTemplate() {
    downloadJson("knx-language-pack-template.json", exportEnglishTemplate());
  }

  function handleExportPack(tag: string) {
    const pack = exportLanguagePack(tag);
    if (!pack) return;
    downloadJson(`knx-language-pack-${tag}.json`, pack);
  }

  function handleRemovePack(tag: string) {
    // No `setUiLanguage` here, deliberately: removing a pack is a
    // pack-management action, not a request to change the active
    // language preference. `languagePack.ts`'s `removeLanguagePack`
    // notifies its own subscribers, which is what makes an
    // already-mounted `useTranslate()` caller re-render and fall back to
    // English the moment `i18n.ts`'s `resolveCatalog` finds no pack left
    // for the active tag — the stored tag itself is untouched, so
    // re-importing the same pack later restores the language with no
    // trip through the select.
    removeLanguagePack(tag);
  }

  // A pack whose own tag collides with a built-in (the exact
  // `exportEnglishTemplate()`-shadowing trap `languagePack.ts` warns
  // about) is left out of the select — it would render a second
  // `value="en"`/`value="de"` option that can never win over the real
  // built-in in `i18n.ts`'s `resolveCatalog`, just a confusing duplicate.
  // It still appears in the management list below so it can be exported,
  // fixed and re-imported, or removed.
  const selectablePacks = packs.filter((pack) => !isShadowedByBuiltIn(pack.tag));

  return (
    <Overlay labelledBy="settings-panel-title" className="settings-panel" onClose={onClose}>
      <h2 className="settings-panel-title" id="settings-panel-title">
        {t("settings.title")}
      </h2>
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.theme")}</span>
        <select
          value={activeThemeId}
          onChange={(e) => onSelectTheme(e.target.value)}
          aria-label={t("settings.theme")}
        >
          {themes.map((theme) => (
            <option key={theme.id} value={theme.id}>
              {theme.name}
            </option>
          ))}
        </select>
      </label>
      {props.appearance && <>
        <label className="settings-field">
          <span className="settings-field-label">{t("appearance.accent")}</span>
          {/* `aria-describedby` rather than nothing: a disabled control
              announces "unavailable" and stops, so without this the reason
              — which is on screen, right below it — reaches a sighted user
              and nobody else. The `aria-label` stays because the hint lives
              inside the wrapping `<label>`, whose text content would
              otherwise become part of the accessible *name*. */}
          <select aria-label={t("appearance.accent")} value={props.appearance.accent}
            disabled={accentUnavailable}
            aria-describedby={accentUnavailable ? ACCENT_HINT_ID : undefined}
            onChange={(e) => props.appearance!.setAccent(e.target.value as typeof ACCENTS[number])}>
            {ACCENTS.map((accent) => <option key={accent} value={accent}>{t(`appearance.${accent}`)}</option>)}
          </select>
          {accentUnavailable && (
            <span className="settings-field-hint" id={ACCENT_HINT_ID}>
              {t("appearance.accentUnavailable")}
            </span>
          )}
        </label>
        <label className="settings-field">
          <span className="settings-field-label">{t("appearance.density")}</span>
          <select aria-label={t("appearance.density")} value={props.appearance.density}
            onChange={(e) => props.appearance!.setDensity(e.target.value as typeof DENSITIES[number])}>
            {DENSITIES.map((density) => <option key={density} value={density}>{t(`appearance.${density}`)}</option>)}
          </select>
        </label>
      </>}
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.motionStyle")}</span>
        <select
          value={activeMotionStyle}
          onChange={(e) => onSelectMotionStyle(e.target.value)}
          aria-label={t("settings.motionStyle")}
        >
          {motionStyles.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
      </label>
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.motionLevel")}</span>
        <select
          value={activeMotionLevel}
          onChange={(e) => onSelectMotionLevel(e.target.value)}
          aria-label={t("settings.motionLevel")}
        >
          {motionLevels.map((l) => (
            <option key={l.id} value={l.id}>
              {l.name}
            </option>
          ))}
        </select>
      </label>
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.productDataLanguage")}</span>
        {productLanguages.length === 0 ? (
          <select aria-label={t("settings.productDataLanguage")} disabled>
            <option>{t("settings.noProductDatabase")}</option>
          </select>
        ) : (
          <select
            value={activeProductLanguage ?? ""}
            onChange={(e) => onSelectProductLanguage(e.target.value === "" ? null : e.target.value)}
            aria-label={t("settings.productDataLanguage")}
          >
            <option value="">{t("settings.packageDefault")}</option>
            {productLanguages.map((l) => (
              <option key={l.language} value={l.language}>
                {t("settings.productLanguageOption", { language: l.language, count: l.rows })}
              </option>
            ))}
          </select>
        )}
      </label>
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.uiLanguage")}</span>
        <select
          value={uiLanguage}
          onChange={(e) => setUiLanguage(e.target.value)}
          aria-label={t("settings.uiLanguage")}
        >
          {AVAILABLE_UI_LANGUAGES.map((language) => (
            <option key={language} value={language}>
              {t(`language.${language}`)}
            </option>
          ))}
          {/* An installed pack names itself here in its own `name` —
              that's what the field is for, `englishName` is diagnostics
              only. React escapes it: no `dangerouslySetInnerHTML` in
              sight, so user-supplied text never becomes markup. */}
          {selectablePacks.map((pack) => (
            <option key={pack.tag} value={pack.tag}>
              {pack.name}
            </option>
          ))}
        </select>
      </label>

      <div className="language-pack-manager">
        <label className="catalog-install">
          {t("languagePack.importLabel")}
          <input
            type="file"
            accept="application/json,.json"
            onChange={(e) => {
              const file = e.target.files?.[0];
              e.currentTarget.value = "";
              if (file) void handleImportFile(file);
            }}
          />
        </label>

        <div className="settings-field">
          <button type="button" onClick={handleExportTemplate}>
            {t("languagePack.exportTemplateButton")}
          </button>
          <p className="language-pack-hint">{t("languagePack.exportTemplateHint")}</p>
        </div>

        {importOutcome && <ImportReport t={t} outcome={importOutcome} />}

        <h3 className="language-pack-list-title">{t("languagePack.installedTitle")}</h3>
        {packs.length === 0 ? (
          <p className="language-pack-hint">{t("languagePack.noPacksInstalled")}</p>
        ) : (
          <ul className="language-pack-list">
            {packs.map((pack) => (
              <li key={pack.tag} className="language-pack-list-item">
                <span className="language-pack-list-name">{pack.name}</span>
                <button
                  type="button"
                  onClick={() => handleExportPack(pack.tag)}
                  aria-label={t("languagePack.exportPackAriaLabel", { name: pack.name })}
                >
                  {t("languagePack.exportPackButton")}
                </button>
                <button
                  type="button"
                  onClick={() => handleRemovePack(pack.tag)}
                  aria-label={t("languagePack.removePackAriaLabel", { name: pack.name })}
                >
                  {t("languagePack.removePackButton")}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Overlay>
  );
}
