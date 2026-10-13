/** Goal B7: the naming-recipe client admits only the preview it asked for. */
import { describe, expect, it, vi } from "vitest";
import { applyNameRecipe, isStaleRecipe, previewNameRecipe, RecipeContractError } from "./client";

const BINDING = { serverIncarnation: "srv", projectIncarnation: 3, snapshotRevision: 7 };
const ROWS = [
  { id: 9, before: "Hall", after: "EG Hall", changed: true },
  { id: 4, before: "EG Light", after: "EG Light", changed: false },
];
function transport(status: number, body: unknown) {
  return vi.fn(async (_path: RequestInfo | URL, _init?: RequestInit) =>
    new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } }));
}

describe("naming recipe client", () => {
  it("sends the binding, the ids in order and the literal affixes", async () => {
    const send = transport(200, { token: "t-1", rows: ROWS, changed: 1, noOp: false, expiresInSeconds: 300 });
    const plan = await previewNameRecipe(BINDING, [9, 4], "EG ", "", send);
    expect(plan.rows).toEqual(ROWS);
    const [path, init] = send.mock.calls[0];
    expect(path).toBe("/api/name-recipe/preview");
    expect(JSON.parse(String(init?.body))).toEqual({ ...BINDING, groupAddresses: [9, 4], prefix: "EG ", suffix: "" });
  });

  it.each([
    ["another order", { rows: [ROWS[1], ROWS[0]] }],
    ["a missing row", { rows: [ROWS[0]] }],
    ["a wrong count", { changed: 2 }],
    ["a no-op that changes", { noOp: true }],
    ["a row whose flag lies", { rows: [{ ...ROWS[0], changed: false }, ROWS[1]] }],
    ["a long-lived token", { expiresInSeconds: 3600 }],
    ["no token", { token: "" }],
  ])("refuses a preview with %s", async (_, change) => {
    const send = transport(200, { token: "t-1", rows: ROWS, changed: 1, noOp: false, expiresInSeconds: 300, ...change });
    await expect(previewNameRecipe(BINDING, [9, 4], "EG ", "", send)).rejects.toBeInstanceOf(RecipeContractError);
  });

  it("applies with the token and an explicit confirmation only", async () => {
    const send = transport(200, { changed: true, nativeGeneration: 12, snapshotRevision: 8, persistence: "native-working-history" });
    await applyNameRecipe("t-1", send);
    expect(JSON.parse(String(send.mock.calls[0][1]?.body))).toEqual({ token: "t-1", confirmed: true });
  });

  it("recognises only the typed stale refusal as stale", async () => {
    const stale = await applyNameRecipe("t-1", transport(409, { error: "preview again", kind: "nameRecipeStale" })).catch((e) => e);
    expect(isStaleRecipe(stale)).toBe(true);
    expect(stale.message).toBe("preview again");
    const other409 = await applyNameRecipe("t-1", transport(409, { error: "busy", kind: "projectBusy" })).catch((e) => e);
    expect(isStaleRecipe(other409)).toBe(false);
    const refused = await applyNameRecipe("t-1", transport(422, { error: "file", kind: "nameRecipePersistenceFailed" })).catch((e) => e);
    expect(isStaleRecipe(refused)).toBe(false);
    expect(refused.kind).toBe("nameRecipePersistenceFailed");
  });
});
