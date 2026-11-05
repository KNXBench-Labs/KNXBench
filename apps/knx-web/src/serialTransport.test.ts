/** Validate serial paths, sources and independent persisted transport preferences. */
// @vitest-environment happy-dom
import { beforeEach, expect, it } from "vitest";
import { loadSerialTarget } from "./serialPreference";
import { validSerialPath, validSerialSource, validSerialTarget } from "./serialTransport";
import { getSetting, resetSettingsForTests, setSetting } from "./settingsStore";
import { loadPreferredGateway, savePreferredGateway } from "./gatewayPreference";

beforeEach(() => { localStorage.clear(); resetSettingsForTests(); });
it("validates canonical sources, excluded/reserved addresses and explicit raw paths", () => {
  for (const value of ["1.1.250", "0.0.1", "15.15.254"]) expect(validSerialSource(value), value).toBe(true);
  for (const value of ["", "0.0.0", "15.15.255", "1.1.220", "16.1.1", "1.16.1", "1.1.256", "01.1.1", "1.1.1\n", " 1.1.1"]) expect(validSerialSource(value), value).toBe(false);
  for (const path of ["", " ", " x", "x\n", "x\0"]) expect(validSerialPath(path), path).toBe(false);
  expect(validSerialTarget({ path: "/synthetic/tty", source: "1.1.250", mode: "passive" })).toBe(true);
  for (const value of [null, {}, {path:"x",source:"1.1.250"}, {path:"x",source:"1.1.250",mode:"auto"}, {path:"x",source:"1.1.250",mode:"active",gateway:"192.0.2.1:3671"}]) expect(validSerialTarget(value)).toBe(false);
});
it("loads older settings safely and preserves independent serial drafts and IP preference", () => {
  savePreferredGateway("192.0.2.10:3671");
  expect(loadSerialTarget()).toEqual({path:"",source:"",mode:"passive"});
  setSetting("preferredBusTransport", "serial"); setSetting("serialPath", "/synthetic/tty");
  setSetting("serialSource", "1.1.250"); setSetting("serialMode", "active");
  expect(loadSerialTarget()).toEqual({path:"/synthetic/tty",source:"1.1.250",mode:"active"});
  expect(loadPreferredGateway()).toBe("192.0.2.10:3671");
  setSetting("preferredBusTransport", "ip");
  expect(getSetting("serialPath")).toBe("/synthetic/tty");
  setSetting("serialMode", "invented"); setSetting("serialSource", 17);
  expect(loadSerialTarget()).toEqual({path:"/synthetic/tty",source:"",mode:"passive"});
});
