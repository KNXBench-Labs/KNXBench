/** Verify shared serial candidate controls preserve drafts and never connect or discover. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import BusTransportFields from "./BusTransportFields";
import { getSetting, resetSettingsForTests, setSetting } from "./settingsStore";
import { loadSerialTarget } from "./serialPreference";
import { connectBusSession, resetBusConnectionForTests } from "./busConnection";
import { resetUiLanguageForTests } from "./uiLanguage";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
import type { SerialCandidates } from "./serialCandidates";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const candidate = { path: "/dev/serial/by-id/usb-fictional-interface00", target: "/dev/ttyUSB1" };
const found: SerialCandidates = { status: "available", candidates: [candidate], truncated: false, omittedEntries: 0 };
let host: HTMLDivElement, root: Root;
let fetcher: ReturnType<typeof vi.fn>;
const flush = () => act(async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); });
async function render(disabled = false) { await act(async () => root.render(<BusTransportFields disabled={disabled} />)); await flush(); }
async function click(text: string) {
  const button = [...host.querySelectorAll<HTMLButtonElement>("button")].find(node => node.textContent === text);
  expect(button).toBeDefined(); await act(async () => button!.click()); await flush();
}
async function input(label: string, value: string) {
  const field = [...host.querySelectorAll<HTMLLabelElement>("label")].find(node => node.textContent === label)!.querySelector("input")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(field, value);
    field.dispatchEvent(new Event("input", { bubbles: true }));
    field.dispatchEvent(new Event("change", { bubbles: true }));
  });
}
function response(value: unknown, status = 200) { return new Response(JSON.stringify(value), { status }); }
beforeEach(() => {
  localStorage.clear(); resetSettingsForTests(); resetUiLanguageForTests();
  setSetting("uiLanguage", "en"); setSetting("preferredBusTransport", "serial");
  setSetting("serialPath", "/synthetic/manual"); setSetting("serialSource", "1.1.250"); setSetting("serialMode", "passive");
  setSetting("preferredGateway", "192.0.2.10:3671");
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  fetcher = vi.fn().mockRejectedValue(new Error("unexpected request")); vi.stubGlobal("fetch", fetcher);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.unstubAllGlobals(); });
for (const language of ["en", "de"] as const) {
  const m = language === "en" ? en : de;
  it(`${language}: scans only explicitly, keeps pending manual edits and selects only the path`, async () => {
    setSetting("uiLanguage", language); resetUiLanguageForTests();
    let finish!: (value: Response) => void;
    fetcher.mockImplementation(() => new Promise<Response>(resolve => { finish = resolve; }));
    const session = { sessionId: 17, serverIncarnation: "fictional" }; localStorage.setItem("knx-desktop:bus-session-context", JSON.stringify(session));
    await render(); expect(fetcher).not.toHaveBeenCalled();
    expect(host.textContent).toContain(m["serialCandidates.warning"]);
    const listButton = host.querySelector<HTMLButtonElement>(".serial-candidate-fields > button")!;
    await act(async () => { listButton.focus(); listButton.click(); listButton.click(); });
    await flush();
    expect(host.textContent).toContain(m["serialCandidates.loading"]);
    expect(listButton.disabled).toBe(false);
    expect(listButton.getAttribute("aria-disabled")).toBe("true");
    expect(document.activeElement).toBe(listButton);
    expect(fetcher).toHaveBeenCalledTimes(1);
    await click(m["serialCandidates.list"]);
    expect(fetcher).toHaveBeenCalledTimes(1);
    await input(m["busTransport.path"], "/synthetic/edited-while-pending");
    await input(m["busTransport.source"], "1.1.249");
    await act(async () => setSetting("serialMode", "active"));
    expect(loadSerialTarget()).toEqual({ path: "/synthetic/edited-while-pending", source: "1.1.249", mode: "active" });
    await act(async () => finish(response(found))); await flush();
    expect(loadSerialTarget().path).toBe("/synthetic/edited-while-pending");
    await click(m["serialCandidates.use"].replace("{path}", candidate.path));
    expect(loadSerialTarget()).toEqual({ path: candidate.path, source: "1.1.249", mode: "active" });
    expect(getSetting("preferredBusTransport")).toBe("serial"); expect(getSetting("preferredGateway")).toBe("192.0.2.10:3671");
    expect(localStorage.getItem("knx-desktop:bus-session-context")).toBe(JSON.stringify(session));
    expect(fetcher).toHaveBeenCalledExactlyOnceWith("/api/bus/serial-candidates", { headers: undefined, method: "GET", cache: "no-store" });
  });
  const states: [unknown, number, keyof typeof en][] = [
    [{ status: "available", candidates: [], truncated: false, omittedEntries: 0 }, 200, "serialCandidates.empty"],
    [{ ...found, truncated: true, omittedEntries: 5 }, 200, "serialCandidates.truncated"],
    [{ status: "unavailable", reason: "missingDirectory" }, 200, "serialCandidates.missingDirectory"],
    [{ status: "unavailable", reason: "permissionDenied" }, 200, "serialCandidates.permissionDenied"],
    [{ status: "unavailable", reason: "io" }, 200, "serialCandidates.io"],
    [{ status: "unsupported" }, 200, "serialCandidates.unsupported"],
    [{ error: "old server" }, 404, "serialCandidates.legacy"],
    [{ error: "failed" }, 500, "serialCandidates.failed"],
    [{ ...found, candidates: [{ path: "/unsafe", target: "/dev/ttyUSB1" }] }, 200, "serialCandidates.failed"],
  ];
  it.each(states)(`${language}: discloses terminal state %# without changing drafts`, async (value, status, key) => {
    setSetting("uiLanguage", language); resetUiLanguageForTests(); fetcher.mockResolvedValue(response(value, status));
    await render(); const before = loadSerialTarget(); await click(m["serialCandidates.list"]);
    expect(host.textContent).toContain(m[key]); expect(loadSerialTarget()).toEqual(before);
    if (key === "serialCandidates.truncated") expect(host.textContent).toContain(m["serialCandidates.omitted"].replace("{count}", "5"));
    expect(fetcher).toHaveBeenCalledTimes(1);
  });
}
it("ignores stale completion after switching away and remounting the serial fields", async () => {
  let finish!: (value: Response) => void;
  fetcher.mockImplementationOnce(() => new Promise<Response>(resolve => { finish = resolve; }));
  await render(); await click(en["serialCandidates.list"]);
  await act(async () => setSetting("preferredBusTransport", "ip"));
  await act(async () => setSetting("preferredBusTransport", "serial"));
  await act(async () => finish(response(found))); await flush();
  expect(host.querySelectorAll(".serial-candidate-fields li")).toHaveLength(0);
  expect(loadSerialTarget().path).toBe("/synthetic/manual"); expect(fetcher).toHaveBeenCalledTimes(1);
  fetcher.mockResolvedValueOnce(response(found)); await click(en["serialCandidates.list"]);
  expect(host.querySelectorAll(".serial-candidate-fields li")).toHaveLength(1); expect(fetcher).toHaveBeenCalledTimes(2);
});
it("ignores a rejected request after unmount, with no automatic retry", async () => {
  let fail!: (error: Error) => void;
  fetcher.mockImplementationOnce(() => new Promise<Response>((_, reject) => { fail = reject; }));
  await render(); await click(en["serialCandidates.list"]); await act(async () => root.render(null));
  await act(async () => fail(new Error("offline"))); await flush();
  expect(host.textContent).toBe(""); expect(loadSerialTarget().path).toBe("/synthetic/manual"); expect(fetcher).toHaveBeenCalledTimes(1);
});
it("disables listing and selection under the existing parent busy guard", async () => {
  fetcher.mockResolvedValue(response(found)); await render(true);
  await click(en["serialCandidates.list"]); expect(fetcher).not.toHaveBeenCalled();
  await render(); await click(en["serialCandidates.list"]); await render(true);
  const choose = host.querySelector<HTMLButtonElement>(".serial-candidate-fields li button")!;
  expect(choose.disabled).toBe(true); await act(async () => choose.click());
  expect(loadSerialTarget().path).toBe("/synthetic/manual"); expect(fetcher).toHaveBeenCalledTimes(1);
});

const tunnelSelect = () => host.querySelector<HTMLSelectElement>(".bus-transport-fields label:nth-of-type(2) select");
async function choose(select: HTMLSelectElement, value: string) {
  await act(async () => { select.value = value; select.dispatchEvent(new Event("change", { bubbles: true })); });
}
for (const language of ["en", "de"] as const) {
  const m = language === "en" ? en : de;
  it(`${language}: KNXnet/IP offers UDP by default and TCP explicitly, states its scope and never connects`, async () => {
    setSetting("uiLanguage", language); resetUiLanguageForTests(); setSetting("preferredBusTransport", "ip");
    await render();
    const select = tunnelSelect()!;
    expect(host.querySelector(".bus-transport-fields label:nth-of-type(2)")!.textContent).toContain(m["busTransport.ipTunnel.label"]);
    expect([...select.options].map(option => [option.value, option.textContent])).toEqual([
      ["udp", m["busTransport.ipTunnel.udp"]], ["tcp", m["busTransport.ipTunnel.tcp"]],
    ]);
    expect(select.value).toBe("udp"); expect(getSetting("preferredIpTunnelTransport")).toBeUndefined();
    // The note limits the scope to monitoring and group traffic; programming stays UDP.
    expect(host.textContent).toContain(m["busTransport.ipTunnel.note"]);
    expect(m["busTransport.ipTunnel.note"]).toMatch(/UDP/);
    await choose(select, "tcp");
    expect(getSetting("preferredIpTunnelTransport")).toBe("tcp"); expect(tunnelSelect()!.value).toBe("tcp");
    expect(getSetting("preferredBusTransport")).toBe("ip");
    await choose(tunnelSelect()!, "udp");
    expect(getSetting("preferredIpTunnelTransport")).toBe("udp");
    expect(fetcher).not.toHaveBeenCalled(); // Selecting a transport never connects, discovers or lists.
  });
  it(`${language}: marks TCP and USB/serial as experimental, never UDP (REL-01 B-14)`, async () => {
    setSetting("uiLanguage", language); resetUiLanguageForTests(); setSetting("preferredBusTransport", "ip");
    await render();
    const note = () => host.querySelector(".bus-transport-experimental");
    expect(note()).toBeNull();
    expect(m["busTransport.ipTunnel.tcp"]).toMatch(language === "en" ? /experimental/ : /experimentell/);
    await choose(tunnelSelect()!, "tcp");
    expect(note()!.textContent).toBe(m["transport.experimentalNote"]);
    expect(m["transport.experimentalNote"]).toMatch(/UDP/);
    await choose(tunnelSelect()!, "udp");
    expect(note()).toBeNull();
    const transportSelect = host.querySelector<HTMLSelectElement>(".bus-transport-fields select")!;
    expect([...transportSelect.options].map(option => option.textContent)).toEqual(["KNXnet/IP", m["busTransport.serialOption"]]);
    await choose(transportSelect, "serial");
    expect(note()!.textContent).toBe(m["transport.experimentalNote"]);
    expect(fetcher).not.toHaveBeenCalled(); // Marking is presentation only: no request, no connect.
  });
}
it.each(["TCP", "tls", "", null, 7, {}, ["tcp"]])("shows UDP, never TCP, for the invalid saved value %j", async (saved) => {
  setSetting("preferredBusTransport", "ip"); setSetting("preferredIpTunnelTransport", saved);
  await render(); expect(tunnelSelect()!.value).toBe("udp"); expect(fetcher).not.toHaveBeenCalled();
});
it("hides the IP tunnel choice for serial and keeps the saved choice for a later switch back", async () => {
  setSetting("preferredBusTransport", "ip"); setSetting("preferredIpTunnelTransport", "tcp"); await render();
  expect(tunnelSelect()!.value).toBe("tcp");
  await act(async () => setSetting("preferredBusTransport", "serial"));
  expect(host.textContent).not.toContain(en["busTransport.ipTunnel.label"]); expect(tunnelSelect()).toBeNull();
  await act(async () => setSetting("preferredBusTransport", "ip"));
  expect(tunnelSelect()!.value).toBe("tcp");
});
it("disables the tunnel choice under the parent guard and while this window's connect is pending", async () => {
  setSetting("preferredBusTransport", "ip"); await render(true);
  expect(tunnelSelect()!.disabled).toBe(true);
  await choose(tunnelSelect()!, "tcp"); expect(getSetting("preferredIpTunnelTransport")).toBeUndefined();
  await render(false); expect(tunnelSelect()!.disabled).toBe(false);
  fetcher.mockImplementation(() => new Promise<Response>(() => {}));
  await act(async () => { void connectBusSession("192.0.2.1:3671"); });
  expect(tunnelSelect()!.disabled).toBe(true);
  await choose(tunnelSelect()!, "tcp"); expect(getSetting("preferredIpTunnelTransport")).toBeUndefined();
  resetBusConnectionForTests();
});
