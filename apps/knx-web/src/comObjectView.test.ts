/** Communication-object filtering and stable sorting contracts. */
import { expect, it } from "vitest";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { ProjectTreeNode } from "./bindings/ProjectTreeNode";
import type { ProductFolderCaption } from "./bindings/ProductFolderCaption";
import { deriveComObjectView, dptOptions, DEFAULT_FILTERS, projectFolderIds, projectFolderNodeLabel, projectFolderProductCaption, projectFolderRefusal, type ComSortColumn } from "./comObjectView";
export function object(id: number, overrides: Partial<ComObjectNode> = {}): ComObjectNode {
  return { id, number: id, name: `Object ${id}`, function_text: null, description: null,
    description_layer: null, dpt: null, dpt_layer: null, program_dpt: null, dpt_text: null,
    is_active: true, activation: "NotEvaluated", channel: null, read: false, write: false,
    transmit: false, update: false, communication: true, read_on_init: false, links: [], ...overrides };
}
const channel = (key: string, order: number) => ({ key, order, kind: "Channel" as const, text: "Same", name: key, number: "07" });
const view = (objects: ComObjectNode[], filters = DEFAULT_FILTERS, column: ComSortColumn | null = null, descending = false, grouped = false) =>
  deriveComObjectView(objects, filters, { column, descending }, "en", grouped);
it("keeps opaque channels and source order without mutating input", () => {
  const objects = [object(3, { activation: "Active", channel: channel("b", 2) }),
    object(10, { activation: "Active", channel: channel("a", 1) }), object(2, { activation: "Active", channel: channel("a", 1) }), object(1)];
  const before = JSON.stringify(objects);
  expect(view(objects, DEFAULT_FILTERS, null, false, true).groups.map(g => g.objects.map(o => o.id))).toEqual([[10, 2], [3], [1]]);
  expect(view(objects).objects.map(o => o.id)).toEqual([3, 10, 2, 1]);
  expect(view(objects, DEFAULT_FILTERS, "number", false, true).groups.map(g => g.objects.map(o => o.id))).toEqual([[2, 10], [3], [1]]);
  expect(JSON.stringify(objects)).toBe(before);
});
it("AND-combines filters with all-address slash/dot search and effective DPT", () => {
  const objects = [object(1, { activation: "Active", program_dpt: "DPST-9-1", description: "Boiler", links: [
    { ga_id: 1, address: "1/2/10", name: "Heat", direction: "Send" }, { ga_id: 2, address: "2/3/4", name: "Kitchen", direction: "Receive" }] }), object(2)];
  const filters = { ...DEFAULT_FILTERS, query: "2.3.4", status: "Active", link: "linked" as const, dpt: "family:9" };
  expect(view(objects, filters).objects.map(o => o.id)).toEqual([1]);
  expect(view(objects, { ...filters, status: "Inactive" }).objects).toEqual([]);
  expect(view(objects, { ...DEFAULT_FILTERS, query: "boiler" }).descriptionMatches).toEqual(new Set([1]));
  expect(view(objects, { ...DEFAULT_FILTERS, query: "kitchen" }).objects.map(o => o.id)).toEqual([1]);
});
it("does not classify dangling links as unlinked or unknown states as inactive", () => {
  const future = object(1, { activation: "Future" as ComObjectNode["activation"], links: [{ ga_id: 1, address: null, name: null, direction: "Send" }] });
  expect(view([future], { ...DEFAULT_FILTERS, link: "linked", status: "unknown" }).objects).toEqual([future]);
  expect(view([future], { ...DEFAULT_FILTERS, status: "Inactive" }).objects).toEqual([]);
});
it("compares number, DPT and minimum GA numerically; missing stays last both ways", () => {
  const objects = [object(10, { dpt: "DPST-9-10", links: [{ ga_id: 1, address: "3/1/2", name: null, direction: "Send" }, { ga_id: 2, address: "1/1/2", name: null, direction: "Receive" }] }),
    object(2, { dpt: "DPST-9-2", links: [{ ga_id: 3, address: "1/1/10", name: null, direction: "Send" }] }), object(99)];
  for (const column of ["number", "dpt"] as const) expect(view(objects, DEFAULT_FILTERS, column).objects.map(o => o.id)).toEqual([2, 10, 99]);
  expect(view(objects, DEFAULT_FILTERS, "dpt", true).objects.map(o => o.id)).toEqual([10, 2, 99]);
  expect(view(objects, DEFAULT_FILTERS, "addresses").objects.map(o => o.id)).toEqual([10, 2, 99]);
  expect(view(objects, DEFAULT_FILTERS, "addresses", true).objects.map(o => o.id)).toEqual([2, 10, 99]);
});
it("keeps duplicate numbers and stable ties in descending order", () => {
  const objects = [object(8, { number: 1, name: "Object 2" }), object(3, { number: 1, name: "Object 10" }), object(9, { number: 1, name: "Object 2" })];
  expect(view(objects, DEFAULT_FILTERS, "number", true).objects.map(o => o.id)).toEqual([8, 3, 9]);
  expect(view(objects, DEFAULT_FILTERS, "name").objects.map(o => o.id)).toEqual([8, 9, 3]);
});
it("offers family, exact, missing and verbatim unrecognized DPT filters", () => {
  const objects = [object(1, { dpt: "DPST-9-1" }), object(2, { program_dpt: "DPT-1" }), object(3, { dpt: "future-raw" }), object(4)];
  expect(dptOptions(objects)).toEqual(["family:1", "family:9", "exact:DPT-1", "exact:DPST-9-1", "exact:future-raw", "missing"]);
  expect(view(objects, { ...DEFAULT_FILTERS, dpt: "missing" }).objects.map(o => o.id)).toEqual([4]);
  expect(view(objects, { ...DEFAULT_FILTERS, dpt: "exact:future-raw" }).objects.map(o => o.id)).toEqual([3]);
});
it("sorts function, status and channel labels independently of translated status labels", () => {
  const objects = [object(1, { activation: "NotEvaluated", function_text: "Fn 10", channel: channel("bad", 0) }),
    object(2, { activation: "Inactive", function_text: "Fn 2" }), object(3, { activation: "Active", channel: channel("a", 0) })];
  expect(view(objects, DEFAULT_FILTERS, "function").objects.map(o => o.id)).toEqual([2, 1, 3]);
  expect(view(objects, DEFAULT_FILTERS, "status").objects.map(o => o.id)).toEqual([3, 2, 1]);
  expect(view(objects, DEFAULT_FILTERS, "channel").objects.map(o => o.id)).toEqual([3, 1, 2]);
});

