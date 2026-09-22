/** Verifies the preferred gateway stays a passive value in the shared settings record. */
// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import {
  PREFERRED_GATEWAY_KEY,
  loadPreferredGateway,
  savePreferredGateway,
} from "./gatewayPreference";
import { SETTINGS_CACHE_KEY, resetSettingsForTests } from "./settingsStore";

afterEach(() => resetSettingsForTests());

describe("preferred gateway", () => {
  it("defaults missing and non-string values to an empty field", () => {
    expect(loadPreferredGateway()).toBe("");
    window.localStorage.setItem(
      SETTINGS_CACHE_KEY,
      JSON.stringify({ schemaVersion: 1, settings: { [PREFERRED_GATEWAY_KEY]: 42 } }),
    );
    resetSettingsForTests();
    window.localStorage.setItem(
      SETTINGS_CACHE_KEY,
      JSON.stringify({ schemaVersion: 1, settings: { [PREFERRED_GATEWAY_KEY]: 42 } }),
    );
    expect(loadPreferredGateway()).toBe("");
  });

  it("trims a saved value and keeps it inside the one cache document", () => {
    savePreferredGateway(" 192.0.2.10:3671 ");
    expect(loadPreferredGateway()).toBe("192.0.2.10:3671");
    const cached = JSON.parse(window.localStorage.getItem(SETTINGS_CACHE_KEY)!);
    expect(cached.settings).toEqual({ preferredGateway: "192.0.2.10:3671" });
    expect(window.localStorage.getItem("knx-desktop:preferred-gateway")).toBeNull();
  });

  it("unsets blank input instead of storing an empty string", () => {
    savePreferredGateway("192.0.2.10:3671");
    savePreferredGateway("   ");
    expect(loadPreferredGateway()).toBe("");
    const cached = JSON.parse(window.localStorage.getItem(SETTINGS_CACHE_KEY)!);
    expect(cached.settings).toEqual({});
  });
});
