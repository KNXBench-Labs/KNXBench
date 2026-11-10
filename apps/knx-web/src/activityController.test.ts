/** Pins activity deadlines, operation identity, retention and stale-result withdrawal. */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ACTIVITY_DELAY_MS, ACTIVITY_RECENT_LIMIT, ACTIVITY_SUCCESS_MS, ActivityController, activityHighlighted, type ActivityIdentity } from "./activityController";

let activity: ActivityController;
let disconnect: () => void;
let now = 0;
const identity = (operationId: string, extra: Partial<ActivityIdentity> = {}): ActivityIdentity => ({ kind: "open", operationId, access: "read", ...extra });
beforeEach(() => {
  vi.useFakeTimers(); now = 0;
  activity = new ActivityController({ now: () => now,
    schedule: (callback, ms) => setTimeout(callback, ms), clear: timer => clearTimeout(timer as ReturnType<typeof setTimeout>) });
  disconnect = activity.connect();
});
afterEach(() => { disconnect(); vi.useRealTimers(); });
function advance(ms: number) { now += ms; vi.advanceTimersByTime(ms); }
const highlighted = () => activity.getSnapshot().filter(activityHighlighted);

describe("delayed presentation follows producer completion", () => {
  it("acknowledges immediately in inventory but a completion below 2s never flashes", () => {
    const task = activity.begin(identity("quick"));
    expect(activity.getSnapshot()).toHaveLength(1);
    advance(ACTIVITY_DELAY_MS - 1); expect(highlighted()).toEqual([]);
    task.finish(); advance(ACTIVITY_DELAY_MS + ACTIVITY_SUCCESS_MS);
    expect(highlighted()).toEqual([]); expect(activity.getSnapshot()[0].state).toBe("succeeded");
  });
  it("appears at 2s and clears 3s after full completion while retaining recent history", () => {
    const task = activity.begin(identity("long"));
    task.update({ phase: "Measured phase", progress: { completed: "7", total: "10" } });
    advance(ACTIVITY_DELAY_MS);
    expect(highlighted()[0]).toMatchObject({ state: "running", phase: "Measured phase", progress: { completed: "7", total: "10" } });
    task.finish(); expect(highlighted()[0]).toMatchObject({ state: "succeeded", phase: undefined, phaseKey: undefined, progress: undefined });
    advance(ACTIVITY_SUCCESS_MS - 1); expect(highlighted()).toHaveLength(1);
    advance(1); expect(highlighted()).toEqual([]); expect(activity.getSnapshot()).toHaveLength(1);
  });
  it("has a bounded session recent list but never silently evicts unread errors", () => {
    const error = activity.begin(identity("error")); error.finish("failed", "read failed");
    for (let i = 0; i < ACTIVITY_RECENT_LIMIT + 5; i++) activity.begin(identity(String(i))).finish();
    expect(activity.getSnapshot()).toHaveLength(ACTIVITY_RECENT_LIMIT + 1);
    advance(60_000); expect(highlighted()).toHaveLength(1);
    activity.acknowledge(error.id); expect(highlighted()).toEqual([]);
    expect(activity.getSnapshot()).toHaveLength(ACTIVITY_RECENT_LIMIT);
  });
  it("keeps concurrent activities independent and decisions static rather than failed", () => {
    const a = activity.begin(identity("a")); const b = activity.begin(identity("b"));
    advance(ACTIVITY_DELAY_MS); expect(highlighted()).toHaveLength(2);
    a.finish("decision"); b.update({ state: "waiting" });
    expect(highlighted().map(row => row.state)).toEqual(["decision", "waiting"]);
    advance(ACTIVITY_SUCCESS_MS); expect(highlighted().map(row => row.state)).toEqual(["waiting"]);
    b.update({ state: "cancelling" }); advance(60_000);
    expect(highlighted()[0].state).toBe("cancelling");
    b.finish("cancelled"); expect(highlighted()).toEqual([]);
  });
});

