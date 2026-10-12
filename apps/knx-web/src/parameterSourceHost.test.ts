/** Pins the host-side parse, verification and projection of parameter source evidence. */
import { describe, expect, it } from "vitest";
import {
  isSourceEditorCandidate,
  parameterRevision,
  parameterSelectionKey,
  projectParameterSource,
  type SourceRequest,
} from "./parameterSourceHost";

const sha = "b".repeat(64);
const declared = {
  device: 7,
  parameterRefId: "M-0083_A-0001-01-0000_P-1_R-1",
  serverIncarnation: "srv-1",
  projectIncarnation: 3,
  snapshotRevision: 12,
  evidence: {
    status: "declared",
    source: {
      sourceSha256: sha,
      programId: "M-0083_A-0001-01-0000",
      parameterTypeId: "M-0083_A-0001-01-0000_PT-Delay",
      schema: 21,
      child: "TypeTime",
      attributes: { minInclusive: "0", maxInclusive: "3600", Unit: "Seconds" },
      parentAttributes: { Id: "M-0083_A-0001-01-0000_PT-Delay", Name: "Delay" },
    },
  },
};
const request: SourceRequest = {
  deviceId: 7,
  etsId: "M-0083_A-0001-01-0000_P-1_R-1",
  programId: "M-0083_A-0001-01-0000",
  snapshot: { serverIncarnation: "srv-1", projectIncarnation: 3, snapshotRevision: 12 },
};

describe("parameter source projection", () => {
  it("projects declared evidence with the response's reference and separate parent attributes", () => {
    expect(projectParameterSource(declared, request)).toEqual({
      kind: "declared",
      source: {
        sourceSha256: sha,
        programId: "M-0083_A-0001-01-0000",
        parameterRefId: "M-0083_A-0001-01-0000_P-1_R-1",
        parameterTypeId: "M-0083_A-0001-01-0000_PT-Delay",
        schema: 21,
        child: "TypeTime",
        attributes: { minInclusive: "0", maxInclusive: "3600", Unit: "Seconds" },
        parentAttributes: { Id: "M-0083_A-0001-01-0000_PT-Delay", Name: "Delay" },
      },
    });
  });

  it("does not let parent attributes leak into child attributes or vice versa", () => {
    const projected = projectParameterSource(declared, request);
    if (projected.kind !== "declared") throw new Error("expected declared");
    expect(projected.source.attributes).not.toHaveProperty("Id");
    expect(projected.source.parentAttributes).not.toHaveProperty("Unit");
  });

  it("keeps every unavailable reason, known or not, as unavailable", () => {
    for (const reason of ["missingSource", "modelMismatch", "somethingNewFromTomorrow"]) {
      expect(projectParameterSource({ ...declared, evidence: { status: "unavailable", reason } }, request))
        .toEqual({ kind: "unavailable", reason });
    }
  });

  it("refuses a response for another device, reference, server or project", () => {
    for (const changed of [
      { device: 8 },
      { parameterRefId: "M-0083_A-0001-01-0000_P-2_R-1" },
      { serverIncarnation: "srv-2" },
      { projectIncarnation: 4 },
    ]) {
      expect(projectParameterSource({ ...declared, ...changed }, request)).toEqual({ kind: "mismatch" });
    }
  });

  it("refuses a response captured at another snapshot than the request", () => {
    expect(projectParameterSource({ ...declared, snapshotRevision: 13 }, request)).toEqual({ kind: "mismatch" });
  });

  it("refuses source evidence for another program than the panel's", () => {
    expect(projectParameterSource(declared, { ...request, programId: "M-0083_A-0002-01-0000" }))
      .toEqual({ kind: "mismatch" });
  });

  it("treats malformed bodies as malformed, never as evidence", () => {
    const bad: unknown[] = [
      null,
      "declared",
      { ...declared, evidence: { status: "declared" } },
      { ...declared, evidence: { status: "maybe", source: declared.evidence.source } },
      { ...declared, evidence: { status: "unavailable" } },
      { ...declared, device: "7" },
      { ...declared, projectIncarnation: 1.5 },
      { ...declared, evidence: { ...declared.evidence, source: { ...declared.evidence.source, schema: "21" } } },
      { ...declared, evidence: { ...declared.evidence, source: { ...declared.evidence.source, attributes: { Unit: 5 } } } },
      { ...declared, evidence: { ...declared.evidence, source: { ...declared.evidence.source, parentAttributes: [] } } },
    ];
    for (const body of bad) expect(projectParameterSource(body, request)).toEqual({ kind: "malformed" });
  });

  it("builds revision and selection keys from authoritative identity only", () => {
    expect(parameterRevision(request.snapshot)).toBe("srv-1/3/12");
    expect(parameterSelectionKey(request.snapshot, 7, null)).toBe("srv-1/3/device:7/scope:null");
    expect(parameterSelectionKey(request.snapshot, 7, { moduleId: "MD-1", instance: 2 }))
      .toBe('srv-1/3/device:7/scope:{"moduleId":"MD-1","instance":2}');
  });

  it("offers the source-checked editor only for kinds the leaf decides on", () => {
    expect(["Time", "Number", "Color", "Picture"].every(isSourceEditorCandidate)).toBe(true);
    expect(["Restriction", "Text", "None", "Float", "IPAddress", "Raw"].some(isSourceEditorCandidate)).toBe(false);
  });
});
