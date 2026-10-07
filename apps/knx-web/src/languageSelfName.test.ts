/** Language labels stay readable without translating or rewriting their stored tags. */
import { afterEach, expect, it, vi } from "vitest";
import { languageSelfName, productLanguageLabel } from "./languageSelfName";

afterEach(() => vi.restoreAllMocks());

it.each([
  ["en", "English"], ["de", "Deutsch"], ["de-DE", "Deutsch"],
  ["bar", "Boarisch"], ["tlh", "Klingonisch/Klingon"],
  ["fr-FR", "français"], ["es-ES", "español"],
])("names %s in itself", (tag, name) => {
  expect(languageSelfName(tag)).toBe(name);
});

it.each(["x-imaginary", "xx", "en_US", "en-??", "", "not a tag"])("preserves unsupported or malformed tag %s", (tag) => {
  expect(languageSelfName(tag)).toBe(tag);
});

it("keeps region tags verbatim alongside the name", () => {
  expect(productLanguageLabel("de-DE")).toBe("Deutsch (de-DE)");
  expect(productLanguageLabel("en-US")).toBe("English (en-US)");
  expect(productLanguageLabel("tlh")).toBe("Klingonisch/Klingon");
});

it("does not invent a name from English when the runtime lacks locale data", () => {
  vi.spyOn(Intl.DisplayNames, "supportedLocalesOf").mockReturnValue([]);
  expect(languageSelfName("fr-FR")).toBe("fr-FR");
});

it("survives a runtime without DisplayNames", () => {
  vi.spyOn(Intl, "DisplayNames", "get").mockReturnValue(undefined as unknown as typeof Intl.DisplayNames);
  expect(languageSelfName("fr-FR")).toBe("fr-FR");
  expect(languageSelfName("tlh")).toBe("Klingonisch/Klingon");
});