describe("identity and lifecycle authority", () => {
  it.each(["succeeded", "failed", "decision", "cancelled", "discarded"] as const)("clears active phase data on %s without losing errors", state => {
    const task = activity.begin(identity(`phase-${state}`));
    task.update({ phase: "Reading", phaseKey: "loadProgress.phase.buildProjectTree", progress: { completed: "1", total: "2" }, errorKey: "backgroundActivity.state.uncertain" });
    task.finish(state, "retained error");
    expect(activity.getSnapshot()[0]).toMatchObject({ state, phase: undefined, phaseKey: undefined, progress: undefined,
      error: "retained error", errorKey: "backgroundActivity.state.uncertain" });
  });
  it("captures readable project and device context independently of fencing and source filename", () => {
    const entities = [{ label: "Pump controller", address: "1.1.42" }];
    const task = activity.begin(identity("context", { projectScope: {}, entityId: "42" }), {
      source: "source.knxdb", projectName: "Original installation", entities,
    });
    entities[0].label = "Renamed in successor";
    task.finish();
    expect(activity.getSnapshot()[0]).toMatchObject({ source: "source.knxdb", projectName: "Original installation",
      entities: [{ label: "Pump controller", address: "1.1.42" }] });
  });
  it("deduplicates an exact observer identity, not a filename or entity caption", () => {
    const scope = {};
    const a = activity.begin(identity("one", { projectScope: scope }), { source: "same.knxdb" });
    expect(activity.begin(identity("one", { projectScope: scope })).id).toBe(a.id);
    const b = activity.begin(identity("two", { projectScope: scope }), { source: "same.knxdb" });
    a.finish(); expect(a.update({ phase: "late" })).toBe(false); expect(a.finish("failed")).toBe(false);
    expect(activity.getSnapshot().find(row => row.id === b.id)?.state).toBe("running");
    expect(activity.getSnapshot()).toHaveLength(2);
  });
  it("rejects an evicted handle and never applies it to a replacement operation", () => {
    const a = activity.begin(identity("retired")); a.finish();
    for (let i = 0; i < ACTIVITY_RECENT_LIMIT; i++) activity.begin(identity(`recent-${i}`)).finish();
    const b = activity.begin(identity("replacement"));
    expect(a.update({ state: "running" })).toBe(false); expect(a.finish("failed", "old")).toBe(false);
    expect(activity.getSnapshot().find(row => row.id === b.id)?.error).toBeUndefined();
  });
  it("discards project reads while retaining writes and independent catalogue work until settlement", () => {
    const scope = {}; const anotherScope = {};
    const read = activity.begin(identity("read", { projectScope: scope }));
    const write = activity.begin(identity("write", { projectScope: scope, access: "write" }));
    const catalogue = activity.begin(identity("catalogue", { kind: "catalogue", access: "independent" }));
    const newRead = activity.begin(identity("read", { projectScope: anotherScope }));
    read.update({ phase: "Retired phase", phaseKey: "loadProgress.phase.buildProjectTree", progress: { completed: "1", total: "2" } });
    activity.invalidateScope(scope);
    expect(activity.getSnapshot().find(row => row.id === read.id)).toMatchObject({ phase: undefined, phaseKey: undefined, progress: undefined });
    expect(read.finish()).toBe(false);
    expect(activity.getSnapshot().map(row => row.state)).toEqual(["discarded", "running", "running", "running"]);
    write.finish(); catalogue.finish(); newRead.finish();
  });
  it("disconnect releases only presentation timers and reconnection honors elapsed time", () => {
    const task = activity.begin(identity("retained")); disconnect();
    now += ACTIVITY_DELAY_MS + 1; vi.advanceTimersByTime(ACTIVITY_DELAY_MS + 1);
    expect(activity.getSnapshot()[0].visible).toBe(false);
    disconnect = activity.connect(); expect(highlighted()[0].state).toBe("running");
    task.finish();
  });
});
