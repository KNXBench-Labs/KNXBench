/** Pins refusal admission and explicit choices of note-aware deletes. */
import { describe, expect, it } from "vitest";
import {
  dispositionRequired, initialChoices, isDispositionStale, moveOptions, wireChoices,
  type DispositionRequired,
} from "./noteDisposition";

const context = { serverIncarnation: "srv", projectIncarnation: 2, snapshotRevision: 9 };
const body = (extra: Record<string, unknown> = {}) => ({
  kind: "noteDispositionRequired",
  error: "this change would leave 2 project notes without their target: …",
  notes: [
    { id: 1, title: "Device note", target: { kind: "device", id: 4 } },
    { id: 3, title: "Object note", target: { kind: "comObject", id: 8 } },
  ],
  removedTargets: [{ kind: "device", id: 4 }, { kind: "comObject", id: 8 }],
  ...context,
  ...extra,
});
const refusal = (status: number, payload: unknown) => Object.assign(new Error("refused"), { status, body: payload });

describe("dispositionRequired", () => {
  it("admits the structured 409 with notes, removed targets and the snapshot", () => {
    const admitted = dispositionRequired(refusal(409, body()));
    expect(admitted).toEqual({
      message: "this change would leave 2 project notes without their target: …",
      notes: body().notes,
      removedTargets: body().removedTargets,
      context,
    });
  });

  it("refuses everything else, so the caller shows the plain error instead", () => {
    expect(dispositionRequired(refusal(400, body()))).toBeNull();
    expect(dispositionRequired(refusal(409, body({ kind: "projectUnsavedChanges" })))).toBeNull();
    expect(dispositionRequired(refusal(409, body({ notes: [] })))).toBeNull();
    expect(dispositionRequired(refusal(409, body({ notes: [{ id: 0, title: "x", target: { kind: "project" } }] })))).toBeNull();
    expect(dispositionRequired(refusal(409, body({ notes: [{ id: 1, title: 7, target: { kind: "device", id: 4 } }] })))).toBeNull();
    expect(dispositionRequired(refusal(409, body({ notes: [{ id: 1, title: "x", target: { kind: "room", id: 4 } }] })))).toBeNull();
    expect(dispositionRequired(refusal(409, body({ removedTargets: "all" })))).toBeNull();
    expect(dispositionRequired(refusal(409, body({ snapshotRevision: -1 })))).toBeNull();
    expect(dispositionRequired(refusal(409, body({ serverIncarnation: "" })))).toBeNull();
    expect(dispositionRequired(new Error("network"))).toBeNull();
    expect(dispositionRequired(null)).toBeNull();
  });

  it("refuses a duplicated note id rather than guessing which entry counts", () => {
    const notes = [body().notes[0], body().notes[0]];
    expect(dispositionRequired(refusal(409, body({ notes })))).toBeNull();
  });
});

describe("isDispositionStale", () => {
  it("recognises only the stale apply answer", () => {
    expect(isDispositionStale(refusal(409, { kind: "noteDispositionStale", error: "changed" }))).toBe(true);
    expect(isDispositionStale(refusal(409, body()))).toBe(false);
    expect(isDispositionStale(refusal(422, { kind: "noteDispositionStale" }))).toBe(false);
  });
});

describe("choices", () => {
  const required = dispositionRequired(refusal(409, body())) as DispositionRequired;

  it("starts every note on the non-destructive choice", () => {
    expect([...initialChoices(required.notes)]).toEqual([[1, { kind: "project" }], [3, { kind: "project" }]]);
  });

  it("offers every catalogue entity except the removed ones as a move target", () => {
    const options = moveOptions([
      { target: { kind: "device", id: 4 }, label: "Device: 1.1.4" },
      { target: { kind: "line", id: 2 }, label: "Line: 1.1" },
      { target: { kind: "comObject", id: 8 }, label: "Object 8" },
      { target: { kind: "device", id: 5 }, label: "Device: 1.1.5" },
    ], required.removedTargets);
    expect(options.map((option) => option.label)).toEqual(["Line: 1.1", "Device: 1.1.5"]);
  });

  it("builds the wire choices in note order only when every note is decided", () => {
    const choices = initialChoices(required.notes);
    choices.set(1, { kind: "move", target: null });
    choices.set(3, { kind: "delete" });
    expect(wireChoices(required.notes, choices)).toBeNull();
    choices.set(1, { kind: "move", target: { kind: "line", id: 2 } });
    expect(wireChoices(required.notes, choices)).toEqual([
      { note: 1, action: { kind: "move", target: { kind: "line", id: 2 } } },
      { note: 3, action: { kind: "delete" } },
    ]);
    choices.delete(3);
    expect(wireChoices(required.notes, choices)).toBeNull();
  });
});
