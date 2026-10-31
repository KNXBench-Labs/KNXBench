/** Pins independently authored IP literal vectors and exact HostAddress source admission. */
import { describe, expect, it } from "vitest";
import { chooseParameterEditor, parameterDraftIssue, type ParameterEditorField, type ParameterEditorSource } from "./plan";
import vectors from "../../../../../crates/knx-productdb/tests/fixtures/ip-address-literal.json";

export const ipField: ParameterEditorField = { etsId: "PR", kind: "IPAddress", value: "203.0.113.19", min: null, max: null, editable: true, writeEtsId: "WRITE-IP", access: "ReadWrite" };
export const ipSource: ParameterEditorSource = { sourceSha256: "a".repeat(64), programId: "A", parameterRefId: "PR", parameterTypeId: "PT", schema: 20, child: "TypeIPAddress", attributes: { AddressType: "HostAddress" }, parentAttributes: { Id: "PT" } };
const context = { programId: "A", sourceSha256: "a".repeat(64), moduleScoped: false };

describe("OKX06 IP literal contract", () => {
  it.each(vectors.cases)("lexical preflight $raw => $accepted", ({ raw, accepted }) => {
    expect(parameterDraftIssue(chooseParameterEditor(ipField, ipSource, context), raw)).toBe(accepted ? null : "ip-address");
  });
  it("admits schemas 20..23 without interpreting historical raw values", () => {
    for (const schema of [20, 21, 22, 23]) for (const value of [null, "", "invalid", "203.0.113.19\n"]) {
      expect(chooseParameterEditor({ ...ipField, value }, { ...ipSource, schema }, context)).toEqual({ kind: "ip-address-literal" });
    }
  });
  it("refuses other roles and every extra or qualified hint, preserving generic fallback", () => {
    const attributes: Record<string, string>[] = [{}, { AddressType: "" }, { AddressType: "SubnetMask" }, { AddressType: "GatewayAddress" }, { AddressType: "hostaddress" }, { "v:AddressType": "HostAddress" },
      ...["SizeInBit", "Encoding", "UIHint", "Unit", "Type", "Increment", "v:UIHint"].map(key => ({ AddressType: "HostAddress", [key]: "32" }))];
    for (const a of attributes) expect(chooseParameterEditor(ipField, { ...ipSource, attributes: a }, context)).toEqual({ kind: "fallback", reason: "unsupported-hints" });
    for (const key of ["UIHint", "SizeInBit", "Encoding", "v:Name"]) {
      expect(chooseParameterEditor(ipField, { ...ipSource, parentAttributes: { Id: "PT", [key]: "x" } }, context)).toEqual({ kind: "fallback", reason: "unsupported-hints" });
    }
    expect(chooseParameterEditor(ipField, { ...ipSource, attributes: { AddressType: "HostAddress", xmlns: "x", "xmlns:v": "y" }, parentAttributes: { Id: "PT", Name: "Literal" } }, context)).toEqual({ kind: "ip-address-literal" });
  });
  it("retains identity, null bounds, missing-source and module checks", () => {
    for (const changed of [{ sourceSha256: "b".repeat(64) }, { sourceSha256: "a".repeat(63) + "\n" }, { programId: "other" }, { parameterRefId: "other" }, { parameterTypeId: "other" }, { child: "v:TypeIPAddress" }, { schema: 19 }]) {
      expect(chooseParameterEditor(ipField, { ...ipSource, ...changed }, context)).toEqual({ kind: "fallback", reason: "source-conflict" });
    }
    expect(chooseParameterEditor({ ...ipField, min: "0" }, ipSource, context)).toEqual({ kind: "fallback", reason: "source-conflict" });
    expect(chooseParameterEditor({ ...ipField, min: "0" }, { ...ipSource, attributes: { ...ipSource.attributes, minInclusive: "0" } }, context).kind).toBe("fallback");
    expect(chooseParameterEditor(ipField, null, context)).toEqual({ kind: "fallback", reason: "missing-evidence" });
    expect(chooseParameterEditor(ipField, ipSource, { ...context, moduleScoped: true })).toEqual({ kind: "fallback", reason: "module-scope" });
  });
});
