/** Status metrics admission and decimal-MB formatting reject anything that is not measured. */
import { describe, expect, it } from "vitest";
import { admitStatusMetrics } from "./statusMetrics";
import type { ProjectTree } from "./bindings/ProjectTree";

const tree = { server_incarnation: "server-a", project_incarnation: 2, snapshot_revision: 7 } as ProjectTree;
const response = () => ({ serverIncarnation: "server-a", project: { state: "available", projectIncarnation: 2, snapshotRevision: 7, currentOnDiskBytes: 1_250_000 }, catalogue: { state: "available", bytes: 2_500_000, counts: { manufacturers: 1, products: 2, applicationPrograms: 3 } } });

describe("admitStatusMetrics", () => {
  it("rejects malformed bytes, states and counts before formatting", () => {
    for (const change of [
      { project: { ...response().project, currentOnDiskBytes: undefined } },
      { project: { ...response().project, currentOnDiskBytes: 1.5 } },
      { project: { ...response().project, state: "unknown" } },
      { project: { ...response().project, state: { toString: () => "available" }, currentOnDiskBytes: null } },
      { catalogue: { ...response().catalogue, counts: { ...response().catalogue.counts, products: undefined } } },
      { catalogue: { ...response().catalogue, state: { toString: () => "error" }, bytes: null, counts: null } },
      { catalogue: { ...response().catalogue, state: "error", bytes: 1 } },
    ]) expect(() => admitStatusMetrics({ ...response(), ...change }, tree)).toThrow(/Malformed status metrics/);
  });
  it("binds server and project incarnation, and revision to the tree", () => {
    expect(admitStatusMetrics(response(), tree)?.projectMatches).toBe(true);
    expect(admitStatusMetrics({ ...response(), serverIncarnation: "server-b" }, tree)).toBeNull();
    expect(admitStatusMetrics({ ...response(), project: { ...response().project, projectIncarnation: 3 } }, tree)).toBeNull();
    expect(admitStatusMetrics({ ...response(), project: { ...response().project, snapshotRevision: 8 } }, tree)?.projectMatches).toBe(false);
    expect(admitStatusMetrics(response(), { ...tree, server_incarnation: undefined })).toBeNull();
    expect(admitStatusMetrics(response(), { ...tree, project_incarnation: undefined })).toBeNull();
    expect(admitStatusMetrics(response(), { ...tree, snapshot_revision: undefined })?.projectMatches).toBe(false);
  });
  it("admits explicit no-project unavailable fields", () => {
    const noProject = { ...tree, project_incarnation: undefined, snapshot_revision: undefined };
    expect(admitStatusMetrics({ ...response(), project: { state: "unavailable", projectIncarnation: null, snapshotRevision: null, currentOnDiskBytes: null } }, noProject)?.projectMatches).toBe(true);
  });
  it("admits an explicitly busy catalogue without measurements", () => {
    const busy = { ...response(), catalogue: { state: "busy", bytes: null, counts: null } };
    expect(admitStatusMetrics(busy, tree)?.metrics.catalogue).toEqual({ state: "busy", bytes: null, counts: null });
  });
});
