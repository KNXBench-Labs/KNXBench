/** Tests for the documentation section/language options sent to preview and export. */
import { describe, expect, it } from "vitest";
import {
  DOCUMENTATION_SECTIONS,
  documentationLanguageFor,
  documentationOptionsFor,
  type DocumentationSection,
} from "./documentationOptions";

describe("documentationOptionsFor", () => {
  it("lists the selected sections in document order, whatever the selection order", () => {
    const selected = new Set<DocumentationSection>(["devices", "summary", "buildings"]);
    expect(documentationOptionsFor(selected, "en")).toEqual({
      sections: ["summary", "buildings", "devices"],
      language: "en",
    });
  });

  it("sends an empty list, not a missing one, when nothing is selected", () => {
    // An absent `sections` would make the server fall back to all of them.
    expect(documentationOptionsFor(new Set(), "en").sections).toEqual([]);
  });

  it("covers exactly the five sections the server accepts", () => {
    expect([...DOCUMENTATION_SECTIONS]).toEqual([
      "summary",
      "topology",
      "buildings",
      "groupAddresses",
      "devices",
    ]);
  });
});

describe("documentationLanguageFor", () => {
  it("maps German to German and every other UI language to English", () => {
    expect(documentationLanguageFor("de")).toBe("de");
    expect(documentationLanguageFor("en")).toBe("en");
    expect(documentationLanguageFor("fr")).toBe("en");
  });
});
