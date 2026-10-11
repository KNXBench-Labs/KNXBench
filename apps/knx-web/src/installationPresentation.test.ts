/** Display fallbacks must never become imported or persisted installation names. */
import { describe, expect, it } from "vitest";
import { translateFor, type Translate } from "./i18n";
import { installationLabel } from "./installationPresentation";

const t: Translate = (key, params) => translateFor("en", key, params);
describe("installationLabel", () => {
  it.each(["", " \t\n", "\u00a0"])("displays unnamed installation ID without mutating %j", (name) => {
    const installation = Object.freeze({ id: 7, name });
    expect(installationLabel(t, installation)).toBe("Unnamed installation (ID 7)");
    expect(installation.name).toBe(name);
  });
  it.each(["Main", "  Annex 🛠  ", "<script>literal</script>"])("preserves the exact nonblank name %j", (name) => {
    expect(installationLabel(t, { id: 0, name })).toBe(name);
  });
  it("localizes German and falls back to English for a missing pack key", () => {
    expect(installationLabel((key, params) => translateFor("de", key, params), { id: 0, name: "" }))
      .toBe("Unbenannte Installation (ID 0)");
    expect(installationLabel((key, params) => translateFor("tlh", key, params), { id: 0, name: "" }))
      .toBe("Unnamed installation (ID 0)");
  });
});
