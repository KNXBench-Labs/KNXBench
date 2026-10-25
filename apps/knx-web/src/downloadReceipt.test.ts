/** The shared receipt table through the strict web reader: the Rust reader runs the same files. */
import { expect, it } from "vitest";
import { admitHistoryPage } from "./activityHistory";
import fixtures from "./downloadReceipt.fixtures.json";
import cases from "./downloadReceipt.cases.json";

type Case = { name: string; fixture: keyof typeof fixtures; set: [string, unknown][]; remove: string[] };

function edited(testCase: Case): Record<string, unknown> {
  const document = structuredClone(fixtures[testCase.fixture]) as Record<string, unknown>;
  const resolve = (pointer: string) => {
    const parts = pointer.split("/").slice(1);
    const key = parts.pop()!;
    let parent: unknown = document;
    for (const part of parts) parent = (parent as Record<string, unknown>)[part];
    return [parent as Record<string, unknown>, key] as const;
  };
  for (const [pointer, value] of testCase.set) {
    const [parent, key] = resolve(pointer);
    parent[key] = value;
  }
  for (const pointer of testCase.remove) {
    const [parent, key] = resolve(pointer);
    expect(Object.hasOwn(parent, key), `${testCase.name}: ${pointer}`).toBe(true);
    delete parent[key];
  }
  return document;
}

/** A page of one stored row, with the fields the reader adds around the stored document. */
function pageOf(document: Record<string, unknown>) {
  const entries: Record<string, unknown>[] = [{ sequence: 1, serverIncarnation: "synthetic-server", interrupted: false, ...document }];
  return {
    format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: 1,
    untracked: ["deviceIdentify", "groupWrite", "serialAddress", "deviceDownload", "addressProgramming", "busMonitor", "lineScan"],
    entries,
  };
}

it("admits every accepted case unchanged, with or without a receipt", () => {
  expect(cases.accepted.length).toBeGreaterThanOrEqual(Object.keys(fixtures).length);
  for (const testCase of cases.accepted as Case[]) {
    const page = pageOf(edited(testCase));
    expect(admitHistoryPage(page), testCase.name).toBe(page);
  }
});

it("refuses the whole page for every refused case, never a valid prefix", () => {
  expect(cases.refused.length).toBeGreaterThanOrEqual(60);
  for (const testCase of cases.refused as Case[]) {
    const bad = edited(testCase);
    expect(() => admitHistoryPage(pageOf(bad)), testCase.name).toThrow(/history/i);
    // The same row behind a valid one: the valid row is not returned either.
    const behind = pageOf(structuredClone(fixtures.completeSessionOnly) as Record<string, unknown>);
    behind.entries.push({ sequence: 2, serverIncarnation: "synthetic-server", interrupted: false, ...bad, id: 2 });
    behind.nextCursor = 2;
    expect(() => admitHistoryPage(behind), `${testCase.name} behind a valid row`).toThrow(/history/i);
  }
});

it("reads absence as absence: a legacy row carries neither receipt and is not refused", () => {
  const page = admitHistoryPage(pageOf(structuredClone(fixtures.legacy) as Record<string, unknown>));
  const evidence = page.entries[0].downloadEvidence!;
  expect(evidence.preparation).toBeUndefined();
  expect(evidence.settlement).toBeUndefined();
  expect(Object.hasOwn(evidence, "preparation")).toBe(false);
});
