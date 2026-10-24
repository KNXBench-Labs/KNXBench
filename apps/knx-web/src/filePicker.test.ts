// @vitest-environment happy-dom
//
// isTauri() reads `window.__TAURI__`, so this file alone needs a browser-
// like global `window` — the rest of the suite runs under vitest's default
// "node" environment (see vitest.config.ts) and doesn't need one.
import { describe, expect, it, vi, beforeEach } from "vitest";

describe("isTauri", () => {
  beforeEach(() => {
    vi.resetModules();
    // @ts-expect-error test-only cleanup
    delete window.__TAURI__;
  });

  it("is false when window.__TAURI__ is absent", async () => {
    const { isTauri } = await import("./filePicker");
    expect(isTauri()).toBe(false);
  });

  it("is true when window.__TAURI__ is present", async () => {
    // @ts-expect-error test-only marker, shape doesn't matter
    window.__TAURI__ = {};
    const { isTauri } = await import("./filePicker");
    expect(isTauri()).toBe(true);
  });
});

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  join: vi.fn(),
  open: vi.fn(),
  save: vi.fn(),
  openMountPicker: vi.fn(),
  saveMountPicker: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
vi.mock("@tauri-apps/api/path", () => ({ join: native.join }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: native.open, save: native.save }));
vi.mock("./FsPicker", () => ({ openMountPicker: native.openMountPicker, saveMountPicker: native.saveMountPicker }));

const filters = [{ name: "KNXBench project", extensions: ["knxbench"] }];

describe("native dialog start folder", () => {
  beforeEach(() => {
    for (const mock of Object.values(native)) mock.mockReset();
    native.join.mockImplementation(async (...parts: string[]) => parts.join("/"));
    native.open.mockResolvedValue("/home/alice/projects/picked.knxbench");
    native.save.mockResolvedValue("/home/alice/projects/saved.knxbench");
    // @ts-expect-error test-only marker, shape doesn't matter
    window.__TAURI__ = {};
  });

  it("joins the shell's directory with the default name for Save As", async () => {
    native.invoke.mockResolvedValue("/home/alice/projects");
    const { pickSavePath } = await import("./filePicker");
    expect(await pickSavePath(filters, "house.knxbench")).toBe("/home/alice/projects/saved.knxbench");
    expect(native.invoke).toHaveBeenCalledWith("default_dialog_directory");
    expect(native.join).toHaveBeenCalledWith("/home/alice/projects", "house.knxbench");
    expect(native.save).toHaveBeenCalledWith({ filters, defaultPath: "/home/alice/projects/house.knxbench" });
  });

  it("starts Open in the shell's directory", async () => {
    native.invoke.mockResolvedValue("/home/alice/projects");
    const { pickOpenPath } = await import("./filePicker");
    expect(await pickOpenPath(filters)).toBe("/home/alice/projects/picked.knxbench");
    expect(native.open.mock.calls).toEqual([[{ multiple: false, filters, defaultPath: "/home/alice/projects" }]]);
  });

  it.each([
    ["reports no directory", () => native.invoke.mockResolvedValue(null)],
    ["cannot be asked", () => native.invoke.mockRejectedValue(new Error("Command not allowed by ACL"))],
  ])("keeps the previous dialog arguments when the shell %s", async (_case, arrange) => {
    arrange();
    const { pickOpenPath, pickSavePath } = await import("./filePicker");
    await pickSavePath(filters, "house.knxbench");
    await pickOpenPath(filters);
    expect(native.join).not.toHaveBeenCalled();
    // Exact argument lists: no `defaultPath: undefined` key sneaks in either.
    expect(native.save.mock.calls).toEqual([[{ filters, defaultPath: "house.knxbench" }]]);
    expect(native.open.mock.calls).toEqual([[{ multiple: false, filters }]]);
    expect(Object.keys(native.open.mock.calls[0][0])).toEqual(["multiple", "filters"]);
  });

  it("keeps the bare name when joining fails", async () => {
    native.invoke.mockResolvedValue("/home/alice/projects");
    native.join.mockRejectedValue(new Error("join refused"));
    const { pickSavePath } = await import("./filePicker");
    await pickSavePath(filters, "house.knxbench");
    expect(native.save.mock.calls).toEqual([[{ filters, defaultPath: "house.knxbench" }]]);
  });

  it("leaves the web build's mount pickers untouched", async () => {
    // @ts-expect-error test-only cleanup
    delete window.__TAURI__;
    native.openMountPicker.mockResolvedValue("/projects/a.knxbench");
    native.saveMountPicker.mockResolvedValue("/projects/b.knxbench");
    const { pickOpenPath, pickSavePath } = await import("./filePicker");
    expect(await pickOpenPath(filters)).toBe("/projects/a.knxbench");
    expect(await pickSavePath(filters, "house.knxbench")).toBe("/projects/b.knxbench");
    expect(native.openMountPicker).toHaveBeenCalledWith(filters);
    expect(native.saveMountPicker).toHaveBeenCalledWith("house.knxbench");
    expect(native.invoke).not.toHaveBeenCalled();
    expect(native.join).not.toHaveBeenCalled();
    expect(native.open).not.toHaveBeenCalled();
    expect(native.save).not.toHaveBeenCalled();
  });
});
