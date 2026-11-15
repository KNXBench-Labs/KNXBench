/** Tests the explicit TCP choice of the service-control panel without any UDP retry. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as api from "./api";
import * as tcp from "./serviceControlApi";
import ServiceControlPanel from "./ServiceControlPanel";
import { resetUiLanguageForTests } from "./uiLanguage";

vi.mock("./api", async (original) => ({ ...await original<typeof import("./api")>(), readServiceControl: vi.fn(), writeServiceControl: vi.fn() }));
vi.mock("./serviceControlApi", async (original) => ({ ...await original<typeof import("./serviceControlApi")>(), readServiceControlTcp: vi.fn(), writeServiceControlTcp: vi.fn() }));
const read = vi.mocked(api.readServiceControl), write = vi.mocked(api.writeServiceControl);
const readTcp = vi.mocked(tcp.readServiceControlTcp), writeTcp = vi.mocked(tcp.writeServiceControlTcp);
const address = "1.1.67", gateway = "192.0.2.10:3671";
const phrase = `I confirm individual-address write enable to ${address}`;
const binding = { serverIncarnation: "fixture", projectIncarnation: 3, revision: 1 };
const before = { address, raw: "0000", mask: "0701", individualAddressWriteEnabled: false };
const facts = (skipped: number) => ({ requestedTransport: "tcp", actualTransport: "tcp", assignedAddress: "1.1.250", skippedFramesBeforeDisconnect: skipped }) as const;
let host: HTMLDivElement | undefined;

async function change(selector: string, value: string, proto: object = HTMLInputElement.prototype, event = "input") {
  const element = host!.querySelector<HTMLElement>(selector)!;
  await act(async () => { Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(element, value); element.dispatchEvent(new Event(event, { bubbles: true })); });
}
const field = (name: string, value: string) => change(`input[name="${name}"]`, value);
const choose = (value: string) => change('select[name="transport"]', value, HTMLSelectElement.prototype, "change");
const click = (selector: string) => act(async () => host!.querySelector<HTMLButtonElement>(selector)!.click());
async function mount(withBinding = true) {
  host = document.createElement("div"); document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => root.render(<ServiceControlPanel projectOpen binding={withBinding ? binding : undefined} />));
  await field("address", address); await field("gateway", gateway);
  return root;
}
afterEach(() => { host?.remove(); host = undefined; for (const m of [read, write, readTcp, writeTcp]) m.mockReset(); resetUiLanguageForTests(); });

describe("explicit TCP service control", () => {
  it("defaults to UDP, contacts nothing on choice and keeps the legacy call shape for UDP", async () => {
    read.mockResolvedValue(before);
    const root = await mount();
    expect(host!.querySelector<HTMLSelectElement>('select[name="transport"]')!.value).toBe("udp");
    await choose("tcp"); await choose("udp");
    expect([read, write, readTcp, writeTcp].map((m) => m.mock.calls.length)).toEqual([0, 0, 0, 0]);
    await click(".service-control-read");
    expect(read).toHaveBeenCalledWith(address, gateway); expect(readTcp).not.toHaveBeenCalled();
    expect(host!.querySelector(".service-control-connection")).toBeNull();
    root.unmount();
  });
  it("reads and writes over the TCP siblings only, freezes the transport and shows the facts", async () => {
    readTcp.mockResolvedValue({ ...before, connection: facts(0) });
    writeTcp.mockResolvedValue({ before, after: "0004", individualAddressWriteEnabled: true, written: true, backupPath: "[LOCAL BACKUP]", connection: facts(2) });
    const root = await mount();
    await choose("tcp");
    await click(".service-control-read");
    expect(readTcp).toHaveBeenCalledWith(address, gateway, binding); expect(read).not.toHaveBeenCalled();
    // Zero skipped frames is a fact, shown as 0.
    expect(host!.querySelector(".service-control-connection")!.textContent).toContain("skipped frames before disconnect: 0");
    await click("button.service-control-review");
    expect(host!.querySelector(".service-control-frozen-transport")!.textContent).toContain("TCP");
    await field("confirmation", phrase); await click(".service-control-write");
    expect(writeTcp).toHaveBeenCalledWith(address, gateway, true, phrase, binding); expect(write).not.toHaveBeenCalled();
    expect(host!.querySelector(".service-control-connection")!.textContent).toContain("skipped frames before disconnect: 2");
    expect(host!.querySelector<HTMLSelectElement>('select[name="transport"]')!.value).toBe("udp");
    root.unmount();
  });
  it("shows unavailable facts instead of zero when no tunnel existed", async () => {
    readTcp.mockRejectedValue(new tcp.ServiceControlTcpRefusal(502, "refused", { requestedTransport: "tcp", actualTransport: null, assignedAddress: null, skippedFramesBeforeDisconnect: null }));
    const root = await mount(); await choose("tcp"); await click(".service-control-read");
    expect(host!.querySelector(".service-control-connection")!.textContent).toContain("unavailable");
    expect(host!.querySelector(".service-control-connection")!.textContent).not.toContain("0");
    expect(host!.querySelector('[role="alert"]')!.textContent).toBe("refused");
    root.unmount();
  });
  it.each([[404, "does not offer"], [405, "does not offer"], [0, "lacked valid connection facts"]])("treats status %i as a refusal with no UDP retry", async (status, text) => {
    readTcp.mockRejectedValue(new tcp.ServiceControlTcpRefusal(status, null, null));
    const root = await mount(); await choose("tcp"); await click(".service-control-read");
    expect(host!.querySelector('[role="alert"]')!.textContent).toContain(text);
    expect(host!.querySelector(".service-control-observation")).toBeNull();
    expect(read).not.toHaveBeenCalled(); expect(write).not.toHaveBeenCalled();
    root.unmount();
  });
  it("offers no TCP choice without a project snapshot", async () => {
    const root = await mount(false);
    expect(host!.querySelector<HTMLOptionElement>('select[name="transport"] option[value="tcp"]')!.disabled).toBe(true);
    root.unmount();
  });
  it("drops a read when the transport changes, so a UDP reading cannot authorise a TCP write", async () => {
    read.mockResolvedValue(before);
    const root = await mount(); await click(".service-control-read");
    expect(host!.querySelector(".service-control-observation")).not.toBeNull();
    await choose("tcp");
    expect(host!.querySelector(".service-control-observation")).toBeNull();
    root.unmount();
  });
});
