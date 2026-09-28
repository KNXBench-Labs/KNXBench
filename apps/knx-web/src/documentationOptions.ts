/** Section and language choices for the project documentation preview and export. */

// Section and language names exactly as `documentation_options`
// (apps/knx-server/src/routes.rs) accepts them, listed in the server's
// canonical document order. Header, contents and the limits/warnings
// section are always rendered and cannot be selected.
export const DOCUMENTATION_SECTIONS = [
  "summary",
  "topology",
  "buildings",
  "groupAddresses",
  "devices",
] as const;
export type DocumentationSection = (typeof DOCUMENTATION_SECTIONS)[number];
export type DocumentationLanguage = "en" | "de";

export interface DocumentationOptions {
  sections: DocumentationSection[];
  language: DocumentationLanguage;
}

// Filtering the canonical list keeps a request in document order, whatever
// order the sections were ticked in.
export function documentationOptionsFor(
  selected: ReadonlySet<DocumentationSection>,
  uiLanguage: string,
): DocumentationOptions {
  return {
    sections: DOCUMENTATION_SECTIONS.filter((section) => selected.has(section)),
    language: documentationLanguageFor(uiLanguage),
  };
}

// The report renderer knows English and German. A UI running any other
// language pack gets the English document rather than a refused request.
export function documentationLanguageFor(uiLanguage: string): DocumentationLanguage {
  return uiLanguage === "de" ? "de" : "en";
}
