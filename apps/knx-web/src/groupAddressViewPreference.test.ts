/** Tests for the group-address view preference outside project data. */
import { expect, it, vi } from "vitest";

it("defaults to tree, admits all four choices and tolerates unavailable storage", async () => {
  const path = "./groupAddressViewPreference";
  const preference = await import(/* @vite-ignore */ path).catch(() => null);
  expect(preference, "view preference module must exist").not.toBeNull();
  const storage = { getItem: vi.fn(() => null as string | null), setItem: vi.fn() };
  expect(preference!.loadGroupAddressView(storage)).toBe("tree");
  for (const view of ["tree", "columns", "atlas", "matrix"]) {
    storage.getItem.mockReturnValue(view);
    expect(preference!.loadGroupAddressView(storage)).toBe(view);
    expect(preference!.saveGroupAddressView(storage, view)).toBe(true);
    expect(storage.setItem).toHaveBeenLastCalledWith("groupAddressView", view);
  }
  storage.getItem.mockReturnValue("invented");
  expect(preference!.loadGroupAddressView(storage)).toBe("tree");
  storage.getItem.mockImplementation(() => { throw new Error("unavailable"); });
  expect(preference!.loadGroupAddressView(storage)).toBe("tree");
  storage.setItem.mockImplementation(() => { throw new Error("unavailable"); });
  expect(preference!.saveGroupAddressView(storage, "matrix")).toBe(false);
});
