/** AR08: which import refusals ask for a project password, and which do not. */
import { describe, expect, it } from "vitest";
import { projectPasswordRefusal } from "./projectPassword";

const refusal = (status: number, body: unknown) => Object.assign(new Error("refused"), { status, body });

describe("projectPasswordRefusal", () => {
  it("recognises the two password refusals of POST /api/project/import", () => {
    expect(projectPasswordRefusal(refusal(422, { error: "x", kind: "projectPasswordRequired" }))).toBe("required");
    expect(projectPasswordRefusal(refusal(422, { error: "x", kind: "projectPasswordWrong" }))).toBe("wrong");
  });

  it("leaves every other failure to the ordinary error path", () => {
    expect(projectPasswordRefusal(refusal(422, { error: "x", kind: "importRefused" }))).toBeNull();
    expect(projectPasswordRefusal(refusal(400, { error: "x", kind: "projectPasswordRequired" }))).toBeNull();
    expect(projectPasswordRefusal(refusal(422, null))).toBeNull();
    expect(projectPasswordRefusal(new TypeError("network"))).toBeNull();
    expect(projectPasswordRefusal(undefined)).toBeNull();
  });
});
