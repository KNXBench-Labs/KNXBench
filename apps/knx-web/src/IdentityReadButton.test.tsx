/** Tests bound manual identity acquisition and its separate report handoff. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
const support = vi.hoisted(() => ({ previewSupport: vi.fn(), diagnoseSupport: vi.fn() }));
vi.mock("./supportApi", () => support);
import IdentityReadButton from "./IdentityReadButton";
import { ReportProblemProvider } from "./ReportProblemEntry";
import type { SupportPreview } from "./supportApi";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const PREVIEW: SupportPreview = { token: "a".repeat(64), digest: "b".repeat(64), report: {}, summary: "bounded", localAddress: "1.1.67", mask: null, profile: "descriptor-only-v1", reads: ["DeviceDescriptorRead type 0"], deadlineSeconds: 30 };
const TREE: ProjectTree = { schema_version: 1, can_undo: false, can_redo: false, is_modified: false, errors: 0, warnings: 0,
  group_address_style: "ThreeLevel", installations: [], server_incarnation: "fixture", project_incarnation: 1, snapshot_revision: 1 };
let host: HTMLDivElement; let root: Root;
const opened = vi.fn();
beforeEach(() => {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  opened.mockReset(); Object.values(support).forEach((m) => m.mockReset());
  support.previewSupport.mockResolvedValue(PREVIEW); support.diagnoseSupport.mockResolvedValue(PREVIEW);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });
async function render(tree: ProjectTree | null = TREE) {
  await act(async () => root.render(<ReportProblemProvider request={null} onOpen={opened} onClose={() => {}} tree={tree} notifications={[]}>
    <IdentityReadButton project={tree} />
  </ReportProblemProvider>));
}
const find = <T extends HTMLElement = HTMLElement>(selector: string) => document.querySelector<T>(selector)!;
async function click(selector: string) { expect(find(selector)).not.toBeNull(); await act(async () => find(selector).click()); }
async function input(selector: string, value: string) {
  await act(async () => {
    const field = find<HTMLInputElement>(selector);
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(field, value);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
async function open(tree: ProjectTree | null = TREE) {
  await render(tree); await click("[data-identity-read-open]"); await input("[data-support-local-address]", "1.1.67");
}
async function consent() {
  await click("[data-support-preview]"); await input("[data-support-gateway]", "192.0.2.10:3671"); await click("[data-support-live-consent]");
}
for (const pending of ["preview", "read"] as const) for (const change of ["revision", "project", "server", "close"] as const) {
  it(`drops late ${pending} after ${change} without a wizard handoff`, async () => {
    let release!: (value: SupportPreview) => void;
    const promise = new Promise<SupportPreview>((resolve) => { release = resolve; });
    await open();
    if (pending === "preview") { support.previewSupport.mockReturnValueOnce(promise); await click("[data-support-preview]"); }
    else { await consent(); support.diagnoseSupport.mockReturnValueOnce(promise); await click("[data-support-read]"); }
    if (change === "close") await click("[data-support-close]");
    else await render({ ...TREE, ...(change === "revision" ? { snapshot_revision: 2 } : change === "project" ? { project_incarnation: 2 } : { server_incarnation: "restarted" }) });
    expect(find("[data-identity-read-dialog]")).toBeNull();
    await act(async () => release(PREVIEW));
    expect(find("[data-identity-read-dialog]")).toBeNull(); expect(opened).not.toHaveBeenCalled();
    expect(support.diagnoseSupport).toHaveBeenCalledTimes(pending === "read" ? 1 : 0);
  });
}
it("equal refresh retains acquisition consent but changing the gateway withdraws it", async () => {
  await open(); await consent(); await render({ ...TREE });
  expect(find<HTMLInputElement>("[data-support-live-consent]").checked).toBe(true);
  await input("[data-support-gateway]", "192.0.2.11:3671");
  expect(find<HTMLInputElement>("[data-support-live-consent]").checked).toBe(false);
  expect(find<HTMLButtonElement>("[data-support-read]").disabled).toBe(true);
  expect(support.diagnoseSupport).not.toHaveBeenCalled();
});
it("refuses an offline response for another address before live read is available", async () => {
  support.previewSupport.mockResolvedValue({ ...PREVIEW, localAddress: "1.1.68" });
  await open(null); await click("[data-support-preview]");
  expect(find("[data-support-live-consent]")).toBeNull(); expect(opened).not.toHaveBeenCalled();
  expect(support.diagnoseSupport).not.toHaveBeenCalled();
});
it("manual acquisition outside an open project never invents a project device", async () => {
  await open(); await consent(); await click("[data-support-read]");
  await click("[data-report-problem-open=support-read]");
  expect(opened).toHaveBeenCalledExactlyOnceWith({ problemType: "deviceReading", operation: { relatedKey: "supportToken", relatedValue: PREVIEW.token } });
  expect(support.previewSupport.mock.calls[0].slice(1, 5)).toEqual([null, undefined, undefined, "1.1.67"]);
  expect(support.diagnoseSupport).toHaveBeenCalledTimes(1);
});