it("searches localized catalogue names and dotted DPT numbers without rewriting source", () => {
  const objects = [object(1, { program_dpt: "DPST-17-1" }), object(2, { dpt: "DPST-9-1" })];
  const before = JSON.stringify(objects);
  for (const [language, query, expected] of [["de", "szenennummer", 1], ["en", "scene number", 1], ["de", "9.001", 2]] as const) {
    expect(deriveComObjectView(objects, { ...DEFAULT_FILTERS, query }, { column: null, descending: false }, language, false)
      .objects.map(o => o.id)).toEqual([expected]);
  }
  expect(JSON.stringify(objects)).toBe(before);
});
it("WP-06a: shows only assigned saved folders, names refusals and searches declared folder ids", () => {
  const assigned = object(1, { project_folder: { status: "Assigned", key: "Channel:4:CH-1/Folder:4:PB-3/Folder:4:PB-4",
    path: [{ kind: "Channel", ref_id: "CH-1" }, { kind: "Folder", ref_id: "PB-3" }, { kind: "Folder", ref_id: "PB-4" }] } });
  const refused = (id: number, status: string) => object(id, { project_folder: {
    status: status as NonNullable<ComObjectNode["project_folder"]>["status"], key: null, path: [] } });
  expect(projectFolderIds(assigned)).toEqual(["PB-3", "PB-4"]);
  expect(projectFolderRefusal(assigned)).toBeNull();
  for (const status of ["Ambiguous", "UnidentifiedNode", "TreeUnresolved", "ModuleObject", "Inactive"]) {
    expect(projectFolderIds(refused(2, status))).toBeNull();
    expect(projectFolderRefusal(refused(2, status))).toBe(status);
  }
  // No folder, not in the tree, not read, or a future status: nothing to show or refuse.
  for (const o of [refused(3, "NoFolder"), refused(4, "NotInSavedTree"), object(5), refused(6, "Future")]) {
    expect(projectFolderIds(o)).toBeNull();
    expect(projectFolderRefusal(o)).toBeNull();
  }
  const objects = [assigned, refused(2, "Ambiguous"), object(3)];
  expect(view(objects, { ...DEFAULT_FILTERS, query: "pb-4" }).objects.map(o => o.id)).toEqual([1]);
  // The channel id of the saved path is not a folder and is not searchable as one.
  expect(view(objects, { ...DEFAULT_FILTERS, query: "CH-1" }).objects).toEqual([]);
});

