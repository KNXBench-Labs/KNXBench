/** Name admission and target lookup preserve Unicode and installation identity. */
import { expect, it } from "vitest";
import { hasRenameContext, nameProblem, renameTarget } from "./rename";
import { renameTree } from "./rename.fixture";
it("uses scalar count and Unicode White_Space without normalizing valid text", () => {
  for (const name of ["  Küche 🛠  ", "e\u0301", "重复", "\ufeff", "<script>literal"]) expect(nameProblem(name)).toBeNull();
  expect(nameProblem("🛠".repeat(1024))).toBeNull(); expect(nameProblem("🛠".repeat(1025))).toBe("long");
  for (const name of ["", " ", "\u0085\u2003"]) expect(nameProblem(name)).toBe("blank");
  for (const control of ["\0", "\t", "\r", "\n", "\u007f", "\u009f", "\u2028", "\u2029", "\ud800"]) expect(nameProblem(`name${control}`)).toBe("control");
});
it("allows repeated names/numeric addresses but refuses repeated internal GA IDs", () => {
  const tree = renameTree(); expect(renameTarget(tree, "group_address", 2)).toEqual({ kind: "group_address", id: 2, name: "Original" });
  tree.installations[0].group_addresses.push({ ...tree.installations[1].group_addresses[0] });
  expect(renameTarget(tree, "group_address", 2)).toBeNull();
});
it("requires actual server/project/snapshot context and refuses old/offline projections", () => {
  const tree = renameTree(); expect(hasRenameContext(tree)).toBe(true);
  expect(hasRenameContext({ ...tree, project_incarnation: undefined })).toBe(false);
  expect(hasRenameContext({ ...tree, server_incarnation: "" })).toBe(false);
  expect(hasRenameContext({ ...tree, snapshot_revision: Number.MAX_SAFE_INTEGER + 1 })).toBe(false);
});
