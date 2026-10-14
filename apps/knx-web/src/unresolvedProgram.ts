/** Decides what the Parameters tab says when a device's program does not resolve (#21). */
import type { ManufacturerFileRestore, UnresolvedProgram } from "./api";

/**
 * - `noDatabase`: the server has no product database at all.
 * - `restorable`: the project names manufacturer files the database lacks;
 *   the original `.knxproj` can supply them.
 * - `notInstalled`: nothing of the project is missing, the program is just
 *   not installed (a device added from another product package).
 */
export type UnresolvedProgramState = "noDatabase" | "restorable" | "notInstalled";

export function unresolvedProgramState(info: UnresolvedProgram): UnresolvedProgramState {
  if (!info.productDatabase) return "noDatabase";
  return info.missingFiles > 0 ? "restorable" : "notInstalled";
}

/** Whether a restore result means the panel should reload. */
export function restoreChangedSomething(result: ManufacturerFileRestore): boolean {
  return result.installed.length > 0;
}