// WP-06 folder grouping: groups by the opaque saved-folder key, never by labels.
type Folder = NonNullable<ComObjectNode["project_folder"]>;
const saved = (key: string | null, path: [string, string][], status = "Assigned"): Folder =>
  ({ status: status as Folder["status"], key, path: path.map(([kind, ref_id]) => ({ kind, ref_id })) });
const pb3 = (ch: string) => saved(`Channel:4:${ch}/Folder:4:PB-3`, [["Channel", ch], ["Folder", "PB-3"]]);
const byFolder = (objects: ComObjectNode[], filters = DEFAULT_FILTERS, column: ComSortColumn | null = null, descending = false) =>
  deriveComObjectView(objects, filters, { column, descending }, "en", true, "folder");
it("folder grouping joins exact opaque keys and keeps equal visible ids under different keys apart", () => {
  const objects = [object(1, { project_folder: pb3("CH-1") }), object(2, { project_folder: pb3("CH-2") }),
    // Same key, different declared path: the key alone decides membership.
    object(3, { project_folder: { ...pb3("CH-1"), path: [{ kind: "Folder", ref_id: "PB-3" }] } })];
  const groups = byFolder(objects).groups;
  expect(groups.map(g => g.key)).toEqual(["folder:Channel:4:CH-1/Folder:4:PB-3", "folder:Channel:4:CH-2/Folder:4:PB-3"]);
  expect(groups.map(g => g.objects.map(o => o.id))).toEqual([[1, 3], [2]]);
  expect(groups.map(g => g.folder?.key)).toEqual(["Channel:4:CH-1/Folder:4:PB-3", "Channel:4:CH-2/Folder:4:PB-3"]);
  expect(groups.map(g => g.channel)).toEqual([null, null]);
});
it("folder grouping puts every unadmitted object into one last fallback group", () => {
  const fallback = [object(1), object(2, { project_folder: saved(null, [], "NoFolder") }),
    object(3, { project_folder: saved(null, [], "NotInSavedTree") }),
    ...["Ambiguous", "UnidentifiedNode", "TreeUnresolved", "ModuleObject", "Inactive", "Future"]
      .map((status, i) => object(4 + i, { project_folder: saved(null, [], status) })),
    object(10, { project_folder: saved(null, [["Channel", "CH-1"], ["Folder", "PB-3"]]) }),
    object(11, { project_folder: saved("", [["Channel", "CH-1"], ["Folder", "PB-3"]]) }),
    object(12, { project_folder: saved("Channel:4:CH-9", [["Channel", "CH-9"]]) }),
    // A key and plausible path never override an explicit refusal status.
    object(14, { project_folder: saved("refused-key", [["Folder", "PB-3"]], "Ambiguous") })];
  const objects = [...fallback, object(13, { project_folder: pb3("CH-1") })];
  const groups = byFolder(objects).groups;
  expect(groups.map(g => g.key)).toEqual(["folder:Channel:4:CH-1/Folder:4:PB-3", "folder-none"]);
  expect(groups[1].objects.map(o => o.id)).toEqual(fallback.map(o => o.id));
  expect(groups[1].folder).toBeNull();
  expect(groups[1].total).toBe(fallback.length);
  // Without fallback members there is no fallback group.
  expect(byFolder([object(13, { project_folder: pb3("CH-1") })]).groups.map(g => g.key)).toEqual(["folder:Channel:4:CH-1/Folder:4:PB-3"]);
});
it("folder groups follow first unfiltered encounter despite opposing channel order, filters and sorting", () => {
  const a = saved("Folder:1:A", [["Folder", "A"]]), b = saved("Folder:1:B", [["Folder", "B"]]);
  const objects = [object(1, { activation: "Active", channel: channel("z", 9), project_folder: b }),
    object(2, { activation: "Active", channel: channel("y", 0), project_folder: a, function_text: "keep" }),
    object(3, { project_folder: b, function_text: "keep" })];
  // The channel view orders by program order; the folder view ignores it.
  expect(view(objects, DEFAULT_FILTERS, null, false, true).groups[0].key).toBe("channel:y");
  expect(byFolder(objects).groups.map(g => g.key)).toEqual(["folder:Folder:1:B", "folder:Folder:1:A"]);
  expect(byFolder(objects).objects.map(o => o.id)).toEqual([1, 3, 2]);
  // Filtering out B's first member does not reorder the groups.
  const filtered = byFolder(objects, { ...DEFAULT_FILTERS, query: "keep" });
  expect(filtered.groups.map(g => [g.key, g.objects.map(o => o.id)])).toEqual([["folder:Folder:1:B", [3]], ["folder:Folder:1:A", [2]]]);
  expect(byFolder(objects, DEFAULT_FILTERS, "number", true).groups.map(g => g.objects.map(o => o.id))).toEqual([[3, 1], [2]]);
});
it("folder grouping counts totals unfiltered, keeps zero-match groups and leaves input untouched", () => {
  const objects = [object(1, { project_folder: pb3("CH-1") }), object(2, { project_folder: pb3("CH-1") }),
    object(3, { project_folder: pb3("CH-2") }), object(4)];
  const before = JSON.stringify(objects);
  const result = byFolder(objects, { ...DEFAULT_FILTERS, query: "Object 2" });
  expect(result.groups.map(g => [g.key, g.objects.length, g.total])).toEqual([
    ["folder:Channel:4:CH-1/Folder:4:PB-3", 1, 2], ["folder:Channel:4:CH-2/Folder:4:PB-3", 0, 1], ["folder-none", 0, 1]]);
  expect(result.objects.map(o => o.id)).toEqual([2]);
  expect(JSON.stringify(objects)).toBe(before);
});
it("an explicit channel grouping equals the default grouping", () => {
  const objects = [object(1, { activation: "Active", channel: channel("b", 2), project_folder: pb3("CH-1") }), object(2)];
  const explicit = deriveComObjectView(objects, DEFAULT_FILTERS, { column: null, descending: false }, "en", true, "channel");
  expect(explicit.groups.map(g => g.key)).toEqual(["channel:b", "unassigned"]);
  expect(explicit).toEqual(view(objects, DEFAULT_FILTERS, null, false, true));
});

