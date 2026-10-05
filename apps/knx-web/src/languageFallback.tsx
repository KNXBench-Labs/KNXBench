/** KL-37 / AR10: one rule and one badge for product text shown in its package's own language. */
import { useTranslate } from "./i18n";

/**
 * True when `text` is shown but no stored language answered it, so it is the
 * package's (or master data's) own text. The server marks an answering
 * language as a string and the fallback as `null` or an absent field.
 * Callers only ask this with a product language selected: without one the
 * package's own text is exactly what was requested.
 */
export function fellBack(text: string | null | undefined, answered: string | null | undefined): boolean {
  return text !== null && text !== undefined && (answered ?? null) === null;
}

/** The badge beside such a text; `source` is the declared language, never guessed. */
export function LanguageFallbackBadge(props: {
  selected: string;
  source: string | null | undefined;
  part?: "label" | "options";
}) {
  const t = useTranslate();
  const options = props.part === "options";
  const source = props.source ?? null;
  return (
    <span className="provenance-badge language-fallback-badge" title={t("untranslated.title", { language: props.selected })}>
      {source === null
        ? t(options ? "untranslated.optionsUnknown" : "untranslated.labelUnknown")
        : t(options ? "untranslated.options" : "untranslated.label", { source })}
    </span>
  );
}
