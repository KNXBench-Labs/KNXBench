/** Display fallbacks must never become imported or persisted device names. */
import { describe, expect, it } from "vitest";
import { translateFor, type Translate } from "./i18n";
import { deviceLabel } from "./devicePresentation";
import { isBlankName } from "./features/missingNames/client";
import { isBlankName as neutralIsBlankName } from "./blankName";

const t: Translate = (key, params) => translateFor("en", key, params);
describe("deviceLabel", () => {
  it.each(["", " \t\n", "\u00a0", "\u2003"])("displays unnamed device ID without mutating %j", (name) => {
    const device = Object.freeze({ id: 42, name });
    expect(deviceLabel(t, device)).toBe("Unnamed device (ID 42)");
    expect(device.name).toBe(name);
  });
  it.each(["Dimmer", "  Aktor 🛠  ", "<script>literal</script>", "0"])("preserves the exact nonblank name %j", (name) => {
    expect(deviceLabel(t, { id: 0, name })).toBe(name);
  });
  it("localizes German and falls back to English for a missing pack key", () => {
    expect(deviceLabel((key, params) => translateFor("de", key, params), { id: 3, name: "" }))
      .toBe("Unbenanntes Gerät (ID 3)");
    expect(deviceLabel((key, params) => translateFor("tlh", key, params), { id: 3, name: "" }))
      .toBe("Unnamed device (ID 3)");
  });
  it("uses the same blank rule as the missing-name wizard", () => {
    expect(isBlankName).toBe(neutralIsBlankName);
  });
});