it("saved schema20 text accompanies IDs verbatim and blank text only falls back for display", () => {
  for (const text of [undefined, "", " \t\n"]) expect(projectFolderNodeLabel({ kind: "Folder", ref_id: "PB-3", text })).toBe("PB-3");
  const node = { kind: "Folder", ref_id: "PB-3", text: "  <b>{{ChNo}}</b> & {{0}}  " };
  expect(projectFolderNodeLabel(node)).toBe("  <b>{{ChNo}}</b> & {{0}}   (PB-3)");
  expect(node.text).toBe("  <b>{{ChNo}}</b> & {{0}}  ");
});
it("searches only admitted Folder text and IDs, not refused paths or saved Channel text", () => {
  const path = [{ kind: "Channel", ref_id: "CH-77", text: "Channel secret" },
    { kind: "Folder", ref_id: "PB-3", text: "<b>{{ChNo}}</b> & Kitchen" }];
  const objects = [object(1, { project_folder: { status: "Assigned", key: "opaque", path } }),
    ...["Ambiguous", "UnidentifiedNode", "TreeUnresolved", "ModuleObject", "Inactive", "NoFolder", "NotInSavedTree"]
      .map((status, i) => object(i + 2, { project_folder: { status: status as Folder["status"], key: "refused-key",
        path: [{ kind: "Folder", ref_id: "REFUSED-ID", text: "Refused secret" }] } })),
    object(9, { project_folder: { status: "Assigned", key: "channel-only", path: [path[0]] } })];
  const before = JSON.stringify(objects);
  for (const query of ["kitchen", "PB-3", "{{ChNo}}", "<b>"]) {
    expect(byFolder(objects, { ...DEFAULT_FILTERS, query }).objects.map(o => o.id)).toEqual([1]);
  }
  for (const query of ["Channel secret", "CH-77", "Refused secret", "REFUSED-ID"]) {
    expect(byFolder(objects, { ...DEFAULT_FILTERS, query }).objects).toEqual([]);
  }
  expect(JSON.stringify(objects)).toBe(before);
});
it("caption equality and caption changes never affect opaque folder membership, totals or order", () => {
  const a = pb3("CH-1"), b = pb3("CH-2");
  const caption = (folder: Folder, text: string): Folder => ({ ...folder, path: folder.path.map(n => ({ ...n, text })) });
  const objects = [object(1, { project_folder: caption(a, "Same") }),
    object(2, { project_folder: caption(b, "Same") }), object(3, { project_folder: caption(a, "Different") }), object(4)];
  const expected = [
    ["folder:Channel:4:CH-1/Folder:4:PB-3", [1, 3], 2],
    ["folder:Channel:4:CH-2/Folder:4:PB-3", [2], 1], ["folder-none", [4], 1],
  ];
  const membership = (items: ComObjectNode[]) => byFolder(items).groups.map(g => [g.key, g.objects.map(o => o.id), g.total]);
  expect(membership(objects)).toEqual(expected);
  expect(membership(objects.map(o => ({ ...o, project_folder: o.project_folder ? caption(o.project_folder, "Changed {{0}}") : undefined })))).toEqual(expected);
  expect(byFolder(objects, { ...DEFAULT_FILTERS, query: "Different" }).groups.map(g => [g.key, g.objects.map(o => o.id), g.total])).toEqual([
    ["folder:Channel:4:CH-1/Folder:4:PB-3", [3], 2],    ["folder:Channel:4:CH-2/Folder:4:PB-3", [], 1], ["folder-none", [], 1],
  ]);
});

