/** Issue #21: what the parameter panel says about an application program it cannot resolve. */
import { describe, expect, it } from "vitest";
import type { ManufacturerFileRestore, UnresolvedProgram } from "./api";
import { restoreChangedSomething, unresolvedProgramState } from "./unresolvedProgram";

const info = (over: Partial<UnresolvedProgram>): UnresolvedProgram => ({
  programRef: "M-0001_H-1_HP-1", productDatabase: true, missingFiles: 0, ...over,
});

const restore = (over: Partial<ManufacturerFileRestore>): ManufacturerFileRestore => ({
  installed: [], alreadyInstalled: 0, mismatched: [], notInArchive: [], failed: [], stillMissing: 0, ...over,
});

describe("unresolvedProgramState", () => {
  it("names a missing product database first, whatever else is known", () => {
    expect(unresolvedProgramState(info({ productDatabase: false, missingFiles: 3 }))).toBe("noDatabase");
  });

  it("offers a restore only when the project's own files are missing", () => {
    expect(unresolvedProgramState(info({ missingFiles: 2 }))).toBe("restorable");
    expect(unresolvedProgramState(info({ missingFiles: 0 }))).toBe("notInstalled");
  });
});

describe("restoreChangedSomething", () => {
  it("reloads only when a file was actually installed", () => {
    expect(restoreChangedSomething(restore({ installed: ["M-0001/Hardware.xml"] }))).toBe(true);
    expect(restoreChangedSomething(restore({ mismatched: ["M-0001/M-0001_A-1.xml"], stillMissing: 1 }))).toBe(false);
  });
});
