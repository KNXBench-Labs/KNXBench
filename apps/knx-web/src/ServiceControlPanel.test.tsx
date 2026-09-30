/** The service-control UI is explicit at every bus and write boundary. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as api from "./api";
import ServiceControlPanel from "./ServiceControlPanel";
import { resetUiLanguageForTests } from "./uiLanguage";

vi.mock("./api", async (original) => ({
  ...await original<typeof import("./api")>(),
  readServiceControl: vi.fn(),
  writeServiceControl: vi.fn(),
}));
const read = vi.mocked(api.readServiceControl);
const write = vi.mocked(api.writeServiceControl);
let host: HTMLDivElement | undefined;
const address = "1.1.67";
const gateway = "192.0.2.10:3671";
const before = { address, raw: "0000", mask: "0701", individualAddressWriteEnabled: false };

function input(name: string): HTMLInputElement {
  return host!.querySelector<HTMLInputElement>(`input[name="${name}"]`)!;
}
async function change(name: string, value: string) {
  const element = input(name);
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(element, value);
    element.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
async function click(selector: string) {
  await act(async () => host!.querySelector<HTMLButtonElement>(selector)!.click());
}
async function mount(projectOpen = true) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => root.render(<ServiceControlPanel projectOpen={projectOpen} />));
  return root;
}
afterEach(() => {
  host?.remove(); host = undefined;
  read.mockReset(); write.mockReset();
  resetUiLanguageForTests();
});

describe("service-control debug action", () => {
  it("never contacts the gateway on mount, input, or opening a review", async () => {
    const root = await mount();
    await change("address", address);
    await change("gateway", gateway);
    expect(read).not.toHaveBeenCalled();
    expect(write).not.toHaveBeenCalled();
    await click(".service-control-read");
    expect(read).toHaveBeenCalledWith(address, gateway);
    expect(host!.querySelector('[role="alert"]')?.textContent).toContain("another device");
    expect(write).not.toHaveBeenCalled();
    root.unmount();
  });

  it("reads only after an explicit click, then demands a new device-specific typed phrase before writing", async () => {
    read.mockResolvedValue(before);
    write.mockResolvedValue({ before, after: "0004", individualAddressWriteEnabled: true, written: true, backupPath: "[PRIVATE BACKUP PATH]" });
    const root = await mount();
    await change("address", address);
    await change("gateway", gateway);
    await click(".service-control-read");
    expect(host!.textContent).toContain("0000");
    await click(".service-control-review");
    expect(write).not.toHaveBeenCalled();
    expect(host!.querySelector<HTMLButtonElement>(".service-control-write")!.disabled).toBe(true);
    await change("confirmation", "I confirm download to 1.1.67");
    expect(host!.querySelector<HTMLButtonElement>(".service-control-write")!.disabled).toBe(true);
    await change("confirmation", "I confirm individual-address write enable to 1.1.67");
    await click(".service-control-write");
    expect(write).toHaveBeenCalledTimes(1);
    expect(write).toHaveBeenCalledWith(address, gateway, true, "I confirm individual-address write enable to 1.1.67");
    expect(host!.textContent).toContain("[PRIVATE BACKUP PATH]");
    expect(host!.querySelector('input[name="confirmation"]')).toBeNull();
    root.unmount();
  });

  it("withholds a success claim when a server reports a write without recovery evidence", async () => {
    read.mockResolvedValue(before);
    write.mockResolvedValue({ before, after: "0004", individualAddressWriteEnabled: true, written: true, backupPath: null });
    const root = await mount();
    await change("address", address); await change("gateway", gateway);
    await click(".service-control-read"); await click(".service-control-review");
    await change("confirmation", "I confirm individual-address write enable to 1.1.67");
    await click(".service-control-write");
    expect(host!.querySelector('[role="alert"]')).not.toBeNull();
    expect(host!.textContent).not.toContain("Write verified");
    root.unmount();
  });

  it("refuses unexpected read targets, invalid inputs and a missing project without a write", async () => {
    read.mockResolvedValue({ ...before, address: "1.1.68" });
    const root = await mount();
    await change("address", "1.1.0"); await change("gateway", gateway);
    await click(".service-control-read");
    expect(read).not.toHaveBeenCalled();
    await change("address", address); await change("gateway", "bad host");
    await click(".service-control-read");
    expect(read).not.toHaveBeenCalled();
    await change("gateway", gateway); await click(".service-control-read");
    expect(read).toHaveBeenCalledTimes(1);
    expect(host!.querySelector(".service-control-review")).toBeNull();
    expect(write).not.toHaveBeenCalled();
    root.unmount();
    const withoutProject = await mount(false);
    expect(host!.querySelector<HTMLButtonElement>(".service-control-read")?.disabled).toBe(true);
    withoutProject.unmount();
  });

  it("clears a reviewed phrase when the address changes and refuses write after a 403", async () => {
    read.mockResolvedValueOnce(before).mockRejectedValueOnce(new Error("403 disabled"));
    const root = await mount();
    await change("address", address); await change("gateway", gateway);
    await click(".service-control-read"); await click(".service-control-review");
    await change("confirmation", "I confirm individual-address write enable to 1.1.67");
    await change("address", "1.1.68");
    expect(host!.querySelector(".service-control-write")).toBeNull();
    await click(".service-control-read");
    expect(host!.querySelector('[role="alert"]')).not.toBeNull();
    expect(write).not.toHaveBeenCalled();
    root.unmount();
  });

  it("treats a write whose pre-read differs from the reviewed value as uncertain", async () => {
    read.mockResolvedValue(before);
    write.mockResolvedValue({ before: { ...before, raw: "0010" }, after: "0014", individualAddressWriteEnabled: true, written: true, backupPath: "[LOCAL BACKUP]" });
    const root = await mount();
    await change("address", address); await change("gateway", gateway);
    await click(".service-control-read"); await click(".service-control-review");
    await change("confirmation", "I confirm individual-address write enable to 1.1.67");
    await click(".service-control-write");
    expect(host!.querySelector('[role="alert"]')?.textContent).toContain("does not match");
    expect(host!.querySelector(".service-control-result")).toBeNull();
    root.unmount();
  });

  it("surfaces a revoked server opt-in after the review, never reporting a write", async () => {
    read.mockResolvedValue(before);
    write.mockRejectedValue(Object.assign(new Error("403 debug action is off"), { status: 403 }));
    const root = await mount();
    await change("address", address); await change("gateway", gateway);
    await click(".service-control-read"); await click(".service-control-review");
    await change("confirmation", "I confirm individual-address write enable to 1.1.67");
    await click(".service-control-write");
    expect(write).toHaveBeenCalledTimes(1);
    expect(host!.querySelector('[role="alert"]')?.textContent).toContain("403");
    expect(host!.querySelector(".service-control-result")).toBeNull();
    root.unmount();
  });

  it("refuses a numeric property word instead of silently coercing it into hex", async () => {
    read.mockResolvedValue({ ...before, raw: 1234 as unknown as string, individualAddressWriteEnabled: true });
    const root = await mount();
    await change("address", address); await change("gateway", gateway);
    await click(".service-control-read");
    expect(host!.querySelector(".service-control-review")).toBeNull();
    expect(host!.querySelector('[role="alert"]')).not.toBeNull();
    expect(write).not.toHaveBeenCalled();
    root.unmount();
  });

  it("refuses a numeric write readback even when its digits resemble a bit-only change", async () => {
    const original = { ...before, raw: "1000" };
    read.mockResolvedValue(original);
    write.mockResolvedValue({ before: original, after: 1004 as unknown as string,
      individualAddressWriteEnabled: true, written: true, backupPath: "[LOCAL BACKUP]" });
    const root = await mount();
    await change("address", address); await change("gateway", gateway);
    await click(".service-control-read"); await click(".service-control-review");
    await change("confirmation", "I confirm individual-address write enable to 1.1.67");
    await click(".service-control-write");
    expect(host!.querySelector(".service-control-result")).toBeNull();
    expect(host!.querySelector('[role="alert"]')).not.toBeNull();
    root.unmount();
  });

  it("keeps the tunnel action locked while an in-flight write survives a project revision", async () => {
    let finish!: (value: api.ServiceControlWriteResponse) => void;
    read.mockResolvedValue(before);
    write.mockReturnValue(new Promise<api.ServiceControlWriteResponse>((resolve) => { finish = resolve; }));
    const root = await mount();
    await change("address", address); await change("gateway", gateway);
    await click(".service-control-read"); await click(".service-control-review");
    await change("confirmation", "I confirm individual-address write enable to 1.1.67");
    await click(".service-control-write");
    expect(host!.querySelector<HTMLButtonElement>(".service-control-read")!.disabled).toBe(true);
    await act(async () => root.render(<ServiceControlPanel projectOpen projectRevision={"changed"} />));
    expect(host!.querySelector<HTMLButtonElement>(".service-control-read")!.disabled).toBe(true);
    expect(host!.querySelector<HTMLInputElement>('input[name="address"]')!.disabled).toBe(true);
    await act(async () => finish({ before, after: "0004", individualAddressWriteEnabled: true,
      written: true, backupPath: "[LOCAL BACKUP]" }));
    expect(host!.querySelector<HTMLButtonElement>(".service-control-read")!.disabled).toBe(false);
    expect(host!.querySelector(".service-control-result")).toBeNull();
    root.unmount();
  });

  it("can explicitly clear the bit without changing any other octets", async () => {
    const enabled = { ...before, raw: "0004", individualAddressWriteEnabled: true };
    read.mockResolvedValue(enabled);
    write.mockResolvedValue({ before: enabled, after: "0000", individualAddressWriteEnabled: false, written: true, backupPath: "[LOCAL BACKUP]" });
    const root = await mount();
    await change("address", address); await change("gateway", gateway);
    await click(".service-control-read"); await click(".service-control-review");
    await change("confirmation", "I confirm individual-address write enable to 1.1.67");
    await click(".service-control-write");
    expect(write).toHaveBeenCalledWith(address, gateway, false, "I confirm individual-address write enable to 1.1.67");
    expect(host!.querySelector(".service-control-result")).not.toBeNull();
    root.unmount();
  });
});
