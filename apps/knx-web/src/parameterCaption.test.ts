/** Caption fallbacks preserve source evidence without interpreting manufacturer templates. */
import { describe, expect, it } from "vitest";
import { parameterCaption } from "./parameterCaption";
import { messages } from "./messages/en";
import { translateFor, type Translate } from "./i18n";

const t: Translate = (key, values) => translateFor("en", key, values);

describe("parameterCaption", () => {
  it("shows filled values verbatim and explains their exact source", () => {
    for (const source of ["StoredValue", "ProductDefault", "TemplateDefault", "EmptyValue"] as const) {
      const caption = parameterCaption({ label: "Functions: Kitchen", labelFill: { template: "Functions: {{0}}", source } }, "Group", t);
      expect(caption.label).toBe("Functions: Kitchen");
      expect(caption.title).toContain(messages[`comTable.textFill.${source}`]);
      expect(caption.title).toContain("Functions: {{0}}");
    }
  });

  it("keeps missing references explicit and never guesses the template's default", () => {
    for (const source of ["NoReference", "UnknownParameter", "NotTextParameter", "Undecided"] as const) {
      const input = { label: "{{0:Not verified}}", labelFill: { template: "{{0:Not verified}}", source } };
      const caption = parameterCaption(input, "Group 3", t);
      expect(caption.label).toBe("Group 3");
      expect(caption.title).toContain(messages[`comTable.textFill.${source}`]);
      expect(input.label).toBe("{{0:Not verified}}");
    }
  });

  it("safely labels unsupported, malformed, empty and marker-only captions without changing them", () => {
    for (const label of [null, "", "   ", "#", " # ", "{{0}}", "{{1:Keep}}", "a {{unclosed", "stray }}"]) {
      const input = { label };
      const caption = parameterCaption(input, "Group 2", t);
      expect(caption.label).toBe("Group 2");
      expect(caption.title).toContain("caption");
      expect(input.label).toBe(label);
    }
    expect(parameterCaption({ label: "Function #1" }, "Group", t)).toEqual({ label: "Function #1", title: undefined });
  });
});
