/** Pure Functions view logic: room/address selection and lossless detail drafts. */
import type { FunctionNode } from "../../bindings/FunctionNode";
import type { FunctionMemberNode } from "../../bindings/FunctionMemberNode";
import { COMPLETIONS, type Completion, type FunctionDetailsDraft } from "./client";

/** Functions located in one building part, in model order. */
export function functionsInPart(functions: readonly FunctionNode[], buildingPartId: number): FunctionNode[] {
  return functions.filter((node) => node.location.buildingPartId === buildingPartId);
}

/** Every membership of one group address, one entry per (Function, member). */
export function membershipsOf(functions: readonly FunctionNode[], groupAddressId: number):
  { function: FunctionNode; member: FunctionMemberNode }[] {
  return functions.flatMap((node) => node.members
    .filter((member) => member.groupAddressId === groupAddressId)
    .map((member) => ({ function: node, member })));
}

/** The admitted completion, or `null` for a spelling this client does not know. */
export function completionOf(node: FunctionNode): Completion | null {
  return (COMPLETIONS as readonly string[]).includes(node.completion) ? node.completion as Completion : null;
}

/** The details exactly as stored, as the starting point of an edit. */
export function detailsOf(node: FunctionNode): FunctionDetailsDraft {
  return {
    name: node.name, functionType: node.functionType, number: node.number, comment: node.comment,
    description: node.description, completion: completionOf(node) ?? "Undefined",
  };
}

export const EMPTY_DETAILS: FunctionDetailsDraft = {
  name: "", functionType: null, number: null, comment: null, description: null, completion: "Undefined",
};

/**
 * An optional text field's value after editing. Clearing a field that was
 * absent keeps it absent; clearing one that held text (or was present but
 * empty) keeps it present and empty, so presentation never becomes an edit.
 */
export function optionalFieldValue(input: string, base: string | null): string | null {
  return input === "" && base === null ? null : input;
}

export function sameDetails(a: FunctionDetailsDraft, b: FunctionDetailsDraft): boolean {
  return a.name === b.name && a.functionType === b.functionType && a.number === b.number && a.comment === b.comment
    && a.description === b.description && a.completion === b.completion;
}

/** Only the blank-name case is checked before sending; every other rule is the server's. */
export function detailsProblem(draft: FunctionDetailsDraft): "blank" | null {
  return /^\p{White_Space}*$/u.test(draft.name) ? "blank" : null;
}
