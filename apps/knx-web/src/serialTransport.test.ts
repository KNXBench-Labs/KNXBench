/** Validate serial paths, sources and independent persisted transport preferences. */
// @vitest-environment happy-dom
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, expect, it } from "vitest";
import { loadSerialTarget, useIpTunnelTransport } from "./serialPreference";
import {
  gatewayWithTransport, isIpTunnelTransport, isTcpEndpoint, validSerialPath, validSerialSource, validSerialTarget,
} from "./serialTransport";
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
it("tells a TCP endpoint from a serial target and a UDP string by real guards", () => {
  const serial = { path: "/synthetic/tty", source: "1.1.250", mode: "passive" };
  expect(isTcpEndpoint({ kind: "tcp", gateway: "192.0.2.1:3671" })).toBe(true);
  for (const value of [serial, "192.0.2.1:3671", null, undefined, [], {}, { kind: "tcp" }, { kind: "tcp", gateway: "" },
    { kind: "tcp", gateway: 3671 }, { kind: "udp", gateway: "192.0.2.1:3671" }, { kind: "tcp", gateway: "192.0.2.1:3671", extra: 1 }]) {
    expect(isTcpEndpoint(value), JSON.stringify(value)).toBe(false);
  }
  // Neither shape is the other, so no `typeof object` shortcut can confuse them.
  expect(validSerialTarget({ kind: "tcp", gateway: "192.0.2.1:3671" })).toBe(false);
  expect(isIpTunnelTransport("udp") && isIpTunnelTransport("tcp")).toBe(true);
  for (const value of ["TCP", "Udp", "", null, undefined, 1, "ip"]) expect(isIpTunnelTransport(value), String(value)).toBe(false);
  expect(gatewayWithTransport("192.0.2.1:3671", "tcp")).toBe("192.0.2.1:3671 · TCP");
  expect(gatewayWithTransport("192.0.2.1:3671", "udp")).toBe("192.0.2.1:3671");
  expect(gatewayWithTransport("192.0.2.1:3671", undefined)).toBe("192.0.2.1:3671");
});
it("persists the tunnel transport safely: default udp, exact tcp, invalid values fall back and drafts stay independent", async () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  let current: string = "", choose = (_: "udp" | "tcp") => {};
  function Probe() { const [value, select] = useIpTunnelTransport(); current = value; choose = select; return null; }
  const host = document.createElement("div"); const root = createRoot(host);
  await act(async () => root.render(createElement(Probe)));
  expect(current).toBe("udp"); expect(getSetting("preferredIpTunnelTransport")).toBeUndefined();
  for (const invalid of ["TCP", "ip", "", null, 1, true, {}, ["tcp"]]) {
    await act(async () => setSetting("preferredIpTunnelTransport", invalid)); expect(current, JSON.stringify(invalid)).toBe("udp");
  }
  setSetting("preferredBusTransport", "serial"); setSetting("serialPath", "/synthetic/tty");
  await act(async () => choose("tcp"));
  expect(current).toBe("tcp"); expect(getSetting("preferredIpTunnelTransport")).toBe("tcp");
  expect(getSetting("preferredBusTransport")).toBe("serial"); expect(getSetting("serialPath")).toBe("/synthetic/tty");
  await act(async () => choose("udp")); expect(current).toBe("udp");
  expect(loadPreferredGateway()).toBe(""); // The gateway preference is untouched by either choice.
  await act(async () => root.unmount());
});
