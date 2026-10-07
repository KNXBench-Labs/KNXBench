/** Language picker autonyms, independent of the selected interface translation. */
import { BUNDLED_LANGUAGE_PACKS } from "./bundledLanguagePacks";
import { isWellFormedBcp47Tag } from "./languagePack";

const FIXED_NAMES: Readonly<Record<string, string>> = {
  en: "English",
  de: "Deutsch",
  ...Object.fromEntries(BUNDLED_LANGUAGE_PACKS.map(({ tag, name }) => [tag, name])),
};

/** Unknown, malformed or runtime-unsupported tags remain visible verbatim.
 * Only the display label changes; the caller retains the original option value. */
export function languageSelfName(tag: string): string {
  if (!isWellFormedBcp47Tag(tag)) return tag;
  const primary = tag.split("-")[0]?.toLowerCase() ?? "";
  if (FIXED_NAMES[primary]) return FIXED_NAMES[primary];
  try {
    if (Intl.DisplayNames.supportedLocalesOf([tag]).length === 0) return tag;
    return new Intl.DisplayNames([tag], { type: "language", fallback: "none" }).of(primary) ?? tag;
  } catch {
    return tag;
  }
}

/** Preserve the exact regional/script tag alongside its readable language name. */
export function productLanguageLabel(tag: string): string {
  const name = languageSelfName(tag);
  return name !== tag && tag.includes("-") ? `${name} (${tag})` : name;
}