const productCaption = (text = "Literal product label"): ProductFolderCaption => ({
  source: "ProductDeclaration", text, application_program_id: "M-FA01_A-0100-01-0000",
  parameter_block_id: "M-FA01_A-0100-01-0000_PB-3", source_sha256: "ab".repeat(32),
});
// Model an untyped JSON boundary without asserting an invalid value is a valid DTO.
function unexpectedCaption(value: unknown): ProjectTreeNode {
  const node: ProjectTreeNode = { kind: "Folder", ref_id: "PB-3" };
  Object.defineProperty(node, "product_caption", { value, enumerable: true });
  return node;
}
it("saved text wins over product metadata, including explicit empty and whitespace values", () => {
  const caption = productCaption();
  const nodes: ProjectTreeNode[] = [
    { kind: "Folder", ref_id: "PB-3", text: "  Saved literal  ", product_caption: caption },
    { kind: "Folder", ref_id: "PB-3", text: "", product_caption: caption },
    { kind: "Folder", ref_id: "PB-3", text: " \t\n", product_caption: caption },
    { kind: "Folder", ref_id: "PB-3", product_caption: caption },
    { kind: "Folder", ref_id: "PB-3" },
  ];
  const before = JSON.stringify(nodes);
  expect(nodes.map(projectFolderNodeLabel)).toEqual(["  Saved literal   (PB-3)", "PB-3", "PB-3", "Literal product label (PB-3)", "PB-3"]);
  expect(nodes.map(projectFolderProductCaption)).toEqual([null, null, null, caption, null]);
  expect(JSON.stringify(nodes)).toBe(before);
});
it("only a complete known product declaration on a Folder can supply a label", () => {
  const caption = productCaption();
  const invalid: unknown[] = [null, "Literal product label", [], {},
    { ...caption, source: "FutureDeclaration" }, { ...caption, source: "productdeclaration" },
    { ...caption, text: " \t" }, { ...caption, text: 7 },
    { source: "ProductDeclaration", text: "Missing provenance" },
    { ...caption, application_program_id: "" }, { ...caption, parameter_block_id: null },
    { ...caption, source_sha256: "not-a-sha256" }, { ...caption, source_sha256: `${caption.source_sha256}\n` }];
  for (const value of invalid) {
    const node = unexpectedCaption(value);
    expect(projectFolderProductCaption(node)).toBeNull();
    expect(projectFolderNodeLabel(node)).toBe("PB-3");
  }
  for (const kind of ["Channel", "FutureNode"]) {
    const node: ProjectTreeNode = { kind, ref_id: "CH-1", product_caption: caption };
    expect(projectFolderProductCaption(node)).toBeNull();
    expect(projectFolderNodeLabel(node)).toBe("CH-1");
  }
});
it("search includes only the selected admitted product literal and Folder ID, not its provenance", () => {
  const caption = productCaption('<img src=x onerror="alert(1)"> & Productonly');
  const folder = (node: ProjectTreeNode): Folder => ({ status: "Assigned", key: "opaque", path: [node] });
  const objects = [object(1, { project_folder: folder({ kind: "Folder", ref_id: "PB-3", product_caption: caption }) }),
    object(2, { project_folder: folder({ kind: "Folder", ref_id: "PB-4", text: "Savedonly", product_caption: caption }) }),
    object(3, { project_folder: folder({ kind: "Folder", ref_id: "PB-5", text: "", product_caption: caption }) }),
    object(4, { project_folder: folder({ kind: "Folder", ref_id: "PB-6", text: " \t", product_caption: caption }) }),
    object(5, { project_folder: folder({ kind: "Channel", ref_id: "CH-99", product_caption: caption }) }),
    object(6, { project_folder: folder(unexpectedCaption({ ...caption, source: "FutureDeclaration" })) }),
    ...(["Ambiguous", "UnidentifiedNode", "TreeUnresolved", "ModuleObject", "Inactive", "NoFolder", "NotInSavedTree"] as const)
      .map((status, index) => object(index + 7, { project_folder: { status, key: "refused-key",
        path: [{ kind: "Folder", ref_id: "REFUSED-ID", product_caption: caption }] } }))];
  const before = JSON.stringify(objects);
  for (const query of ["productonly", "<img", "onerror", "PB-3"]) {
    expect(view(objects, { ...DEFAULT_FILTERS, query }).objects.map(o => o.id)).toEqual(query === "PB-3" ? [1, 6] : [1]);
  }
  expect(view(objects, { ...DEFAULT_FILTERS, query: "savedonly" }).objects.map(o => o.id)).toEqual([2]);
  for (const query of [caption.application_program_id, caption.parameter_block_id, caption.source_sha256,
    "ProductDeclaration", "FutureDeclaration", "REFUSED-ID", "CH-99"]) {
    expect(view(objects, { ...DEFAULT_FILTERS, query }).objects).toEqual([]);
  }
  for (const id of ["PB-4", "PB-5", "PB-6"]) expect(view(objects, { ...DEFAULT_FILTERS, query: id }).objects).toHaveLength(1);
  expect(JSON.stringify(objects)).toBe(before);
});
it("equal product captions and changing source metadata do not change opaque groups or order", () => {
  const annotated = (ch: string, caption: ProductFolderCaption | undefined): Folder => ({ ...pb3(ch),
    path: [{ kind: "Channel", ref_id: ch }, { kind: "Folder", ref_id: "PB-3", product_caption: caption }] });
  const membership = (items: ComObjectNode[]) => byFolder(items).groups.map(g => [g.key, g.objects.map(o => o.id), g.total]);
  const objects = [object(1, { project_folder: annotated("CH-2", productCaption("Same")) }),
    object(2, { project_folder: annotated("CH-1", productCaption("Same")) }),
    object(3, { project_folder: annotated("CH-2", productCaption("Different")) }), object(4)];
  const expected = [["folder:Channel:4:CH-2/Folder:4:PB-3", [1, 3], 2],
    ["folder:Channel:4:CH-1/Folder:4:PB-3", [2], 1], ["folder-none", [4], 1]];
  expect(membership(objects)).toEqual(expected);
  for (const caption of [undefined, { ...productCaption("Changed"), source_sha256: "cd".repeat(32) }]) {
    expect(membership(objects.map(o => ({ ...o, project_folder: o.project_folder ? {
      ...o.project_folder, path: o.project_folder.path.map(node => ({ ...node, product_caption: caption })),
    } : undefined })))).toEqual(expected);
  }
});
