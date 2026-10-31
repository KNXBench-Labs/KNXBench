/** Exercises exact-source bounded editor selection and lossless value presentation. */
import { describe, expect, it } from "vitest";
import { parameterDraftIssue, chooseParameterEditor, type ParameterEditorContext, type ParameterEditorField, type ParameterEditorSource } from "./plan";

const sha = "a".repeat(64);
const field: ParameterEditorField = { etsId: "PR", kind: "Time", value: "0010", min: "0", max: "3600", editable: true, writeEtsId: "PR", access: "ReadWrite" };
const context: ParameterEditorContext = { programId: "A", sourceSha256: sha, moduleScoped: false };
const source: ParameterEditorSource = { sourceSha256: sha, programId: "A", parameterRefId: "PR", parameterTypeId: "PT", schema: 20, child: "TypeTime", attributes: { SizeInBit: "16", minInclusive: "0", maxInclusive: "3600", Unit: "Seconds", UIHint: "Duration_hhmmss" }, parentAttributes: { Id: "PT" } };

describe("parameter editor admission", () => {
  it("OKX06 admits exact HostAddress source as a lossless IP literal editor", () => {
    expect(chooseParameterEditor(
      { ...field, kind: "IPAddress", min: null, max: null, value: "203.0.113.19" },
      { ...source, child: "TypeIPAddress", attributes: { AddressType: "HostAddress" } }, context,
    )).toEqual({ kind: "ip-address-literal" });
  });
  it("does not silently disregard unknown transformation metadata", () => {
    const metadata = { ...source, attributes: { ...source.attributes, DisplayFactor: "10" } };
    expect(chooseParameterEditor(field, metadata, context)).toEqual({ kind: "fallback", reason: "unsupported-hints" });
    expect(chooseParameterEditor(field, { ...source, parentAttributes: { Id: "PT", Offset: "10" } }, context)).toEqual({ kind: "fallback", reason: "unsupported-hints" });
  });
  it("preflights integer lexemes and bounds without coercing or rounding drafts", () => {
    const plan = { kind: "time-count", unit: null, min: "0", max: "9007199254740993" } as const;
    for (const value of ["", "1.5", "1e2", " 10", "10 ", "x", "9".repeat(65)]) expect(parameterDraftIssue(plan, value)).toBe("integer");
    for (const value of ["-1", "9007199254740994"]) expect(parameterDraftIssue(plan, value)).toBe("bounds");
    for (const value of ["0010", "+10", "9007199254740993"]) expect(parameterDraftIssue(plan, value)).toBeNull();
    expect(parameterDraftIssue({ kind: "fallback", reason: "unsupported-codec" }, "10")).toBe("unsupported");
  });
  it("refuses a slider whose bounds cannot be represented without integer rounding", () => {
    const max = "9007199254740993";
    const item = { ...field, kind: "Number", min: "0", max };
    const metadata = { ...source, child: "TypeNumber", attributes: { Type: "unsignedInt", UIHint: "Slider", minInclusive: "0", maxInclusive: max } };
    expect(chooseParameterEditor(item, metadata, context)).toEqual({ kind: "fallback", reason: "invalid-range" });
  });
  it("does not guess increment semantics or resolve contradictory slider hints", () => {
    const item = { ...field, kind: "Number", min: "0", max: "100" };
    const metadata = { ...source, child: "TypeNumber", attributes: { Type: "unsignedInt", UIHint: "Slider", minInclusive: "0", maxInclusive: "100", Increment: "10" } };
    expect(chooseParameterEditor(item, metadata, context)).toEqual({ kind: "fallback", reason: "unsupported-hints" });
    const contradictory = { ...metadata, attributes: { Type: "unsignedInt", UIHint: "Slider", minInclusive: "0", maxInclusive: "100" }, parentAttributes: { Id: "PT", UIHint: "Dropdown" } };
    expect(chooseParameterEditor(item, contradictory, context)).toEqual({ kind: "fallback", reason: "unsupported-hints" });
  });
  it("does not turn undeclared or fractional current values into a clamped slider value", () => {
    const metadata = { ...source, child: "TypeNumber", attributes: { Type: "unsignedInt", UIHint: "Slider", minInclusive: "0", maxInclusive: "100" } };
    for (const value of [null, "", "2.5", "-1", "101", "1e1"]) {
      expect(chooseParameterEditor({ ...field, kind: "Number", min: "0", max: "100", value }, metadata, context)).toEqual({ kind: "fallback", reason: "invalid-range" });
    }
  });
  it("keeps module scopes outside the direct-static evidence contract", () => {
    expect(chooseParameterEditor(field, source, { ...context, moduleScoped: true })).toEqual({ kind: "fallback", reason: "module-scope" });
  });
  it("reports unverified colour and picture encodings explicitly", () => {
    for (const kind of ["Color", "Picture"]) {
      const item = { ...field, kind, min: null, max: null, value: "literal" };
      const metadata = { ...source, child: `Type${kind}`, attributes: { RefId: "https://untrusted.invalid/image", Space: "RGB" } };
      expect(chooseParameterEditor(item, metadata, context)).toEqual({ kind: "fallback", reason: "unsupported-codec" });
    }
  });
  it("selects a bounded integer slider only from an explicit numeric slider declaration", () => {
    const item = { ...field, kind: "Number", min: "0", max: "100", value: "10" };
    const metadata = { ...source, child: "TypeNumber", attributes: { SizeInBit: "8", Type: "unsignedInt", UIHint: "Slider", minInclusive: "0", maxInclusive: "100" } };
    expect(chooseParameterEditor(item, metadata, context)).toEqual({ kind: "integer-slider", min: 0, max: 100, step: 1 });
  });
  it("does not specialize malformed, fractional or reversed declared integer ranges", () => {
    for (const [min, max] of [["NaN", "3600"], ["0", "Infinity"], ["0.5", "3600"], ["20", "10"]]) {
      const item = { ...field, min, max };
      const metadata = { ...source, attributes: { ...source.attributes, minInclusive: min, maxInclusive: max } };
      expect(chooseParameterEditor(item, metadata, context)).toEqual({ kind: "fallback", reason: "invalid-range" });
    }
  });
  it("refuses foreign or contradictory source identities before selecting a control", () => {
    const wrong: ParameterEditorSource[] = [
      { ...source, sourceSha256: "b".repeat(64) },
      { ...source, programId: "other" },
      { ...source, parameterRefId: "other" },
      { ...source, schema: 99 },
      { ...source, child: "TypeNumber" },
      { ...source, parentAttributes: { Id: "other" } },
      { ...source, attributes: { ...source.attributes, minInclusive: "1" } },
    ];
    for (const value of wrong) expect(chooseParameterEditor(field, value, context)).toEqual({ kind: "fallback", reason: "source-conflict" });
  });
  it("keeps the decimal count for every plain time unit the collection declares", () => {
    for (const unit of ["Milliseconds", "TenMilliseconds", "HundredMilliseconds", "Seconds", "Minutes", "Hours"]) {
      const metadata = { ...source, attributes: { ...source.attributes, Unit: unit } };
      expect(chooseParameterEditor(field, metadata, context)).toEqual({ kind: "time-count", unit, min: "0", max: "3600" });
    }
  });
  it("does not offer a packed or unknown time unit as a decimal count (census G6)", () => {
    for (const unit of ["PackedSecondsAndMilliseconds", "PackedMinutesSecondsAndMilliseconds", "PackedDaysHoursMinutesAndSeconds", "Fortnights", ""]) {
      const metadata = { ...source, attributes: { ...source.attributes, Unit: unit, UIHint: "Time_ssfff" } };
      expect(chooseParameterEditor(field, metadata, context)).toEqual({ kind: "fallback", reason: "unsupported-codec" });
    }
  });
  it("keeps an undeclared unit as an unlabelled count", () => {
    const { Unit: _unit, ...attributes } = source.attributes;
    expect(chooseParameterEditor(field, { ...source, attributes }, context)).toEqual({ kind: "time-count", unit: null, min: "0", max: "3600" });
  });
  it("admits time as an unchanged decimal count, not guessed clock arithmetic", () => {
    const before = JSON.stringify({ field, source, context });
    expect(chooseParameterEditor(field, source, context)).toEqual({ kind: "time-count", unit: "Seconds", min: "0", max: "3600" });
    expect(JSON.stringify({ field, source, context })).toBe(before);
    expect(field.value).toBe("0010");
  });
});
