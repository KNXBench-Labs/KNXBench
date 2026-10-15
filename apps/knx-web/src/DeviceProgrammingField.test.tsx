/** Marking a device programmed by hand: state shown, bound call, undo-ready tree. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { PendingProgramming } from "./bindings/PendingProgramming";
import { resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({ markDeviceProgramming: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock, errorMessage: (error: unknown) => error instanceof Error ? error.message : String(error) }));
import DeviceProgrammingField from "./DeviceProgrammingField";

function tree(state: PendingProgramming | undefined, revision = 4): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "s", project_incarnation: 2, snapshot_revision: revision,
    installations: [{ id: 1, name: "House", topology: [], buildings: [], group_addresses: [], group_ranges: [],
      unassigned: [{ id: 7, name: "Dimmer", address: "1.1.7", description: null, com_object_count: 0,
        ...(state ? { pending_programming: state } : {}) }] }] };
}
const some: PendingProgramming = { address: false, application: false, parameters: true, group_addresses: false };
const none: PendingProgramming = { address: false, application: false, parameters: false, group_addresses: false };
const every: PendingProgramming = { address: true, application: true, parameters: true, group_addresses: true };

let host: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
const applied = vi.fn();
beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  apiMock.markDeviceProgramming.mockReset(); applied.mockReset();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); resetUiLanguageForTests(); vi.unstubAllGlobals(); });

async function render(project: ProjectTree) {
  await act(async () => root.render(<DeviceProgrammingField deviceId={7} tree={project} onApplied={applied} />));
}
const button = (text: string) => [...host.querySelectorAll("button")].find((b) => b.textContent === text)!;

it("shows each part's state and marks by hand against the shown tree", async () => {
  const shown = tree(some);
  const answer = tree(none, 5);
  apiMock.markDeviceProgramming.mockResolvedValue(answer);
  await render(shown);
  const rows = [...host.querySelectorAll(".device-programming-parts > div")];
  expect(rows.map((row) => [row.querySelector("dt")!.textContent, row.querySelector("dd")!.textContent])).toEqual([
    ["Address", "Programmed"], ["Application", "Programmed"], ["Parameters", "Not programmed"], ["Group addresses", "Programmed"],
  ]);
  expect(button("Mark as programmed").disabled).toBe(false);
  expect(button("Mark as not programmed").disabled).toBe(false);
  await act(async () => button("Mark as programmed").click());
  expect(apiMock.markDeviceProgramming).toHaveBeenCalledExactlyOnceWith(7, true, shown);
  expect(applied).toHaveBeenCalledExactlyOnceWith(answer);
});

it("offers only the mark that would change something", async () => {
  await render(tree(none));
  expect(button("Mark as programmed").disabled).toBe(true);
  expect(button("Mark as not programmed").disabled).toBe(false);
  await render(tree(every));
  expect(button("Mark as programmed").disabled).toBe(false);
  expect(button("Mark as not programmed").disabled).toBe(true);
});

it("keeps the state and says why when the server refuses", async () => {
  apiMock.markDeviceProgramming.mockRejectedValue(new Error("The project changed since this device was shown"));
  await render(tree(some));
  await act(async () => button("Mark as not programmed").click());
  expect(applied).not.toHaveBeenCalled();
  expect(host.querySelector('[role="alert"]')?.textContent).toContain("project changed");
  expect(button("Mark as not programmed").disabled).toBe(false);
});

it("does not guess a state the server did not report", async () => {
  await render(tree(undefined));
  expect(host.textContent).toContain("does not report the programming state");
  expect(host.querySelectorAll("button")).toHaveLength(0);
});
