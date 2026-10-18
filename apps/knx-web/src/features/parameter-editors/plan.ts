/** Selects bounded parameter controls from exact source evidence, never product-name heuristics. */

export interface ParameterEditorField {
  etsId: string;
  kind: string;
  value: string | null;
  min: string | null;
  max: string | null;
  editable: boolean;
  writeEtsId: string | null;
  access: string | null;
}

/** Host-projected evidence from the read-only ProductDB adapter; not an HTTP wire DTO. */
export interface ParameterEditorSource {
  sourceSha256: string;
  programId: string;
  parameterRefId: string;
  parameterTypeId: string;
  schema: number;
  child: string;
  attributes: Record<string, string>;
  parentAttributes: Record<string, string>;
}

export interface ParameterEditorContext {
  programId: string;
  sourceSha256: string;
  moduleScoped: boolean;
}

export type ParameterEditorPlan =
  | { kind: "time-count"; unit: string | null; min: string | null; max: string | null }
  | { kind: "integer-slider"; min: number; max: number; step: 1 }
  | { kind: "fallback"; reason: "missing-evidence" | "source-conflict" | "unsupported-codec" | "unsupported-hints" | "invalid-range" | "module-scope" };

/**
 * `TypeTime` units whose raw value is offered as a plain count: exactly the plain units
 * the local collection declares (18 October 2026: 1,004 of 1,456 declarations). Every
 * other unit, the packed ones included, falls back.
 */
const COUNT_TIME_UNITS: ReadonlySet<string> = new Set([
  "Milliseconds", "TenMilliseconds", "HundredMilliseconds", "Seconds", "Minutes", "Hours",
]);

export function chooseParameterEditor(
  field: ParameterEditorField,
  source: ParameterEditorSource | null,
  context: ParameterEditorContext,
): ParameterEditorPlan {
  if (context.moduleScoped) return { kind: "fallback", reason: "module-scope" };
  if (!source) return { kind: "fallback", reason: "missing-evidence" };
  if (!/^[0-9a-f]{64}$/.test(source.sourceSha256)
      || source.sourceSha256 !== context.sourceSha256 || source.programId !== context.programId
      || source.parameterRefId !== field.etsId || ![20, 21, 22, 23].includes(source.schema)
      || source.child !== `Type${field.kind}` || source.parentAttributes.Id !== source.parameterTypeId
      || (source.attributes.minInclusive ?? null) !== field.min || (source.attributes.maxInclusive ?? null) !== field.max) {
    return { kind: "fallback", reason: "source-conflict" };
  }
  if (field.kind === "Color" || field.kind === "Picture") return { kind: "fallback", reason: "unsupported-codec" };
  const knownTypeAttributes = new Set(["SizeInBit", "Type", "minInclusive", "maxInclusive", "Unit", "UIHint"]);
  const knownParentAttributes = new Set(["Id", "Name", "UIHint"]);
  const namespaceAttribute = (name: string) => name === "xmlns" || name.startsWith("xmlns:");
  if (Object.keys(source.attributes).some(name => !knownTypeAttributes.has(name) && !namespaceAttribute(name))
      || Object.keys(source.parentAttributes).some(name => !knownParentAttributes.has(name) && !namespaceAttribute(name))) {
    return { kind: "fallback", reason: "unsupported-hints" };
  }
  const parentHint = source.parentAttributes.UIHint;
  const hint = source.attributes.UIHint;
  if ((parentHint !== undefined && parentHint !== hint) || source.attributes.Increment !== undefined) {
    return { kind: "fallback", reason: "unsupported-hints" };
  }
  const minimum = decimalInteger(field.min);
  const maximum = decimalInteger(field.max);
  if ((field.min !== null && minimum === null) || (field.max !== null && maximum === null)
      || (minimum !== null && maximum !== null && minimum > maximum)) {
    return { kind: "fallback", reason: "invalid-range" };
  }
  if (field.kind === "Number" && source.attributes.UIHint === "Slider"
      && ["unsignedInt", "signedInt"].includes(source.attributes.Type)
      && minimum !== null && maximum !== null) {
    const current = decimalInteger(field.value);
    if (!Number.isSafeInteger(Number(minimum)) || !Number.isSafeInteger(Number(maximum))
        || current === null || current < minimum || current > maximum
        || (source.attributes.Type === "unsignedInt" && minimum < 0n)) {
      return { kind: "fallback", reason: "invalid-range" };
    }
    return { kind: "integer-slider", min: Number(minimum), max: Number(maximum), step: 1 };
  }
  if (field.kind === "Time" && source.child === "TypeTime") {
    const unit = source.attributes.Unit;
    // A packed unit (`Packed…`) combines several fields in one integer; its layout is
    // not documented, so a decimal count would mislabel the value (census G6).
    if (unit !== undefined && !COUNT_TIME_UNITS.has(unit)) return { kind: "fallback", reason: "unsupported-codec" };
    return { kind: "time-count", unit: unit ?? null, min: field.min, max: field.max };
  }
  return { kind: "fallback", reason: "missing-evidence" };
}

/** Parse only a bounded integer lexeme; original strings are never rewritten. */
function decimalInteger(value: string | null): bigint | null {
  return value !== null && /^[+-]?[0-9]{1,64}$/.test(value) ? BigInt(value) : null;
}

export type ParameterDraftIssue = "integer" | "bounds" | "unsupported";

/** UI preflight only; existing application validation remains authoritative. */
export function parameterDraftIssue(plan: ParameterEditorPlan, value: string): ParameterDraftIssue | null {
  if (plan.kind === "fallback") return "unsupported";
  const parsed = decimalInteger(value);
  if (parsed === null) return "integer";
  const minimum = decimalInteger(plan.min === null ? null : String(plan.min));
  const maximum = decimalInteger(plan.max === null ? null : String(plan.max));
  if ((minimum !== null && parsed < minimum) || (maximum !== null && parsed > maximum)) return "bounds";
  return null;
}
