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
  importLanguagePack,
  listLanguagePacks,
  removeLanguagePack,
} from "./languagePack";
import type { LanguagePack, LanguagePackImportReport } from "./languagePack";

/**
 * RFC 5646 §4.5's handful of "irregular" grandfathered tags — the ones
 * `languagePack.ts`'s `BCP47_PATTERN` has no production for at all (an
 * `i-`/`sgn-` prefix isn't a legal primary subtag under any reading this
 * project uses) — each with its modern IANA-registered replacement. Not
 * this surface's job to accept them (that ship, per the brief, sailed with
 * `languagePack.ts`), only to be a little kinder about *why* one was
 * rejected when we happen to recognise it. Deliberately small: these are
 * the tags actually listed in the RFC, not a guess at every historical
 * form anyone ever used.
 */
const GRANDFATHERED_TAG_HINTS: Record<string, string> = {
  "i-ami": "ami",
  "i-bnn": "bnn",
  "i-hak": "hak",
  "i-klingon": "tlh",
  "i-lux": "lb",
  "i-navajo": "nv",
  "i-pwn": "pwn",
  "i-tao": "tao",
  "i-tay": "tay",
  "i-tsu": "tsu",
  "sgn-be-fr": "sfb",
  "sgn-be-nl": "vgt",
  "sgn-ch-de": "sgg",
};

function grandfatheredHint(rawTag: unknown): { oldTag: string; modernTag: string } | undefined {
  if (typeof rawTag !== "string") return undefined;
  const modernTag = GRANDFATHERED_TAG_HINTS[rawTag.toLowerCase()];
  return modernTag ? { oldTag: rawTag, modernTag } : undefined;
}

/** Whether `tag` is one of the compiled-in catalogues — the shadowing trap
 * `languagePack.ts`'s `exportEnglishTemplate` doc comment warns about
 * (its own placeholder tag is `"en"`), generalised to `"de"` too: either
 * built-in is checked before any installed pack in `i18n.ts`'s
 * `resolveCatalog`, so a pack tagged one of these is dead on arrival. */
function isShadowedByBuiltIn(tag: string): boolean {
  return (AVAILABLE_UI_LANGUAGES as readonly string[]).includes(tag);
}

type ImportOutcome =
  | { ok: true; report: LanguagePackImportReport }
  | { ok: false; error: string; hint?: { oldTag: string; modernTag: string } };

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
        <p className="field-error">{t("languagePack.importReport.rejected", { reason: outcome.error })}</p>
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
 * manager: `languagePack.ts` (task 6) built the format, the store, import
 * and export as plain functions with no UI of their own — this component
 * is the only place any of it becomes visible. Installed packs are held
 * in local `useState`, refreshed after every import/remove, rather than a
 * shared external store: unlike the active language, nothing outside this
 * panel needs to react to "a pack was installed" on its own.
 */
export default function SettingsPanel(props: {
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

  const [packs, setPacks] = useState<LanguagePack[]>(() => listLanguagePacks());
  const [importOutcome, setImportOutcome] = useState<ImportOutcome | null>(null);

  async function handleImportFile(file: File) {
    let raw: unknown;
    try {
      raw = JSON.parse(await file.text());
    } catch {
      setImportOutcome({ ok: false, error: t("languagePack.importReport.invalidJson") });
      return;
    }

    const result = importLanguagePack(raw);
    if (result.ok) {
      setPacks(listLanguagePacks());
      setImportOutcome(result);
      return;
    }

    const rawTag = typeof raw === "object" && raw !== null ? (raw as Record<string, unknown>).tag : undefined;
    setImportOutcome({ ok: false, error: result.error, hint: grandfatheredHint(rawTag) });
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
    removeLanguagePack(tag);
    setPacks(listLanguagePacks());
    // `i18n.ts`'s `resolveCatalog` already falls back to English for any
    // tag naming a pack that doesn't exist — that resolution logic is not
    // duplicated here. But nothing re-renders an *already-mounted* sibling
    // just because this store changed underneath it (the packs store has
    // no subscribers of its own); explicitly handing the active-language
    // store back to English when the pack it names is the one just removed
    // is what makes that fallback visible immediately, everywhere, rather
    // than only on whatever next render happens to touch a translated
    // string. If the same pack is re-imported later, the user picks it
    // again — the alternative (leaving the stale tag in place so a
    // re-import silently reactivates it) trades a surprise reappearance
    // for a redundant click, and the click is the smaller surprise.
    if (tag === uiLanguage) setUiLanguage("en");
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
