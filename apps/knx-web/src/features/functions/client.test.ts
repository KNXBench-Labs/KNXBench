/** Strict admission and request shape of the project Functions client (Goal B1b). */
import { describe, expect, it, vi } from "vitest";
import {
  admitFunctionsEdited, admitFunctionsRead, createFunctionsClient, FunctionsContractError, isFunctionsRefused, isFunctionsStale,
} from "./client";

const member = (id: number, ga: number) => ({ id, sourceId: `P-0001-0_GF-${id}`, groupAddressId: ga, address: "1/0/1",
  groupAddressName: "Light", name: "", role: "SwitchOnOff" });
const fn = (id: number, members = [member(10 + id, 7)]) => ({
  id, sourceId: `P-0001-0_F-${id}`, installationId: 0, location: { buildingPartId: 3, name: "Kitchen", kind: "Room" },
  name: "Kitchen light", functionType: "FT-1", number: null, comment: "", description: null, completion: "Editing", members,
});
const read = (functions: unknown[] = [fn(1)]) => ({ functions, serverIncarnation: "srv", projectIncarnation: 2, snapshotRevision: 9 });

describe("Functions client admission", () => {
  it("admits the read exactly, keeping imported text and null versus empty verbatim", () => {
    const view = admitFunctionsRead(read([fn(1, [member(5, 7)])]));
    expect(view.functions[0].comment).toBe("");
    expect(view.functions[0].description).toBeNull();
    expect(view.functions[0].members[0].sourceId).toBe("P-0001-0_GF-5");
    expect(admitFunctionsRead(read([{ ...fn(1), name: "Line\u0007bell" }])).functions[0].name).toBe("Line\u0007bell");
  });

  it.each([
    ["an unknown top-level key", { ...read(), extra: 1 }],
    ["a missing context", { functions: [] }],
    ["an unknown function key", read([{ ...fn(1), site: 1 }])],
    ["a missing source id", read([(({ sourceId: _s, ...rest }) => rest)(fn(1))])],
    ["an unknown completion", read([{ ...fn(1), completion: "Done" }])],
    ["a repeated function id", read([fn(1), fn(1, [])])],
    ["a repeated member id across functions", read([fn(1, [member(4, 7)]), fn(2, [member(4, 8)])])],
    ["an installation id above 255", read([{ ...fn(1), installationId: 256 }])],
    ["a zero function id", read([fn(0)])],
  ])("refuses %s as a whole", (_case, value) => {
    expect(() => admitFunctionsRead(value)).toThrow(FunctionsContractError);
  });

  it("admits an edit answer with its created id and refuses a malformed one", () => {
    expect(admitFunctionsEdited({ ...read(), createdId: 4 }).createdId).toBe(4);
    expect(admitFunctionsEdited({ ...read(), createdId: null }).createdId).toBeNull();
    expect(() => admitFunctionsEdited(read())).toThrow(FunctionsContractError);
    expect(() => admitFunctionsEdited({ ...read(), createdId: 0 })).toThrow(FunctionsContractError);
  });
});

describe("Functions client requests", () => {
  it("sends the read's binding with the action and nothing else", async () => {
    const transport = vi.fn().mockResolvedValue(new Response(JSON.stringify({ ...read(), createdId: null })));
    const client = createFunctionsClient(transport);
    await client.edit({ serverIncarnation: "srv", projectIncarnation: 2, snapshotRevision: 9 }, { kind: "removeMember", functionId: 1, memberId: 11 });
    const [path, init] = transport.mock.calls[0];
    expect(path).toBe("/api/project/functions/edit");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual({ serverIncarnation: "srv", projectIncarnation: 2, snapshotRevision: 9,
      action: { kind: "removeMember", functionId: 1, memberId: 11 } });
  });

  it.each([[409, "functionStale", true, false], [422, "functionRefused", false, true], [409, "renameConflict", false, false], [500, null, false, false]])(
    "types a %i %s refusal", async (status, kind, stale, refused) => {
      const body = kind === null ? "{}" : JSON.stringify({ error: "server text", kind });
      const client = createFunctionsClient(vi.fn().mockResolvedValue(new Response(body, { status })));
      const error = await client.read().catch((reason: unknown) => reason);
      expect(isFunctionsStale(error)).toBe(stale);
      expect(isFunctionsRefused(error)).toBe(refused);
      expect((error as { status: number }).status).toBe(status);
    });
});
