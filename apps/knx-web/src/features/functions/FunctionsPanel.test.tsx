/** Functions panels: explicit load, bound edits, tree re-read, drafts on conflicts (Goal B1b). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "../../bindings/ProjectTree";
import type { FunctionNode } from "../../bindings/FunctionNode";
import type { FunctionsClient, FunctionsRead } from "./client";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "../../uiLanguage";
import { resetSettingsForTests, setSetting } from "../../settingsStore";

const apiMock = vi.hoisted(() => ({ currentProject: vi.fn() }));
vi.mock("../../api", () => ({
  ...apiMock,
  noteRefusal: () => {},
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import { BuildingPartFunctions, GroupAddressFunctions } from "./FunctionsPanel";

const tree = (revision: number, project = 1): ProjectTree => ({
  schema_version: 15, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "srv", project_incarnation: project, snapshot_revision: revision,
  installations: [{ id: 0, name: "Home", topology: [], unassigned: [], group_ranges: [],
    buildings: [{ id: 3, name: "Kitchen", kind: "Room", devices: [], children: [] }, { id: 4, name: "Hall", kind: "Room", devices: [], children: [] }],
    group_addresses: [7, 8].map((id) => ({ id, name: `GA ${id}`, address: `1/0/${id}`, dpt: null, linked_dpts: [], links: [],
      range: null, central: false, unfiltered: false })) as never }],
});
const fn = (id: number, part = 3, overrides: Partial<FunctionNode> = {}): FunctionNode => ({
  id, sourceId: `P-0001-0_F-${id}`, installationId: 0, location: { buildingPartId: part, name: part === 3 ? "Kitchen" : "Hall", kind: "Room" },
  name: `Light ${id}`, functionType: "FT-1", number: null, comment: null, description: null, completion: "Editing",
  members: [{ id: 100 + id, sourceId: `KB-FunctionMember-${100 + id}`, groupAddressId: 7, address: "1/0/7", groupAddressName: "GA 7", name: "", role: "SwitchOnOff" }],
  ...overrides,
});
const view = (revision: number, functions: FunctionNode[], project = 1): FunctionsRead =>
  ({ serverIncarnation: "srv", projectIncarnation: project, snapshotRevision: revision, functions });
const ctx = (revision: number, project = 1) => ({ serverIncarnation: "srv", projectIncarnation: project, snapshotRevision: revision });
const refusal = (status: number, kind: string, message = `server ${kind}`) => Object.assign(new Error(message), { status, kind });

let host: HTMLDivElement | undefined;
let root: Root | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined; host?.remove(); host = undefined;
  vi.clearAllMocks(); resetSettingsForTests(); resetUiLanguageForTests();
});

function client(reads: FunctionsRead[], edit = vi.fn()): FunctionsClient & { read: ReturnType<typeof vi.fn>; edit: ReturnType<typeof vi.fn> } {
  const read = vi.fn();
  for (const value of reads) read.mockResolvedValueOnce(value);
  read.mockResolvedValue(reads[reads.length - 1]);
  return { read, edit } as never;
}
async function render(node: React.ReactNode) {
  if (!host) { host = document.createElement("div"); document.body.append(host); root = createRoot(host); }
  await act(async () => root!.render(node));
}
async function open() {
  const details = host!.querySelector<HTMLDetailsElement>("details.functions-panel")!;
  await act(async () => { details.open = true; details.dispatchEvent(new Event("toggle")); });
}
function set(element: HTMLInputElement | HTMLSelectElement, value: string) {
  const proto = element instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(element, value);
  element.dispatchEvent(new Event(element instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }));
}
const button = (label: string, scope: ParentNode = host!) => [...scope.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === label)!;
const labelled = (label: string, scope: ParentNode = host!) =>
  [...scope.querySelectorAll("label")].find((l) => l.firstChild?.textContent === label)!.querySelector<HTMLInputElement | HTMLSelectElement>("input, select")!;

describe("building-part Functions", () => {
  it.each([
    ["en", 0, "0 group addresses", "Delete this Function? It has no group addresses."],
    ["en", 1, "1 group address", "Delete this Function and its 1 membership? The group address stays."],
    ["de", 1, "1 Gruppenadresse", "Diese Funktion und ihre 1 Mitgliedschaft löschen? Die Gruppenadresse bleibt."],
    ["de", 2, "2 Gruppenadressen", "Diese Funktion und ihre 2 Mitgliedschaften löschen? Die Gruppenadressen bleiben."],
  ] as const)("counts members with real plurals (%s, %i)", async (language, count, heading, confirm) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, language);
    const members = [7, 8].slice(0, count).map((ga) => ({ id: 200 + ga, sourceId: `S-${ga}`, groupAddressId: ga, address: `1/0/${ga}`,
      groupAddressName: `GA ${ga}`, name: "", role: null }));
    const fake = client([view(9, [fn(1, 3, { members })])]);
    await render(<BuildingPartFunctions tree={tree(9)} buildingPartId={3} onApplied={vi.fn()} client={fake} />);
    await open();
    const item = host!.querySelector("[data-function-id]")!;
    expect([...item.querySelectorAll("h4")].map((h) => h.textContent)).toContain(heading);
    await act(async () => button(language === "en" ? "Delete Function" : "Funktion löschen", item).click());
    expect(item.textContent).toContain(confirm);
    expect(fake.edit).not.toHaveBeenCalled();
  });

  it("reads nothing until opened, then shows only this part's Functions with provenance", async () => {
    const fake = client([view(9, [fn(1), fn(2, 4)])]);
    await render(<BuildingPartFunctions tree={tree(9)} buildingPartId={3} onApplied={vi.fn()} client={fake} />);
    expect(fake.read).not.toHaveBeenCalled();
    await open();
    expect(fake.read).toHaveBeenCalledOnce();
    expect(host!.querySelectorAll("[data-function-id]")).toHaveLength(1);
    expect(host!.textContent).toContain("Light 1");
    expect(host!.textContent).not.toContain("Light 2");
    expect(host!.textContent).toContain("P-0001-0_F-1");
    expect(host!.textContent).toContain("KB-FunctionMember-101");
  });

  it("creates with the read's binding and exact details, then publishes the re-read tree", async () => {
    const applied = vi.fn();
    const after = tree(10);
    apiMock.currentProject.mockResolvedValue(after);
    const edit = vi.fn().mockResolvedValue({ ...view(10, [fn(1), fn(5)]), createdId: 5 });
    const fake = client([view(9, [fn(1)])], edit);
    await render(<BuildingPartFunctions tree={tree(9)} buildingPartId={3} onApplied={applied} client={fake} />);
    await open();
    const create = host!.querySelectorAll<HTMLFormElement>("form.functions-details");
    const form = create[create.length - 1];
    await act(async () => set(labelled("Name", form), "Blind"));
    await act(async () => form.requestSubmit());
    expect(edit).toHaveBeenCalledExactlyOnceWith(ctx(9), { kind: "create", location: 3, details:
      { name: "Blind", functionType: null, number: null, comment: null, description: null, completion: "Undefined" } });
    expect(applied).toHaveBeenCalledExactlyOnceWith(after);
    expect(host!.querySelectorAll("[data-function-id]")).toHaveLength(2);
  });

  it.each(["en", "de"] as const)("keeps the input on a stale binding and reloads only on request (%s)", async (language) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, language);
    const applied = vi.fn();
    apiMock.currentProject.mockResolvedValue(tree(11));
    const edit = vi.fn().mockRejectedValue(refusal(409, "functionStale"));
    const fake = client([view(9, [fn(1)]), view(11, [fn(1)])], edit);
    await render(<BuildingPartFunctions tree={tree(9)} buildingPartId={3} onApplied={applied} client={fake} />);
    await open();
    const forms = host!.querySelectorAll<HTMLFormElement>("form.functions-details");
    const form = forms[forms.length - 1];
    const name = labelled(language === "de" ? "Name" : "Name", form) as HTMLInputElement;
    await act(async () => set(name, "Blind"));
    await act(async () => form.requestSubmit());
    expect(host!.querySelector('[role="alert"]')!.textContent).toContain(language === "de" ? "Nichts wurde übernommen" : "Nothing was applied");
    expect(name.value).toBe("Blind");
    expect(applied).not.toHaveBeenCalled();
    await act(async () => button(language === "de" ? "Aktuellen Stand laden" : "Reload current state").click());
    expect(applied).toHaveBeenCalledExactlyOnceWith(tree(11));
    expect(fake.read).toHaveBeenCalledTimes(2);
    expect(edit).toHaveBeenCalledOnce();
    expect(name.value).toBe("Blind");
  });

  it("shows a refusal with the server's own text after a translated headline", async () => {
    const edit = vi.fn().mockRejectedValue(refusal(422, "functionRefused", "InvalidFunction(TooLong)"));
    await render(<BuildingPartFunctions tree={tree(9)} buildingPartId={3} onApplied={vi.fn()} client={client([view(9, [fn(1)])], edit)} />);
    await open();
    await act(async () => button("Remove from Function").click());
    const alert = host!.querySelector('[role="alert"]')!;
    expect(alert.textContent).toContain("The edit was refused; nothing was applied.");
    expect(alert.querySelector(".functions-server-text")!.textContent).toBe("InvalidFunction(TooLong)");
    expect(apiMock.currentProject).not.toHaveBeenCalled();
  });

  it("rebases an untouched-underneath details draft onto a newer read", async () => {
    apiMock.currentProject.mockResolvedValue(tree(12));
    const edit = vi.fn().mockResolvedValue({ ...view(12, [fn(1, 3, { name: "Ceiling" })]), createdId: null });
    const fake = client([view(9, [fn(1)]), view(10, [fn(1)])], edit);
    await render(<BuildingPartFunctions tree={tree(9)} buildingPartId={3} onApplied={vi.fn()} client={fake} />);
    await open();
    await act(async () => button("Edit details").click());
    const form = host!.querySelector<HTMLFormElement>("[data-function-id] form.functions-details")!;
    await act(async () => set(labelled("Name", form), "Ceiling"));
    // An unrelated change elsewhere moves the project on; this Function is untouched.
    await render(<BuildingPartFunctions tree={tree(10)} buildingPartId={3} onApplied={vi.fn()} client={fake} />);
    await act(async () => form.requestSubmit());
    expect(edit).toHaveBeenCalledExactlyOnceWith(ctx(10), { kind: "update", id: 1, details:
      { name: "Ceiling", functionType: "FT-1", number: null, comment: null, description: null, completion: "Editing" } });
  });

  it("holds a draft whose Function changed underneath until the user decides", async () => {
    apiMock.currentProject.mockResolvedValue(tree(12));
    const edit = vi.fn().mockResolvedValue({ ...view(12, [fn(1)]), createdId: null });
    const fake = client([view(9, [fn(1)]), view(10, [fn(1, 3, { name: "Renamed elsewhere", completion: "Tested" })])], edit);
    await render(<BuildingPartFunctions tree={tree(9)} buildingPartId={3} onApplied={vi.fn()} client={fake} />);
    await open();
    await act(async () => button("Edit details").click());
    const form = host!.querySelector<HTMLFormElement>("[data-function-id] form.functions-details")!;
    await act(async () => set(labelled("Name", form), "Mine"));
    await render(<BuildingPartFunctions tree={tree(10)} buildingPartId={3} onApplied={vi.fn()} client={fake} />);
    expect(form.textContent).toContain("now: Renamed elsewhere, Tested");
    expect(button("Apply details", form).disabled).toBe(true);
    expect(button("Apply details", form).classList.contains("primary-action")).toBe(true);
    await act(async () => form.requestSubmit());
    expect(edit).not.toHaveBeenCalled();
    // Throwing the draft away loses the user's input, so it looks destructive.
    expect(button("Start again from the current details", form).classList.contains("danger-action")).toBe(true);
    expect(button("Keep my input", form).classList.contains("danger-action")).toBe(false);
    await act(async () => button("Keep my input", form).click());
    // The forced apply overwrites the newer details, so it looks destructive, not routine.
    const anyway = button("Apply my details anyway", form);
    expect(anyway.classList.contains("danger-action")).toBe(true);
    expect(anyway.classList.contains("primary-action")).toBe(false);
    await act(async () => anyway.click());
    expect(edit).toHaveBeenCalledExactlyOnceWith(ctx(10), expect.objectContaining({ kind: "update", id: 1,
      details: expect.objectContaining({ name: "Mine", completion: "Editing" }) }));
  });

  it("deletes only after confirmation and moves to another part of the same installation", async () => {
    apiMock.currentProject.mockResolvedValue(tree(10));
    const edit = vi.fn().mockResolvedValue({ ...view(10, [fn(1)]), createdId: null });
    // A second installation's room must never be offered as a move target.
    const two = tree(9);
    two.installations.push({ ...two.installations[0], id: 1, name: "Annex", group_addresses: [],
      buildings: [{ id: 9, name: "Garage", kind: "Room", devices: [], children: [] }] } as never);
    await render(<BuildingPartFunctions tree={two} buildingPartId={3} onApplied={vi.fn()} client={client([view(9, [fn(1)])], edit)} />);
    await open();
    await act(async () => button("Delete Function").click());
    expect(edit).not.toHaveBeenCalled();
    expect(host!.textContent).toContain("its 1 membership?");
    expect(button("Delete").classList.contains("danger-action")).toBe(true);
    expect(button("Cancel").classList.contains("primary-action")).toBe(false);
    await act(async () => button("Delete").click());
    expect(edit).toHaveBeenLastCalledWith(ctx(9), { kind: "delete", id: 1 });
    const select = labelled("Move to building part") as HTMLSelectElement;
    expect([...select.options].map((o) => o.textContent)).toEqual(["Choose a building part…", "Hall"]);
    await act(async () => set(select, "4"));
    await act(async () => button("Move Function").click());
    expect(edit).toHaveBeenLastCalledWith(ctx(10), { kind: "move", id: 1, location: 4 });
  });

  it("drops the read of a project that is no longer open", async () => {
    const fake = client([]);
    let arrive!: (value: FunctionsRead) => void;
    fake.read.mockReturnValueOnce(Promise.resolve(view(9, [fn(1)]))).mockReturnValueOnce(new Promise((resolve) => { arrive = resolve; }));
    await render(<BuildingPartFunctions tree={tree(9)} buildingPartId={3} onApplied={vi.fn()} client={fake} />);
    await open();
    expect(host!.textContent).toContain("Light 1");
    // While the new project's read is still on its way, nothing of the old one may be shown or edited.
    await render(<BuildingPartFunctions tree={tree(1, 2)} buildingPartId={3} onApplied={vi.fn()} client={fake} />);
    expect(fake.read).toHaveBeenCalledTimes(2);
    expect(host!.textContent).not.toContain("Light 1");
    expect(button("Create Function")).toBeUndefined();
    expect(host!.textContent).toContain("Loading Functions");
    await act(async () => arrive(view(1, [], 2)));
    expect(host!.textContent).toContain("No Function is located here yet.");
  });
});

describe("group-address Functions", () => {
  it("lists every membership and removes one with the read's binding", async () => {
    apiMock.currentProject.mockResolvedValue(tree(10));
    const edit = vi.fn().mockResolvedValue({ ...view(10, []), createdId: null });
    const other = fn(2, 4, { members: [{ id: 300, sourceId: "S", groupAddressId: 8, address: "1/0/8", groupAddressName: "GA 8", name: "", role: null }] });
    await render(<GroupAddressFunctions tree={tree(9)} groupAddressId={7} onApplied={vi.fn()} client={client([view(9, [fn(1), other])], edit)} />);
    await open();
    expect(host!.querySelectorAll("[data-member-id]")).toHaveLength(1);
    expect(host!.textContent).toContain("Remove it from the Function first");
    await act(async () => host!.querySelector<HTMLButtonElement>('[aria-label="Remove 1/0/7 from Light 1"]')!.click());
    expect(edit).toHaveBeenCalledExactlyOnceWith(ctx(9), { kind: "removeMember", functionId: 1, memberId: 101 });
  });
});
